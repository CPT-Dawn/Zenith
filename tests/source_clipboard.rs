//! The clipboard source against a fake Hoarder Daemon: a Unix socket speaking the
//! documented JSON lines (Hoarder's docs/ipc.md), with the source on a glib main loop.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io::{IoSliceMut, Write};
use std::mem::MaybeUninit;
use std::os::fd::OwnedFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use gtk4::glib;
use pretty_assertions::assert_eq;
use rustix::net::{RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, recvmsg};
use serde_json::{Value, json};
use zenith_bar::modules::clipboard::source::ClipboardSource;
use zenith_bar::modules::clipboard::{ClipboardConfig, Kind, Recent, Row};

// ---------------------------------------------------------------------------
// The fake Daemon
// ---------------------------------------------------------------------------

/// What the fake Daemon holds and what it was asked.
#[derive(Default)]
struct Book {
    state: String,
    clips: Vec<Value>,
    thumbs: HashMap<i64, Vec<u8>>,
    requests: Vec<Value>,
    /// The live connection, for pushing events.
    client: Option<UnixStream>,
}

struct FakeDaemon {
    path: PathBuf,
    book: Arc<Mutex<Book>>,
    stop: Arc<AtomicBool>,
    accept: Option<JoinHandle<()>>,
}

fn clip(id: i64, kind: &str, preview: &str) -> Value {
    json!({
        "id": id,
        "kind": kind,
        "preview": preview,
        "app": null,
        "at": "2026-11-08T12:00:00Z",
        "created": "2026-11-08T12:00:00Z",
        "pinned": false,
        "size": preview.len(),
        "types": if kind == "image" { json!(["image/png"]) } else { json!(["text/plain"]) },
    })
}

fn send(stream: &mut UnixStream, line: Value) {
    let mut text = line.to_string();
    text.push('\n');
    // The client may be gone already; that is the source's business.
    let _ = stream.write_all(text.as_bytes());
}

fn event(name: &str, fields: Value) -> Value {
    let mut line = json!({"v": 1, "event": name});
    line.as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    line
}

impl FakeDaemon {
    fn start(path: &Path, clips: Vec<Value>) -> Self {
        let book = Arc::new(Mutex::new(Book {
            state: "active".into(),
            clips,
            ..Book::default()
        }));
        Self::serve(path, book)
    }

