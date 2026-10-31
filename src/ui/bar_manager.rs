//! One bar per matching output, following hotplug.
//!
//! The decision of which outputs get a bar is pure ([`placement::reconcile`]); this file
//! only applies it whenever GDK's output list changes. GDK calls an output a
//! `gdk::Monitor`; everything here that is ours says output. Bars share the process's sources,
//! so an extra output adds widgets, never polling.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{gdk, gio, glib};

use super::bar;
use super::placement;
use super::sources::Sources;
use crate::config::ResolvedConfig;

pub struct BarManager {
    inner: Rc<Inner>,
    outputs: gio::ListModel,
    handler: Option<glib::SignalHandlerId>,
}

struct Inner {
    app: gtk4::Application,
    config: ResolvedConfig,
    sources: Sources,
    /// Each bar with the output it is on.
    bars: RefCell<Vec<(gdk::Monitor, gtk4::ApplicationWindow)>>,
}

impl BarManager {
    pub fn start(
        app: &gtk4::Application,
        display: &gdk::Display,
        config: &ResolvedConfig,
        sources: &Sources,
    ) -> Self {
        let inner = Rc::new(Inner {
            app: app.clone(),
            config: config.clone(),
            sources: sources.clone(),
            bars: RefCell::default(),
        });
        let outputs = display.monitors();
        for i in 0..outputs.n_items() {
            watch_connector(&inner, &outputs, i);
        }
        inner.sync(&outputs);

        let weak = Rc::downgrade(&inner);
        let handler = outputs.connect_items_changed(move |outputs, position, _, added| {
            let Some(inner) = weak.upgrade() else { return };
            for i in position..position + added {
                watch_connector(&inner, outputs, i);
            }
            inner.sync(outputs);
        });
        Self {
            inner,
            outputs,
            handler: Some(handler),
        }
    }
}

impl Inner {
    fn sync(&self, model: &gio::ListModel) {
        let outputs: Vec<gdk::Monitor> = model
            .iter::<gdk::Monitor>()
            .filter_map(Result::ok)
            .collect();
        let connectors: Vec<Option<glib::GString>> =
            outputs.iter().map(|output| output.connector()).collect();
        let current: Vec<(gdk::Monitor, Option<&str>)> = outputs
            .iter()
            .cloned()
            .zip(connectors.iter().map(|c| c.as_deref()))
            .collect();
        let have: Vec<gdk::Monitor> = self
            .bars
            .borrow()
            .iter()
            .map(|(output, _)| output.clone())
            .collect();

        let changes = placement::reconcile(&have, &current, &self.config.bar.monitors);

        let mut bars = self.bars.borrow_mut();
        for output in &changes.remove {
            log::info!("removing bar from {}", name(output));
            if let Some(i) = bars.iter().position(|(o, _)| o == output) {
                bars.remove(i).1.destroy();
            }
        }
        for output in changes.add {
            log::info!("adding bar on {}", name(&output));
            let window = bar::build(&self.app, Some(&output), &self.config, &self.sources);
            window.present();
            bars.push((output, window));
        }
    }
}

/// A hotplugged output can join the list before its connector name is known (seen on
/// Sway), so a `monitors` list only matches it once the name arrives: sync again then.
fn watch_connector(inner: &Rc<Inner>, outputs: &gio::ListModel, i: u32) {
    let Some(output) = outputs.item(i).and_downcast::<gdk::Monitor>() else {
        return;
    };
    let weak = Rc::downgrade(inner);
    let outputs = outputs.downgrade();
    output.connect_connector_notify(move |_| {
        if let (Some(inner), Some(outputs)) = (weak.upgrade(), outputs.upgrade()) {
            inner.sync(&outputs);
        }
    });
}

fn name(output: &gdk::Monitor) -> String {
    output
        .connector()
        .map_or_else(|| "an unnamed output".into(), |c| c.to_string())
}

impl Drop for BarManager {
    fn drop(&mut self) {
        if let Some(handler) = self.handler.take() {
            self.outputs.disconnect(handler);
        }
        for (_, bar) in self.inner.bars.borrow_mut().drain(..) {
            bar.destroy();
        }
    }
}
