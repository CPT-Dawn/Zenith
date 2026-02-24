use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, CheckButton, Entry, Label, Orientation, Popover, ScrolledWindow,
    Separator,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

// ─── Data Model ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub text: String,
    pub done: bool,
    pub priority: u8, // 1 (highest) – 9 (lowest), 0 = none
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TodoStore {
    pub items: Vec<TodoItem>,
}

impl TodoStore {
    /// Path: ~/.config/zenith/todos.json
    fn storage_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("zenith").join("todos.json"))
    }

    pub fn load() -> Self {
        Self::storage_path()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::storage_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = fs::write(path, json);
            }
        }
    }

    /// Number of incomplete items.
    pub fn pending_count(&self) -> usize {
        self.items.iter().filter(|t| !t.done).count()
    }

    /// First incomplete item text (the one shown on the bar).
    pub fn top_task(&self) -> Option<&str> {
        self.items
            .iter()
            .find(|t| !t.done)
            .map(|t| t.text.as_str())
    }
}

// ─── Widget Construction ─────────────────────────────────────────────────────

/// Type alias for the refresh callback wrapped in Rc<RefCell<Option<...>>>.
type RefreshCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

/// Create the todo bar widget: a button that shows the top task or "+" when empty.
pub fn create() -> GtkBox {
    let store = Rc::new(RefCell::new(TodoStore::load()));

    let container = GtkBox::new(Orientation::Horizontal, 0);
    container.set_halign(Align::Start);

    // ── Bar Button ───────────────────────────────────────────────────
    let bar_btn = Button::new();
    bar_btn.add_css_class("zenith-todo-btn");
    bar_btn.add_css_class("zenith-module");
    bar_btn.add_css_class("zenith-module-left");
    container.append(&bar_btn);

    // ── Popover ──────────────────────────────────────────────────────
    let popover = Popover::new();
    popover.set_autohide(true);
    popover.set_cascade_popdown(true);
    popover.set_has_arrow(false);
    popover.set_position(gtk4::PositionType::Bottom);
    popover.add_css_class("zenith-todo-popup");
    popover.set_parent(&bar_btn);

    // ── Popover Content ──────────────────────────────────────────────
    let pop_box = GtkBox::new(Orientation::Vertical, 0);
    pop_box.set_width_request(320);
    pop_box.add_css_class("zenith-todo-container");

    // Header
    let header = GtkBox::new(Orientation::Horizontal, 8);
    header.add_css_class("zenith-todo-header");
    let title = Label::new(Some("  Tasks"));
    title.add_css_class("zenith-todo-title");
    title.set_hexpand(true);
    title.set_halign(Align::Start);
    header.append(&title);

    // Progress badge in header
    let progress_label = Label::new(None);
    progress_label.add_css_class("zenith-todo-progress");
    header.append(&progress_label);
    pop_box.append(&header);

    // Progress bar
    let progress_bar = GtkBox::new(Orientation::Horizontal, 0);
    progress_bar.add_css_class("zenith-todo-progress-track");
    let progress_fill = GtkBox::new(Orientation::Horizontal, 0);
    progress_fill.add_css_class("zenith-todo-progress-fill");
    progress_bar.append(&progress_fill);
    pop_box.append(&progress_bar);

    let sep = Separator::new(Orientation::Horizontal);
    sep.add_css_class("zenith-todo-sep");
    pop_box.append(&sep);

    // Scrollable task list
    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_min_content_height(60);
    scroll.set_max_content_height(280);
    scroll.set_propagate_natural_height(true);
    scroll.add_css_class("zenith-todo-scroll");

    let list_box = GtkBox::new(Orientation::Vertical, 2);
    list_box.add_css_class("zenith-todo-list");
    scroll.set_child(Some(&list_box));
    pop_box.append(&scroll);

    // ── Input Row ────────────────────────────────────────────────────
    let sep2 = Separator::new(Orientation::Horizontal);
    sep2.add_css_class("zenith-todo-sep");
    pop_box.append(&sep2);

    let input_row = GtkBox::new(Orientation::Horizontal, 6);
    input_row.add_css_class("zenith-todo-input-row");

    let entry = Entry::new();
    entry.set_placeholder_text(Some("Add a task…"));
    entry.set_hexpand(true);
    entry.add_css_class("zenith-todo-entry");
    input_row.append(&entry);

    let add_btn = Button::with_label("+");
    add_btn.add_css_class("zenith-todo-add-btn");
    input_row.append(&add_btn);

    pop_box.append(&input_row);
    popover.set_child(Some(&pop_box));

    // ── State Refresh Closures ───────────────────────────────────────
    let store_rc = Rc::clone(&store);
    let bar_btn_weak = bar_btn.downgrade();
    let list_box_rc = Rc::new(list_box);
    let progress_label_rc = Rc::new(progress_label);
    let progress_fill_rc = Rc::new(progress_fill);

    // This closure rebuilds the full list and bar label from the current store.
    let refresh: RefreshCallback = Rc::new(RefCell::new(None));
    let refresh_clone = Rc::clone(&refresh);

    let store_for_refresh = Rc::clone(&store_rc);
    let bar_btn_for_refresh = bar_btn_weak.clone();
    let list_box_for_refresh = Rc::clone(&list_box_rc);
    let progress_label_for_refresh = Rc::clone(&progress_label_rc);
    let progress_fill_for_refresh = Rc::clone(&progress_fill_rc);

    let build_refresh = move || {
        let store = Rc::clone(&store_for_refresh);
        let bar_btn_w = bar_btn_for_refresh.clone();
        let list_box = Rc::clone(&list_box_for_refresh);
        let prog_lbl = Rc::clone(&progress_label_for_refresh);
        let prog_fill = Rc::clone(&progress_fill_for_refresh);
        let refresh_self = Rc::clone(&refresh_clone);

        Box::new(move || {
            let s = store.borrow();

            // ── Update bar button ────────────────────────────────
            if let Some(btn) = bar_btn_w.upgrade() {
                if s.items.is_empty() {
                    btn.set_label(" ");
                    btn.remove_css_class("zenith-todo-btn-active");
                    btn.remove_css_class("zenith-todo-btn-urgent");
                    btn.add_css_class("zenith-todo-btn-empty");
                } else {
                    let pending = s.pending_count();
                    let top = s
                        .top_task()
                        .unwrap_or("All done ✓")
                        .chars()
                        .take(28)
                        .collect::<String>();

                    let label = if pending == 0 {
                        format!("✓ {}", top)
                    } else {
                        format!(" {} [{}]", top, pending)
                    };
                    btn.set_label(&label);

                    btn.remove_css_class("zenith-todo-btn-empty");
                    btn.remove_css_class("zenith-todo-btn-urgent");
                    btn.remove_css_class("zenith-todo-btn-active");

                    if pending == 0 {
                        btn.add_css_class("zenith-todo-btn-active");
                    } else if pending >= 5 {
                        btn.add_css_class("zenith-todo-btn-urgent");
                    } else {
                        btn.add_css_class("zenith-todo-btn-active");
                    }
                }
            }

            // ── Update progress label & bar ──────────────────────
            let total = s.items.len();
            let done = s.items.iter().filter(|t| t.done).count();
            prog_lbl.set_label(&format!("{}/{}", done, total));

            let pct = if total > 0 {
                (done as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            // We use margin-end trick: fill goes full width, we clip via CSS
            // Actually, set a size-request proportional fraction of 300px track
            let fill_px = ((pct / 100.0) * 296.0) as i32;
            prog_fill.set_width_request(fill_px.max(0));

            // Color the progress fill based on completion
            prog_fill.remove_css_class("zenith-todo-fill-low");
            prog_fill.remove_css_class("zenith-todo-fill-mid");
            prog_fill.remove_css_class("zenith-todo-fill-high");
            if pct >= 75.0 {
                prog_fill.add_css_class("zenith-todo-fill-high");
            } else if pct >= 40.0 {
                prog_fill.add_css_class("zenith-todo-fill-mid");
            } else {
                prog_fill.add_css_class("zenith-todo-fill-low");
            }

            // ── Rebuild list ─────────────────────────────────────
            // Remove all children
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }

            let items_snapshot: Vec<(usize, TodoItem)> =
                s.items.iter().cloned().enumerate().collect();
            drop(s); // release borrow before building rows

            for (idx, item) in items_snapshot {
                let row = build_todo_row(idx, &item, &store, &refresh_self);
                list_box.append(&row);
            }
        }) as Box<dyn Fn()>
    };

    *refresh.borrow_mut() = Some(build_refresh());

    // Initial paint
    if let Some(ref f) = *refresh.borrow() {
        f();
    }

    // ── Popover toggle ───────────────────────────────────────────────
    bar_btn.connect_clicked({
        let popover = popover.clone();
        move |_| {
            if popover.is_visible() {
                popover.popdown();
            } else {
                popover.popup();
            }
        }
    });

    // ── Add task via button or Enter ─────────────────────────────────
    let add_task = {
        let store = Rc::clone(&store_rc);
        let entry = entry.clone();
        let refresh = Rc::clone(&refresh);
        move || {
            let text = entry.text().trim().to_string();
            if text.is_empty() {
                return;
            }

            // Parse optional priority prefix: "3:Deploy server" → priority=3
            let (priority, task_text) = parse_priority(&text);

            store.borrow_mut().items.push(TodoItem {
                text: task_text,
                done: false,
                priority,
            });
            store.borrow().save();
            entry.set_text("");

            if let Some(ref f) = *refresh.borrow() {
                f();
            }
        }
    };

    add_btn.connect_clicked({
        let add_task = add_task.clone();
        move |_| add_task()
    });

    entry.connect_activate(move |_| add_task());

    container
}

/// Parse "N:text" for priority shorthand. Returns (priority, clean_text).
fn parse_priority(input: &str) -> (u8, String) {
    if input.len() >= 2 {
        let first = input.as_bytes()[0];
        if first.is_ascii_digit() && input.as_bytes()[1] == b':' {
            let prio = first - b'0';
            return (prio, input[2..].trim().to_string());
        }
    }
    (0, input.to_string())
}

/// Build a single todo row widget.
fn build_todo_row(
    idx: usize,
    item: &TodoItem,
    store: &Rc<RefCell<TodoStore>>,
    refresh: &RefreshCallback,
) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.add_css_class("zenith-todo-row");
    if item.done {
        row.add_css_class("zenith-todo-row-done");
    }

    // Priority accent bar (thin colored stripe on the left)
    let accent = GtkBox::new(Orientation::Vertical, 0);
    accent.set_width_request(3);
    accent.set_vexpand(true);
    accent.add_css_class("zenith-todo-accent");
    match item.priority {
        1..=3 => accent.add_css_class("zenith-todo-prio-high"),
        4..=6 => accent.add_css_class("zenith-todo-prio-mid"),
        7..=9 => accent.add_css_class("zenith-todo-prio-low"),
        _ => accent.add_css_class("zenith-todo-prio-none"),
    }
    row.append(&accent);

    // Checkbox
    let check = CheckButton::new();
    check.set_active(item.done);
    check.add_css_class("zenith-todo-check");
    row.append(&check);

    // Task text
    let label = Label::new(Some(&item.text));
    label.set_hexpand(true);
    label.set_halign(Align::Start);
    label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    label.add_css_class("zenith-todo-text");
    if item.done {
        label.add_css_class("zenith-todo-text-done");
    }
    row.append(&label);

    // Priority badge (if set)
    if item.priority > 0 {
        let badge = Label::new(Some(&format!("P{}", item.priority)));
        badge.add_css_class("zenith-todo-badge");
        match item.priority {
            1..=3 => badge.add_css_class("zenith-todo-badge-high"),
            4..=6 => badge.add_css_class("zenith-todo-badge-mid"),
            _ => badge.add_css_class("zenith-todo-badge-low"),
        }
        row.append(&badge);
    }

    // Move up button
    if idx > 0 {
        let up_btn = Button::with_label("▲");
        up_btn.add_css_class("zenith-todo-move-btn");
        let store_c = Rc::clone(store);
        let refresh_c = Rc::clone(refresh);
        up_btn.connect_clicked(move |_| {
            let mut s = store_c.borrow_mut();
            if idx > 0 && idx < s.items.len() {
                s.items.swap(idx, idx - 1);
                s.save();
            }
            drop(s);
            if let Some(ref f) = *refresh_c.borrow() {
                f();
            }
        });
        row.append(&up_btn);
    }

    // Delete button
    let del_btn = Button::with_label("✕");
    del_btn.add_css_class("zenith-todo-del-btn");
    let store_c = Rc::clone(store);
    let refresh_c = Rc::clone(refresh);
    del_btn.connect_clicked(move |_| {
        let mut s = store_c.borrow_mut();
        if idx < s.items.len() {
            s.items.remove(idx);
            s.save();
        }
        drop(s);
        if let Some(ref f) = *refresh_c.borrow() {
            f();
        }
    });
    row.append(&del_btn);

    // Checkbox toggle
    let store_c = Rc::clone(store);
    let refresh_c = Rc::clone(refresh);
    check.connect_toggled(move |cb| {
        let mut s = store_c.borrow_mut();
        if idx < s.items.len() {
            s.items[idx].done = cb.is_active();
            s.save();
        }
        drop(s);
        if let Some(ref f) = *refresh_c.borrow() {
            f();
        }
    });

    row
}
