use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation};

/// Create a placeholder widget for System Stats / Tray on the right side.
///
/// A full implementation would read from `/proc/stat`, `/proc/meminfo`,
/// `/sys/class/power_supply/`, or subscribe to `upower` D-Bus signals.
/// For now, static placeholder labels are shown.
pub fn create() -> GtkBox {
    let container = GtkBox::new(Orientation::Horizontal, 12);
    container.set_halign(gtk4::Align::End);

    let cpu = Label::new(Some("CPU: --%"));
    cpu.add_css_class("zenith-module");
    cpu.add_css_class("zenith-module-right");
    container.append(&cpu);

    let mem = Label::new(Some("MEM: --%"));
    mem.add_css_class("zenith-module");
    mem.add_css_class("zenith-module-right");
    container.append(&mem);

    container
}
