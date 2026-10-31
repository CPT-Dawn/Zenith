//! The todo view: a count on the bar that opens a popover to manage the list.
//!
//! Keyboard: Tab moves between the add entry, the priority picker and every row's
//! controls; Enter adds; Space toggles a check; on a row, F2 edits, Delete deletes,
//! and the priority button cycles high → normal → low. Escape cancels an edit or
//! closes the popover. Focus is restored to the same row after every change.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{gdk, glib};
use gtk4_layer_shell::{KeyboardMode, LayerShell};

use super::model::{Command, Priority, Todo, TodoList};
use super::source::TodoSource;
use crate::ui::binding::bind;

pub fn build(source: &Rc<TodoSource>) -> gtk4::Widget {
    let icon = gtk4::Image::new();
    let count = gtk4::Label::new(None);
    let content = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    content.append(&icon);
    content.append(&count);

    let button = gtk4::MenuButton::builder()
        .child(&content)
        .always_show_arrow(false)
        .build();
    bind(&button, &source.state(), move |_, state| {
        icon.set_icon_name(state.icon.as_deref());
        count.set_label(&state.label);
    });
    button.set_popover(Some(&popover(source)));
    button.upcast()
}

fn popover(source: &Rc<TodoSource>) -> gtk4::Popover {
    let popover = gtk4::Popover::builder()
        .has_arrow(false)
        .css_classes(["zb-popover", "zb-todo-popover"])
        .build();

    // The bar never takes the keyboard, except while this popover is open.
    popover.connect_show(|popover| set_keyboard(popover, KeyboardMode::Exclusive));
    popover.connect_closed(|popover| set_keyboard(popover, KeyboardMode::None));

    let entry = gtk4::Entry::builder()
        .placeholder_text("Add a todo…")
        .hexpand(true)
        .build();
    let picker = gtk4::DropDown::from_strings(&Priority::ALL.map(Priority::label));
    picker.set_selected(1);
    picker.set_tooltip_text(Some("Priority for new todos"));
    let add_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    add_row.add_css_class("zb-todo-add");
    add_row.append(&entry);
    add_row.append(&picker);

    let rows = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["zb-todo-list"])
        .build();
    let empty = gtk4::Label::new(Some("Nothing to do. Enjoy it."));
    empty.add_css_class("zb-todo-empty");
    rows.set_placeholder(Some(&empty));
    let scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .propagate_natural_height(true)
        .max_content_height(360)
        .child(&rows)
        .build();

    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    content.set_width_request(320);
    content.append(&add_row);
    content.append(&scroll);
    popover.set_child(Some(&content));

    {
        let source = source.clone();
        let picker = picker.clone();
        entry.connect_activate(move |entry| {
            let priority = Priority::ALL[(picker.selected() as usize).min(2)];
            source.apply(Command::Add {
                text: entry.text().into(),
                priority,
            });
            entry.set_text("");
        });
    }
    popover.set_default_widget(None::<&gtk4::Widget>);
    {
        let entry = entry.clone();
        popover.connect_show(move |_| {
            entry.grab_focus();
        });
    }

    let list = source.list();
    let built: Rc<RefCell<Vec<Row>>> = Rc::default();
    let weak_rows = rows.downgrade();
    let weak_entry = entry.downgrade();
    let source_weak = Rc::downgrade(source);
    let subscription = list.subscribe(move |list| {
        let (Some(rows), Some(entry), Some(source)) = (
            weak_rows.upgrade(),
            weak_entry.upgrade(),
            source_weak.upgrade(),
        ) else {
            return;
        };
        rebuild(&rows, &entry, &source, list, &built);
    });
    popover.connect_destroy(move |_| list.unsubscribe(subscription));

    popover
}

/// One rendered row and its focusable parts, for restoring focus after a rebuild.
struct Row {
    id: u64,
    row: gtk4::ListBoxRow,
    parts: [gtk4::Widget; 4],
}

fn rebuild(
    rows: &gtk4::ListBox,
    entry: &gtk4::Entry,
    source: &Rc<TodoSource>,
    list: &TodoList,
    built: &RefCell<Vec<Row>>,
) {
    // Where was the focus: (todo id, part index, row index)?
    let focus = rows.root().and_then(|root| root.focus());
    let focused = focus.as_ref().and_then(|focus| {
        built.borrow().iter().enumerate().find_map(|(index, row)| {
            row.parts
                .iter()
                .position(|part| focus == part || focus.is_ancestor(part))
                .map(|part| (row.id, part, index))
        })
    });

    for row in built.borrow_mut().drain(..) {
        rows.remove(&row.row);
    }
    let mut new = Vec::new();
    for todo in list.items() {
        let row = row(todo, source);
        rows.append(&row.row);
        new.push(row);
    }

    if let Some((id, part, index)) = focused {
        let target = new
            .iter()
            .find(|row| row.id == id)
            .map(|row| &row.parts[part])
            .or_else(|| {
                new.get(index.min(new.len().saturating_sub(1)))
                    .map(|r| &r.parts[0])
            });
        match target {
            Some(widget) => widget.grab_focus(),
            None => entry.grab_focus(),
        };
    }
    *built.borrow_mut() = new;
}

