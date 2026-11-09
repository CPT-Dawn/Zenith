//! The one clipboard source per process: a connection to the Hoarder Daemon's socket,
//! all async on the main loop.
//!
//! `hoarder-ipc` supplies the protocol's types and framing; its client blocks, so the
//! socket is driven here with gio (ADR-0002). On connecting, the source subscribes,
//! asks `status`, then `list`s the most recent Clips; after that it only reacts to
//! pushed events. Asking `list` once `status` has answered keeps the two in order:
//! every Clip event before the `list` answer is already in it. With no Daemon, or
//! when it goes away, the module shows Offline and tries again with a [`Backoff`].

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::path::PathBuf;
use std::rc::Rc;

use gio_unix::prelude::*;
use gtk4::{gio, glib};
use hoarder_ipc::{
    ClipSummary, Event, EventLine, Op, Outcome, PROTOCOL, Recording, Request, Response,
};
use jiff::tz::TimeZone;
use serde_json::Value;

use super::ClipboardConfig;
use super::model::{Backoff, Clipboard, RECENT, Recent};
use crate::modules::{Shared, ViewState, if_alive};

pub struct ClipboardSource {
    /// The Daemon's socket; `None` without `$XDG_RUNTIME_DIR`, which leaves it Offline.
    path: Option<PathBuf>,
    config: RefCell<ClipboardConfig>,
    model: RefCell<Clipboard>,
    state: Rc<Shared<ViewState>>,
    recent: Rc<Shared<Recent>>,
    link: RefCell<Option<Link>>,
    backoff: RefCell<Backoff>,
    retry: RefCell<Option<glib::SourceId>>,
}

/// One live connection.
struct Link {
    connection: gio::SocketConnection,
    /// Cancels this connection's reads (and thumbnail pipes) when it is dropped.
    cancellable: gio::Cancellable,
    next_id: u64,
    pending: HashMap<u64, Pending>,
}

/// What to do with the answer to a request.
enum Pending {
    Status,
    List,
    /// Only a failure matters, and only to the log.
    Logged(&'static str),
    Thumb(Rc<Thumb>),
}

impl ClipboardSource {
    pub fn start(config: ClipboardConfig) -> Rc<Self> {
        Self::start_at(hoarder_ipc::socket_path(), config)
    }

    /// Talk to the Daemon at `path` instead of the usual socket.
    pub fn start_at(path: Option<PathBuf>, config: ClipboardConfig) -> Rc<Self> {
        let model = Clipboard::new();
        let source = Rc::new(Self {
            path,
            config: RefCell::new(config),
            state: Shared::new(model.view_state(&TimeZone::system())),
            recent: Shared::new(model.recent()),
            model: RefCell::new(model),
            link: RefCell::new(None),
            backoff: RefCell::default(),
            retry: RefCell::new(None),
        });
        source.connect();
        source
    }

    /// The bar's state: active, paused or offline.
    pub fn state(&self) -> Rc<Shared<ViewState>> {
        self.state.clone()
    }

    /// The recent Clips, for popovers.
    pub fn recent(&self) -> Rc<Shared<Recent>> {
        self.recent.clone()
    }

    pub fn reconfigure(&self, config: ClipboardConfig) {
        *self.config.borrow_mut() = config;
    }

    /// The command behind "Search all…".
    pub fn search_command(&self) -> String {
        self.config.borrow().search_command.clone()
    }

    /// Make Clip `id` the clipboard again. The Daemon's `clip_added` moves it up.
    pub fn restore(self: &Rc<Self>, id: i64) {
        let op = Op::Restore {
            clip: id,
            plain: false,
        };
        self.request(op, Pending::Logged("restore"), None);
    }

    /// Pause recording until resumed, or resume it. Nothing while Offline.
    pub fn toggle_pause(self: &Rc<Self>) {
        let op = self.model.borrow().toggle_pause();
        if let Some(op) = op {
            self.request(op, Pending::Logged("pause"), None);
        }
    }

    /// Fetch Clip `id`'s thumbnail (a PNG), or `None` if it has none or the Daemon is gone.
    pub fn thumb(self: &Rc<Self>, id: i64, done: impl FnOnce(Option<glib::Bytes>) + 'static) {
        let thumb = Rc::new(Thumb {
            done: RefCell::new(Some(Box::new(done))),
            ..Thumb::default()
        });
        let Some(cancellable) = self.link.borrow().as_ref().map(|l| l.cancellable.clone()) else {
            thumb.answered(false);
            thumb.read(None);
            return;
        };
        let (reader, writer) = match std::io::pipe() {
            Ok(pipe) => pipe,
            Err(e) => {
                log::warn!("clipboard: no pipe for a thumbnail: {e}");
                thumb.answered(false);
                thumb.read(None);
                return;
            }
        };
        self.request(
            Op::Thumb { clip: id },
            Pending::Thumb(thumb.clone()),
            Some(writer.as_fd()),
        );
        // The Daemon has its own copy now; ours must go, or the pipe never ends.
        drop(writer);

        // Read while the Daemon writes: the PNG may not fit in the pipe's buffer.
        let input = gio_unix::InputStream::take_fd(OwnedFd::from(reader));
        let output = gio::MemoryOutputStream::new_resizable();
        output.clone().splice_async(
            &input,
            gio::OutputStreamSpliceFlags::CLOSE_SOURCE | gio::OutputStreamSpliceFlags::CLOSE_TARGET,
            glib::Priority::DEFAULT,
            Some(&cancellable),
            move |result| thumb.read(result.ok().map(|_| output.steal_as_bytes())),
        );
    }