    fn serve(path: &Path, book: Arc<Mutex<Book>>) -> Self {
        let listener = UnixListener::bind(path).expect("bind the fake Daemon");
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let accept = std::thread::spawn({
            let book = book.clone();
            let stop = stop.clone();
            move || {
                while !stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            stream.set_nonblocking(false).unwrap();
                            let book = book.clone();
                            std::thread::spawn(move || converse(stream, book));
                        }
                        Err(_) => std::thread::sleep(Duration::from_millis(5)),
                    }
                }
            }
        });
        Self {
            path: path.to_owned(),
            book,
            stop,
            accept: Some(accept),
        }
    }

    /// Push an event to the connected Front end.
    fn push(&self, line: Value) {
        let mut book = self.book.lock().unwrap();
        let client = book.client.as_mut().expect("a connected Front end");
        send(client, line);
    }

    fn requests(&self) -> Vec<Value> {
        self.book.lock().unwrap().requests.clone()
    }

    fn ops(&self) -> Vec<String> {
        self.requests()
            .iter()
            .map(|r| r["op"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    fn connected(&self) -> bool {
        self.book.lock().unwrap().client.is_some()
    }

    /// Like the Daemon quitting: hang up and take the socket away.
    fn kill(mut self) -> Arc<Mutex<Book>> {
        self.shutdown();
        self.book.clone()
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(accept) = self.accept.take() {
            accept.join().unwrap();
        }
        if let Some(client) = self.book.lock().unwrap().client.take() {
            let _ = client.shutdown(std::net::Shutdown::Both);
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

impl Drop for FakeDaemon {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Serve one connection: read request lines (and any fds sent with them), answer them.
fn converse(stream: UnixStream, book: Arc<Mutex<Book>>) {
    let mut out = stream.try_clone().unwrap();
    book.lock().unwrap().client = Some(stream.try_clone().unwrap());
    let mut fds: VecDeque<OwnedFd> = VecDeque::new();
    let mut pending = Vec::new();
    loop {
        let mut buf = [0u8; 4096];
        let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(4))];
        let mut ancillary = RecvAncillaryBuffer::new(&mut space);
        let Ok(got) = recvmsg(
            &stream,
            &mut [IoSliceMut::new(&mut buf)],
            &mut ancillary,
            RecvFlags::CMSG_CLOEXEC,
        ) else {
            break;
        };
        for message in ancillary.drain() {
            if let RecvAncillaryMessage::ScmRights(received) = message {
                fds.extend(received);
            }
        }
        if got.bytes == 0 {
            break;
        }
        pending.extend_from_slice(&buf[..got.bytes]);
        while let Some(end) = pending.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = pending.drain(..=end).collect();
            let request: Value = serde_json::from_slice(&line).expect("a JSON request line");
            answer(&request, &mut out, &mut fds, &book);
        }
    }
    let mut book = book.lock().unwrap();
    book.client = None;
}

fn answer(request: &Value, out: &mut UnixStream, fds: &mut VecDeque<OwnedFd>, book: &Mutex<Book>) {
    let mut book = book.lock().unwrap();
    book.requests.push(request.clone());
    assert_eq!(request["v"], 1, "every request names protocol v1");
    let id = request["id"].as_u64().expect("every request has an id");
    let ok = |out: &mut UnixStream, value: Value| send(out, json!({"v": 1, "id": id, "ok": value}));
    let recording = |book: &Book| json!({"state": book.state});
    match request["op"].as_str().unwrap() {
        "subscribe" => {
            ok(out, Value::Null);
            send(
                out,
                event(
                    "selection",
                    json!({"selection": "clipboard", "types": null}),
                ),
            );
            send(out, event("state", recording(&book)));
        }
        "status" => {
            let mut status = json!({
                "daemon": "0.1.0", "pid": 4242, "data_control": "ext", "persist": true,
                "clipboard": {"owner": "app", "types": [], "kept": [], "reading": false},
                "primary": null,
            });
            status["state"] = json!(book.state);
            ok(out, status);
        }
        "list" => {
            let limit = request["limit"].as_u64().unwrap_or(u64::MAX) as usize;
            ok(
                out,
                json!(book.clips.iter().take(limit).collect::<Vec<_>>()),
            );
        }
        "restore" => {
            let clip = request["clip"].as_i64().unwrap();
            let at = book.clips.iter().position(|c| c["id"] == clip).unwrap();
            let restored = book.clips.remove(at);
            book.clips.insert(0, restored.clone());
            send(out, event("clip_added", json!({"clip": restored})));
            ok(out, restored);
        }
        op @ ("pause" | "resume") => {
            book.state = if op == "pause" { "paused" } else { "active" }.into();
            send(out, event("state", recording(&book)));
            ok(out, recording(&book));
        }
        "thumb" => {
            let fd = fds.pop_front().expect("thumb comes with a pipe");
            let clip = request["clip"].as_i64().unwrap();
            match book.thumbs.get(&clip) {
                Some(png) => {
                    let summary = book.clips.iter().find(|c| c["id"] == clip).unwrap();
                    ok(
                        out,
                        json!({"clip": summary, "mime": "image/png", "size": png.len()}),
                    );
                    std::fs::File::from(fd).write_all(png).unwrap();
                }
                None => send(
                    out,
                    json!({"v": 1, "id": id, "err": {"code": "not_found", "message": "no thumbnail"}}),
                ),
            }
        }
        _ => send(
            out,
            json!({"v": 1, "id": id, "err": {"code": "unknown_op", "message": "no such op"}}),
        ),
    }
}

// ---------------------------------------------------------------------------
// The main loop
// ---------------------------------------------------------------------------

/// Tests take turns owning the default main context, which the source runs on.
static MAIN_CONTEXT: Mutex<()> = Mutex::new(());

fn on_main_loop(test: impl FnOnce(&glib::MainContext)) {
    let _turn = MAIN_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    let context = glib::MainContext::default();
    let _owner = context.acquire().expect("own the default main context");
    test(&context);
    // Let anything the test left behind (cancelled I/O, timers) run out.
    while context.iteration(false) {}
}

/// Run the main loop until `done`, or fail after a few seconds.
fn wait_until(context: &glib::MainContext, what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting until {what}");
        if !context.iteration(false) {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

/// Run the main loop for a while, for things that should not happen.
fn run_for(context: &glib::MainContext, time: Duration) {
    let deadline = Instant::now() + time;
    while Instant::now() < deadline {
        if !context.iteration(false) {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

/// A fresh socket path. Kept short: Unix socket paths stop at 108 bytes.
fn socket_path() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("zb-clip-{}-{n}.sock", std::process::id()));
    let _ = std::fs::remove_file(&path);
    path
}

fn start(path: &Path) -> Rc<ClipboardSource> {
    ClipboardSource::start_at(Some(path.to_owned()), ClipboardConfig::default())
}

fn classes(source: &ClipboardSource) -> Vec<&'static str> {
    source.state().with(|state| state.classes.clone())
}

fn recent(source: &ClipboardSource) -> Recent {
    source.recent().with(Clone::clone)
}

fn row(id: i64, kind: Kind, label: &str) -> Row {
    Row {
        id,
        kind,
        label: label.into(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn with_no_daemon_the_module_is_offline_and_nothing_fails() {
    on_main_loop(|context| {
        let path = socket_path();

        let source = start(&path);
        run_for(context, Duration::from_millis(100));

        assert_eq!(classes(&source), vec!["zb-clipboard-offline"]);
        assert_eq!(recent(&source), Recent::default());
    });
}

#[test]
fn it_subscribes_and_lists_the_ten_most_recent_clips() {
    on_main_loop(|context| {
        let path = socket_path();
        let clips = (1..=12)
            .rev()
            .map(|id| clip(id, "text", &format!("clip {id}")));
        let daemon = FakeDaemon::start(&path, clips.collect());

        let source = start(&path);
        wait_until(context, "listed", || recent(&source).rows.len() == 10);

        assert_eq!(classes(&source), vec!["zb-clipboard-active"]);
        let ids: Vec<i64> = recent(&source).rows.iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![12, 11, 10, 9, 8, 7, 6, 5, 4, 3]);
        assert!(daemon.ops().contains(&"subscribe".to_string()));
        let list = daemon
            .requests()
            .into_iter()
            .find(|r| r["op"] == "list")
            .expect("a list request");
        assert_eq!(list["limit"], 10);
    });
}

#[test]
fn pushed_events_keep_the_bar_and_the_list_current() {
    on_main_loop(|context| {
        let path = socket_path();
        let daemon = FakeDaemon::start(&path, vec![clip(1, "text", "first")]);
        let source = start(&path);
        wait_until(context, "listed", || recent(&source).rows.len() == 1);

        daemon.push(event(
            "clip_added",
            json!({"clip": clip(2, "image", "640×480")}),
        ));
        daemon.push(event("state", json!({"state": "paused"})));
        daemon.push(event("from_the_future", json!({"x": 1})));
        wait_until(context, "paused", || {
            classes(&source) == vec!["zb-clipboard-paused"]
        });

        assert_eq!(
            recent(&source).rows,
            vec![row(2, Kind::Image, "640×480"), row(1, Kind::Text, "first")]
        );
        daemon.push(event("clip_removed", json!({"clip": 2})));
        wait_until(context, "clip 2 gone", || recent(&source).rows.len() == 1);
    });
}

#[test]
fn restoring_a_clip_asks_the_daemon_and_moves_it_to_the_top() {
    on_main_loop(|context| {
        let path = socket_path();
        let daemon = FakeDaemon::start(
            &path,
            vec![clip(2, "text", "newer"), clip(1, "text", "older")],
        );
        let source = start(&path);
        wait_until(context, "listed", || recent(&source).rows.len() == 2);

        source.restore(1);
        wait_until(context, "restored", || recent(&source).rows[0].id == 1);

        let restore = daemon
            .requests()
            .into_iter()
            .find(|r| r["op"] == "restore")
            .expect("a restore request");
        assert_eq!(restore["clip"], 1);
    });
}

#[test]
fn toggling_pauses_then_resumes_recording() {
    on_main_loop(|context| {
        let path = socket_path();
        let daemon = FakeDaemon::start(&path, Vec::new());
        let source = start(&path);
        wait_until(context, "online", || recent(&source).online);

        source.toggle_pause();
        wait_until(context, "paused", || {
            classes(&source) == vec!["zb-clipboard-paused"]
        });
        source.toggle_pause();
        wait_until(context, "active", || {
            classes(&source) == vec!["zb-clipboard-active"]
        });

        let pause = daemon
            .requests()
            .into_iter()
            .find(|r| r["op"] == "pause")
            .expect("a pause request");
        assert_eq!(pause.get("secs"), None, "pauses until resumed");
        assert!(daemon.ops().contains(&"resume".to_string()));
    });
}

#[test]
fn a_thumbnail_comes_back_through_the_pipe() {
    on_main_loop(|context| {
        let path = socket_path();
        let daemon = FakeDaemon::start(
            &path,
            vec![clip(2, "image", "64×64"), clip(1, "text", "words")],
        );
        let png = b"\x89PNG\r\n\x1a\n not really, but bytes are bytes".to_vec();
        daemon.book.lock().unwrap().thumbs.insert(2, png.clone());
        let source = start(&path);
        wait_until(context, "listed", || recent(&source).rows.len() == 2);

        let image: Rc<RefCell<Option<Option<Vec<u8>>>>> = Rc::default();
        let text: Rc<RefCell<Option<Option<Vec<u8>>>>> = Rc::default();
        for (clip, slot) in [(2, &image), (1, &text)] {
            let slot = slot.clone();
            source.thumb(clip, move |bytes| {
                *slot.borrow_mut() = Some(bytes.map(|b| b.to_vec()));
            });
        }
        wait_until(context, "both answered", || {
            image.borrow().is_some() && text.borrow().is_some()
        });

        assert_eq!(image.take(), Some(Some(png)));
        assert_eq!(
            text.take(),
            Some(None),
            "a Clip without a thumbnail has none"
        );
    });
}

#[test]
fn when_the_daemon_goes_away_it_goes_offline_and_comes_back_with_it() {
    on_main_loop(|context| {
        let path = socket_path();
        let daemon = FakeDaemon::start(&path, vec![clip(1, "text", "kept")]);
        let source = start(&path);
        wait_until(context, "listed", || recent(&source).rows.len() == 1);

        let book = daemon.kill();
        wait_until(context, "offline", || {
            classes(&source) == vec!["zb-clipboard-offline"]
        });
        assert_eq!(recent(&source), Recent::default());

        let daemon = FakeDaemon::serve(&path, book);
        wait_until(context, "back online", || recent(&source).rows.len() == 1);
        assert_eq!(recent(&source).rows, vec![row(1, Kind::Text, "kept")]);
        assert!(daemon.connected());
    });
}

#[test]
fn a_daemon_started_after_the_bar_is_found() {
    on_main_loop(|context| {
        let path = socket_path();
        let source = start(&path);
        run_for(context, Duration::from_millis(50));

        let _daemon = FakeDaemon::start(&path, vec![clip(1, "text", "late")]);

        wait_until(context, "listed", || recent(&source).rows.len() == 1);
    });
}

#[test]
fn dropping_the_source_hangs_up() {
    on_main_loop(|context| {
        let path = socket_path();
        let daemon = FakeDaemon::start(&path, Vec::new());
        let source = start(&path);
        wait_until(context, "online", || recent(&source).online);

        drop(source);

        wait_until(context, "hung up", || !daemon.connected());
    });
}