fn row(todo: &Todo, source: &Rc<TodoSource>) -> Row {
    let id = todo.id;

    let check = gtk4::CheckButton::builder()
        .active(todo.done)
        .tooltip_text(if todo.done { "Reopen" } else { "Complete" })
        .build();
    let text = gtk4::Label::builder()
        .label(&todo.text)
        .hexpand(true)
        .xalign(0.0)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .css_classes(["zb-todo-text"])
        .build();
    let editor = gtk4::Entry::builder()
        .text(&todo.text)
        .hexpand(true)
        .build();
    let stack = gtk4::Stack::new();
    stack.add_named(&text, Some("text"));
    stack.add_named(&editor, Some("edit"));
    stack.set_hexpand(true);

    let priority = gtk4::Button::builder()
        .label(todo.priority.label())
        .tooltip_text("Change priority")
        .css_classes(["zb-todo-priority", todo.priority.css_class()])
        .build();
    let edit = icon_button("✎", "Edit");
    let delete = icon_button("✕", "Delete");

    let line = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    for widget in [
        check.upcast_ref::<gtk4::Widget>(),
        stack.upcast_ref(),
        priority.upcast_ref(),
        edit.upcast_ref(),
        delete.upcast_ref(),
    ] {
        line.append(widget);
    }
    let row = gtk4::ListBoxRow::builder()
        .child(&line)
        .activatable(false)
        .css_classes(["zb-todo-row"])
        .build();
    if todo.done {
        row.add_css_class("zb-done");
    }
    if todo.priority == Priority::High && !todo.done {
        row.add_css_class("zb-high");
    }

    let send = {
        let source = Rc::downgrade(source);
        move |command: Command| {
            if let Some(source) = source.upgrade() {
                // Rebuilding the list destroys the widget emitting this signal; do it after.
                glib::idle_add_local_once(move || source.apply(command));
            }
        }
    };

    check.connect_toggled({
        let send = send.clone();
        move |_| send(Command::Toggle(id))
    });
    priority.connect_clicked({
        let send = send.clone();
        let next = todo.priority.next();
        move |_| send(Command::SetPriority { id, priority: next })
    });
    delete.connect_clicked({
        let send = send.clone();
        move |_| send(Command::Delete(id))
    });

    let start_edit = {
        let stack = stack.clone();
        let editor = editor.clone();
        move || {
            stack.set_visible_child_name("edit");
            editor.grab_focus();
        }
    };
    edit.connect_clicked({
        let start_edit = start_edit.clone();
        move |_| start_edit()
    });
    editor.connect_activate({
        let send = send.clone();
        let stack = stack.clone();
        move |editor| {
            stack.set_visible_child_name("text");
            send(Command::Edit {
                id,
                text: editor.text().into(),
            });
        }
    });
    // Escape cancels the edit instead of closing the popover.
    let cancel = gtk4::EventControllerKey::new();
    cancel.connect_key_pressed({
        let stack = stack.clone();
        let editor = editor.clone();
        let edit = edit.clone();
        let original = todo.text.clone();
        move |_, key, _, _| {
            if key != gdk::Key::Escape {
                return glib::Propagation::Proceed;
            }
            editor.set_text(&original);
            stack.set_visible_child_name("text");
            edit.grab_focus();
            glib::Propagation::Stop
        }
    });
    editor.add_controller(cancel);

    // Row shortcuts, while focus is anywhere in the row but the editor.
    let keys = gtk4::EventControllerKey::new();
    keys.connect_key_pressed({
        let stack = stack.clone();
        move |_, key, _, _| {
            if stack.visible_child_name().as_deref() == Some("edit") {
                return glib::Propagation::Proceed;
            }
            match key {
                gdk::Key::F2 => start_edit(),
                gdk::Key::Delete | gdk::Key::KP_Delete => send(Command::Delete(id)),
                _ => return glib::Propagation::Proceed,
            }
            glib::Propagation::Stop
        }
    });
    row.add_controller(keys);

    Row {
        id,
        row,
        parts: [
            check.upcast(),
            priority.upcast(),
            edit.upcast(),
            delete.upcast(),
        ],
    }
}

fn icon_button(glyph: &str, name: &str) -> gtk4::Button {
    let button = gtk4::Button::builder()
        .label(glyph)
        .tooltip_text(name)
        .css_classes(["zb-todo-action"])
        .build();
    button.update_property(&[gtk4::accessible::Property::Label(name)]);
    button
}

fn set_keyboard(popover: &gtk4::Popover, mode: KeyboardMode) {
    if let Some(window) = popover
        .parent()
        .and_then(|p| p.root())
        .and_downcast::<gtk4::Window>()
    {
        window.set_keyboard_mode(mode);
    }
}
