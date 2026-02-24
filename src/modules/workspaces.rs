use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation};

/// Create a placeholder widget for the Workspaces module.
///
/// This returns a horizontal box with numbered workspace indicators.
/// In a full implementation this would listen to the Hyprland IPC socket
/// (`$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket2.sock`) for
/// workspace change events; for now it displays static placeholders.
pub fn create() -> GtkBox {
    let container = GtkBox::new(Orientation::Horizontal, 6);
    container.set_halign(gtk4::Align::Start);

    for i in 1..=5 {
        let ws = Label::new(Some(&format!(" {} ", i)));
        ws.add_css_class("zenith-module");
        ws.add_css_class("zenith-module-left");
        container.append(&ws);
    }

    container
}
