//! The one todo source per process: loads the file once, applies commands from any
//! bar's popover, publishes the list and the bar state, and saves after each change.
//! All file I/O is async on the main loop; saves never overlap.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::gio::prelude::*;
use gtk4::{gio, glib};

use super::model::{self, Command, TodoList};
use super::store;
use crate::modules::{Shared, ViewState};

pub struct TodoSource {
    path: PathBuf,
    state: Rc<Shared<ViewState>>,
    list: Rc<Shared<TodoList>>,
    /// Commands that arrive before the file has been read; applied once it has.
    pending: RefCell<Option<Vec<Command>>>,
    saving: Cell<bool>,
    dirty: Cell<bool>,
}

impl TodoSource {
    pub fn start(path: PathBuf) -> Rc<Self> {
        let list = TodoList::default();
        let source = Rc::new(Self {
            path,
            state: Shared::new(model::view_state(&list)),
            list: Shared::new(list),
            pending: RefCell::new(Some(Vec::new())),
            saving: Cell::new(false),
            dirty: Cell::new(false),
        });
        let weak = Rc::downgrade(&source);
        glib::spawn_future_local(async move {
            let Some(path) = weak.upgrade().map(|s| s.path.clone()) else {
                return;
            };
            let read = store::read(&path).await;
            let Some(source) = weak.upgrade() else { return };
            source.loaded(read).await;
        });
        source
    }

    /// The bar's state: open count, urgency, tooltip.
    pub fn state(&self) -> Rc<Shared<ViewState>> {
        self.state.clone()
    }

    /// The list itself, for popovers.
    pub fn list(&self) -> Rc<Shared<TodoList>> {
        self.list.clone()
    }

    pub fn apply(self: &Rc<Self>, command: Command) {
        if let Some(pending) = self.pending.borrow_mut().as_mut() {
            pending.push(command);
            return;
        }
        let mut list = self.list.with(Clone::clone);
        if list.apply(command).is_some() {
            self.publish(list);
            self.save();
        }
    }

    async fn loaded(self: &Rc<Self>, read: Result<Option<String>, glib::Error>) {
        let path = self.path.display();
        let loaded = match read {
            Ok(contents) => {
                let corrupt = contents.is_some();
                let loaded = model::load(contents.as_deref());
                if let Some(warning) = &loaded.warning {
                    log::warn!("todo: {path}: {warning}");
                    if corrupt {
                        self.set_aside().await;
                    }
                }
                loaded.list
            }
            Err(e) => {
                log::warn!("todo: cannot read {path}: {e}; starting with an empty list");
                TodoList::default()
            }
        };
        let pending = self.pending.take().unwrap_or_default();
        let mut list = loaded;
        let mut changed = false;
        for command in pending {
            changed |= list.apply(command).is_some();
        }
        self.publish(list);
        if changed {
            self.save();
        }
    }

    /// Keep a corrupt file as `<name>.corrupt` so the next save cannot destroy it.
    async fn set_aside(&self) {
        let mut name = self.path.as_os_str().to_owned();
        name.push(".corrupt");
        let (moved, _progress) = gio::File::for_path(&self.path).move_future(
            &gio::File::for_path(PathBuf::from(&name)),
            gio::FileCopyFlags::OVERWRITE,
            glib::Priority::DEFAULT,
        );
        match moved.await {
            Ok(()) => log::warn!("todo: kept the corrupt file as {}", name.display()),
            Err(e) => log::warn!("todo: cannot set the corrupt file aside: {e}"),
        }
    }

    fn publish(&self, list: TodoList) {
        self.state.set(model::view_state(&list));
        self.list.set(list);
    }

    /// Save the current list. A change during a save triggers one more save afterwards.
    fn save(self: &Rc<Self>) {
        if self.saving.replace(true) {
            self.dirty.set(true);
            return;
        }
        let source = self.clone();
        glib::spawn_future_local(async move {
            loop {
                source.dirty.set(false);
                let json = source.list.with(TodoList::to_json);
                if let Err(e) = store::save(&source.path, json).await {
                    log::warn!("todo: cannot save {}: {e}", source.path.display());
                }
                if !source.dirty.get() {
                    break;
                }
            }
            source.saving.set(false);
        });
    }
}