    fn connect(self: &Rc<Self>) {
        let Some(path) = self.path.clone() else {
            log::debug!("clipboard: XDG_RUNTIME_DIR is not set, so there is no Hoarder to find");
            return;
        };
        let address = gio::UnixSocketAddress::new(&path);
        let weak = Rc::downgrade(self);
        gio::SocketClient::new().connect_async(&address, gio::Cancellable::NONE, move |result| {
            match result {
                Ok(connection) => match weak.upgrade() {
                    Some(source) => source.greet(connection),
                    None => {
                        let _ = connection.socket().close();
                    }
                },
                Err(e) => if_alive(&weak, |source| {
                    log::debug!("clipboard: no Hoarder at {}: {e}", path.display());
                    source.retry_later();
                }),
            }
        });
    }

    /// Connected: subscribe, ask the state, and start reading.
    fn greet(self: &Rc<Self>, connection: gio::SocketConnection) {
        let cancellable = gio::Cancellable::new();
        *self.link.borrow_mut() = Some(Link {
            connection: connection.clone(),
            cancellable: cancellable.clone(),
            next_id: 1,
            pending: HashMap::new(),
        });
        let input = gio::DataInputStream::new(&connection.input_stream());
        input.set_newline_type(gio::DataStreamNewlineType::Lf);
        self.read(input, cancellable);
        self.request(Op::Subscribe, Pending::Logged("subscribe"), None);
        self.request(Op::Status, Pending::Status, None);
    }

    fn read(self: &Rc<Self>, input: gio::DataInputStream, cancellable: gio::Cancellable) {
        let weak = Rc::downgrade(self);
        input.clone().read_line_utf8_async(
            glib::Priority::DEFAULT,
            Some(&cancellable.clone()),
            move |result| {
                if cancellable.is_cancelled() {
                    return;
                }
                let Some(source) = weak.upgrade() else { return };
                match result {
                    Ok(Some(line)) => {
                        source.receive(&line);
                        if !cancellable.is_cancelled() {
                            source.read(input, cancellable);
                        }
                    }
                    Ok(None) => source.lost("Hoarder hung up"),
                    Err(e) => source.lost(&e.to_string()),
                }
            },
        );
    }

    /// One line from the Daemon: an event, or the answer to a request.
    fn receive(self: &Rc<Self>, line: &str) {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            log::info!("clipboard: ignoring a line that is not JSON");
            return;
        };
        if value.get("event").is_some() {
            if let Ok(EventLine { event, .. }) = serde_json::from_value(value) {
                self.apply(event);
            }
            return;
        }
        let Ok(response) = serde_json::from_value::<Response>(value) else {
            return;
        };
        let Some(id) = response.id else { return };
        let pending = self
            .link
            .borrow_mut()
            .as_mut()
            .and_then(|link| link.pending.remove(&id));
        match (pending, response.outcome) {
            (Some(Pending::Status), Outcome::Ok(status)) => {
                // `status` carries the Recording's fields at its top level.
                let recording = serde_json::from_value::<Recording>(status).unwrap_or_default();
                self.backoff.borrow_mut().reset();
                self.model.borrow_mut().connected(recording, Vec::new());
                self.publish();
                let list = Op::List {
                    limit: Some(RECENT),
                    concealed: false,
                };
                self.request(list, Pending::List, None);
            }
            (Some(Pending::List), Outcome::Ok(clips)) => {
                let clips = serde_json::from_value::<Vec<ClipSummary>>(clips).unwrap_or_default();
                self.model.borrow_mut().listed(clips);
                self.publish();
            }
            (Some(Pending::Thumb(thumb)), outcome) => {
                thumb.answered(matches!(outcome, Outcome::Ok(_)));
            }
            (Some(Pending::Status | Pending::List), Outcome::Err(failure)) => {
                self.lost(&format!("Hoarder would not say its state: {failure}"));
            }
            (Some(Pending::Logged(op)), Outcome::Err(failure)) => {
                log::info!("clipboard: {op} failed: {failure}");
            }
            _ => {}
        }
    }

    fn apply(self: &Rc<Self>, event: Event) {
        self.model.borrow_mut().apply(event);
        self.publish();
    }

