//! Decides which outputs get a bar. For now: the first output only.
//! Output selection (`monitors`) and hotplug build on this.

use gtk4::gdk;
use gtk4::prelude::*;

use super::bar;
use super::sources::Sources;
use crate::config::ResolvedConfig;

pub struct BarManager {
    bars: Vec<gtk4::ApplicationWindow>,
}

impl BarManager {
    pub fn start(app: &gtk4::Application, config: &ResolvedConfig, sources: &Sources) -> Self {
        let monitor = gdk::Display::default()
            .and_then(|display| display.monitors().item(0))
            .and_then(|item| item.downcast::<gdk::Monitor>().ok());

        let window = bar::build(app, monitor.as_ref(), config, sources);
        window.present();
        Self { bars: vec![window] }
    }
}

impl Drop for BarManager {
    fn drop(&mut self) {
        for bar in self.bars.drain(..) {
            bar.destroy();
        }
    }
}