    /// Send one request, with `fd` attached if given. A failed send drops the link.
    fn request(self: &Rc<Self>, op: Op, pending: Pending, fd: Option<BorrowedFd<'_>>) {
        let sent = {
            let mut link = self.link.borrow_mut();
            let Some(link) = link.as_mut() else {
                if let Pending::Thumb(thumb) = pending {
                    thumb.answered(false);
                }
                return;
            };
            let id = link.next_id;
            link.next_id += 1;
            let line = hoarder_ipc::encode(&Request {
                v: PROTOCOL,
                id,
                op,
            });
            link.pending.insert(id, pending);
            send(&link.connection.socket(), line.as_bytes(), fd)
                .map_err(|e| (e, link.cancellable.clone()))
        };
        if let Err((e, failed_link)) = sent {
            // Not now: the caller may be a subscriber mid-publish, or a view
            // holding a borrow of the list. Dropping the link re-publishes.
            let weak = Rc::downgrade(self);
            let why = e.to_string();
            glib::idle_add_local_once(move || {
                let Some(source) = weak.upgrade() else { return };
                // Only if that link is still the current one: a reconnect may have
                // won the race to the main loop, and its new link is healthy.
                let current = source.link.borrow().as_ref().map(|l| l.cancellable.clone());
                if current.as_ref() == Some(&failed_link) {
                    source.lost(&why);
                }
            });
        }
    }

    /// The connection is gone: show Offline and try again later.
    fn lost(self: &Rc<Self>, why: &str) {
        let Some(link) = self.link.take() else { return };
        log::info!("clipboard: lost Hoarder ({why}); will try again");
        link.close();
        self.model.borrow_mut().disconnected();
        self.publish();
        self.retry_later();
    }

    fn retry_later(self: &Rc<Self>) {
        let delay = self.backoff.borrow_mut().after_failure();
        let weak = Rc::downgrade(self);
        let retry = glib::timeout_add_local_once(delay, move || {
            if let Some(source) = weak.upgrade() {
                // This timer has fired; forget it rather than remove it.
                source.retry.take();
                source.connect();
            }
        });
        if let Some(old) = self.retry.replace(Some(retry)) {
            old.remove();
        }
    }

    fn publish(&self) {
        // Subscribers run inside `set`; the model must not stay borrowed then.
        let (state, recent) = {
            let model = self.model.borrow();
            (model.view_state(&TimeZone::system()), model.recent())
        };
        self.state.set(state);
        self.recent.set(recent);
    }
}

impl Drop for ClipboardSource {
    fn drop(&mut self) {
        if let Some(link) = self.link.take() {
            link.close();
        }
        if let Some(retry) = self.retry.take() {
            retry.remove();
        }
    }
}

impl Link {
    /// Hang up now; every request still waiting gets no answer.
    fn close(self) {
        self.cancellable.cancel();
        let _ = self.connection.socket().close();
        for (_, pending) in self.pending {
            if let Pending::Thumb(thumb) = pending {
                thumb.answered(false);
            }
        }
    }
}

/// Write one request line on the socket, with `fd` riding on its first byte
/// (`SCM_RIGHTS`). Lines are small, so this does not wait on a live Daemon.
fn send(socket: &gio::Socket, line: &[u8], fd: Option<BorrowedFd<'_>>) -> Result<(), glib::Error> {
    let mut messages = Vec::new();
    if let Some(fd) = fd {
        let message = gio_unix::FDMessage::new();
        message.append_fd(fd)?;
        messages.push(message.upcast::<gio::SocketControlMessage>());
    }
    let mut sent = socket.send_message(
        None::<&gio::SocketAddress>,
        &[gio::OutputVector::new(line)],
        &messages,
        0,
        gio::Cancellable::NONE,
    )?;
    while sent < line.len() {
        sent += socket.send(&line[sent..], gio::Cancellable::NONE)?;
    }
    Ok(())
}

type ThumbDone = Box<dyn FnOnce(Option<glib::Bytes>)>;

/// A thumbnail on its way: the Daemon's answer and the pipe's contents, in either order.
#[derive(Default)]
struct Thumb {
    ok: Cell<Option<bool>>,
    data: RefCell<Option<Option<glib::Bytes>>>,
    done: RefCell<Option<ThumbDone>>,
}

impl Thumb {
    fn answered(&self, ok: bool) {
        self.ok.set(Some(ok));
        self.finish();
    }

    fn read(&self, data: Option<glib::Bytes>) {
        *self.data.borrow_mut() = Some(data);
        self.finish();
    }

    fn finish(&self) {
        let (Some(ok), Some(data)) = (self.ok.get(), self.data.borrow().clone()) else {
            return;
        };
        let Some(done) = self.done.take() else { return };
        done(data.filter(|bytes| ok && !bytes.is_empty()));
    }
}
