//! The one media source per process: MPRIS over gio D-Bus, all async, no polling.
//!
//! Players are keyed by their unique bus name (`:1.42`), because that is what signals
//! carry. `NameOwnerChanged` (filtered to `org.mpris.MediaPlayer2.*` by the bus itself)
//! tells us when players come, go or restart; `PropertiesChanged` and `Seeked` keep the
//! model current. The only timer is the model's progress tick, armed only while playing.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::time::Duration;

use gtk4::gio::prelude::*;
use gtk4::{gio, glib};

use super::{Media, MediaConfig, PropertyMap, Value};
use crate::modules::{Shared, ViewState};

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2";
const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
const PLAYER_IFACE: &str = "org.mpris.MediaPlayer2.Player";
const DBUS_NAME: &str = "org.freedesktop.DBus";
const DBUS_PATH: &str = "/org/freedesktop/DBus";
const PROPERTIES_IFACE: &str = "org.freedesktop.DBus.Properties";

pub struct MediaSource {
    state: Rc<Shared<ViewState>>,
    config: RefCell<MediaConfig>,
    media: RefCell<Media>,
    bus: RefCell<Option<gio::DBusConnection>>,
    /// Well-known `org.mpris.MediaPlayer2.*` name → unique owner.
    owners: RefCell<HashMap<String, String>>,
    signals: RefCell<Vec<gio::SignalSubscription>>,
    timer: RefCell<Option<glib::SourceId>>,
}

impl MediaSource {
    pub fn start(config: MediaConfig) -> Rc<Self> {
        let source = Rc::new(Self {
            state: Shared::new(ViewState::default()),
            config: RefCell::new(config),
            media: RefCell::new(Media::new()),
            bus: RefCell::new(None),
            owners: RefCell::default(),
            signals: RefCell::default(),
            timer: RefCell::new(None),
        });
        source.publish();
        source.connect();
        source
    }

    pub fn state(&self) -> Rc<Shared<ViewState>> {
        self.state.clone()
    }

    pub fn reconfigure(self: &Rc<Self>, config: MediaConfig) {
        *self.config.borrow_mut() = config;
        self.publish();
    }

    /// The built-in click action: play/pause the player the module is showing.
    pub fn play_pause(&self) {
        let Some(bus) = self.bus.borrow().clone() else {
            return;
        };
        let Some(player) = self.media.borrow().active_player().map(str::to_owned) else {
            return;
        };
        let target = player.clone();
        bus.call(
            Some(&target),
            MPRIS_PATH,
            PLAYER_IFACE,
            "PlayPause",
            None,
            None,
            gio::DBusCallFlags::NONE,
            -1,
            gio::Cancellable::NONE,
            move |result| {
                if let Err(e) = result {
                    log::info!("media: PlayPause on {player} failed: {e}");
                }
            },
        );
    }

    /// Publish the model's state and re-arm the progress tick (only while playing).
    fn publish(self: &Rc<Self>) {
        if let Some(timer) = self.timer.take() {
            timer.remove();
        }
        let mut state = self.media.borrow().view_state(&self.config.borrow(), now());
        let wake = state.next_wake.take();
        self.state.set(state);

        if let Some(wake) = wake {
            let weak = Rc::downgrade(self);
            let timer = glib::timeout_add_local_once(wake, move || {
                if let Some(source) = weak.upgrade() {
                    // This timer has fired; forget it rather than remove it.
                    source.timer.take();
                    source.publish();
                }
            });
            *self.timer.borrow_mut() = Some(timer);
        }
    }

    fn connect(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        gio::bus_get(gio::BusType::Session, gio::Cancellable::NONE, move |bus| {
            let Some(source) = weak.upgrade() else { return };
            match bus {
                Ok(bus) => source.watch(bus),
                Err(e) => log::warn!("media: no session bus, showing no media: {e}"),
            }
        });
    }

    /// Subscribe to every signal first, then list the players already running.
    fn watch(self: &Rc<Self>, bus: gio::DBusConnection) {
        *self.bus.borrow_mut() = Some(bus.clone());
        let mut signals = self.signals.borrow_mut();

        let weak = Rc::downgrade(self);
        signals.push(bus.subscribe_to_signal(
            Some(DBUS_NAME),
            Some(DBUS_NAME),
            Some("NameOwnerChanged"),
            Some(DBUS_PATH),
            Some(MPRIS_PREFIX),
            gio::DBusSignalFlags::MATCH_ARG0_NAMESPACE,
            move |signal| {
                let Some((name, old, new)) = signal.parameters.get::<(String, String, String)>()
                else {
                    return;
                };
                with(&weak, |source| source.owner_changed(&name, &old, &new));
            },
        ));

        let weak = Rc::downgrade(self);
        signals.push(bus.subscribe_to_signal(
            None,
            Some(PROPERTIES_IFACE),
            Some("PropertiesChanged"),
            Some(MPRIS_PATH),
            Some(PLAYER_IFACE),
            gio::DBusSignalFlags::NONE,
            move |signal| {
                let sender = signal.sender_name.to_owned();
                let changed = map(&signal.parameters.child_value(1));
                let invalidated = signal.parameters.child_value(2).n_children() > 0;
                with(&weak, |source| {
                    source.changed(&sender, &changed);
                    if invalidated {
                        source.fetch_all(&sender);
                    }
                });
            },
        ));

        let weak = Rc::downgrade(self);
        signals.push(bus.subscribe_to_signal(
            None,
            Some(PLAYER_IFACE),
            Some("Seeked"),
            Some(MPRIS_PATH),
            None,
            gio::DBusSignalFlags::NONE,
            move |signal| {
                let sender = signal.sender_name.to_owned();
                let Some((position,)) = signal.parameters.get::<(i64,)>() else {
                    return;
                };
                with(&weak, |source| {
                    source.media.borrow_mut().seeked(&sender, position, now());
                    source.publish();
                });
            },
        ));
        drop(signals);

        let weak = Rc::downgrade(self);
        bus.call(
            Some(DBUS_NAME),
            DBUS_PATH,
            DBUS_NAME,
            "ListNames",
            None,
            Some(glib::VariantTy::new("(as)").expect("valid type")),
            gio::DBusCallFlags::NONE,
            -1,
            gio::Cancellable::NONE,
            move |reply| {
                let names = match reply.map(|r| r.get::<(Vec<String>,)>()) {
                    Ok(Some((names,))) => names,
                    Ok(None) => return,
                    Err(e) => {
                        log::warn!("media: cannot list players: {e}");
                        return;
                    }
                };
                with(&weak, |source| {
                    for name in names.iter().filter(|n| is_mpris(n)) {
                        source.resolve_owner(name);
                    }
                });
            },
        );
    }

    /// A player already on the bus: find its unique name, then load it.
    fn resolve_owner(self: &Rc<Self>, name: &str) {
        let Some(bus) = self.bus.borrow().clone() else {
            return;
        };
        let weak = Rc::downgrade(self);
        let name = name.to_owned();
        bus.call(
            Some(DBUS_NAME),
            DBUS_PATH,
            DBUS_NAME,
            "GetNameOwner",
            Some(&(name.as_str(),).to_variant()),
            Some(glib::VariantTy::new("(s)").expect("valid type")),
            gio::DBusCallFlags::NONE,
            -1,
            gio::Cancellable::NONE,
            move |reply| {
                // The player may have quit in between; NameOwnerChanged covers that.
                let Ok(Some((owner,))) = reply.map(|r| r.get::<(String,)>()) else {
                    return;
                };
                with(&weak, |source| source.owner_changed(&name, "", &owner));
            },
        );
    }

    fn owner_changed(self: &Rc<Self>, name: &str, old: &str, new: &str) {
        if !is_mpris(name) {
            return;
        }
        let mut owners = self.owners.borrow_mut();
        if !old.is_empty() {
            owners.remove(name);
            // One process may own several MPRIS names; it leaves when the last one goes.
            if !owners.values().any(|owner| owner == old) {
                log::debug!("media: {name} ({old}) left");
                self.media.borrow_mut().player_removed(old);
            }
        }
        if !new.is_empty() {
            owners.insert(name.to_owned(), new.to_owned());
        }
        drop(owners);
        if !new.is_empty() {
            log::debug!("media: {name} ({new}) appeared");
            self.fetch_all(new);
        }
        self.publish();
    }

    fn changed(self: &Rc<Self>, player: &str, changed: &PropertyMap) {
        self.media
            .borrow_mut()
            .properties_changed(player, changed, now());
        // Position is never signalled; ask for it when the estimate may have drifted.
        if changed.contains_key("PlaybackStatus") || changed.contains_key("Metadata") {
            self.fetch_position(player);
        }
        self.publish();
    }

    /// Load (or reload) a player from its `GetAll` reply.
    fn fetch_all(self: &Rc<Self>, name: &str) {
        let player = name.to_owned();
        self.get(
            name,
            "GetAll",
            (PLAYER_IFACE,).to_variant(),
            move |source, reply| {
                let properties = map(&reply.child_value(0));
                source
                    .media
                    .borrow_mut()
                    .player_added(&player, &properties, now());
                source.publish();
            },
        );
    }

    fn fetch_position(self: &Rc<Self>, name: &str) {
        let player = name.to_owned();
        let arguments = (PLAYER_IFACE, "Position").to_variant();
        self.get(name, "Get", arguments, move |source, reply| {
            let position = value(&reply.child_value(0));
            let changed = PropertyMap::from([("Position".to_owned(), position)]);
            source
                .media
                .borrow_mut()
                .properties_changed(&player, &changed, now());
            source.publish();
        });
    }

    /// Call an `org.freedesktop.DBus.Properties` method on a player, async.
    fn get(
        self: &Rc<Self>,
        player: &str,
        method: &str,
        arguments: glib::Variant,
        on_reply: impl FnOnce(&Rc<Self>, glib::Variant) + 'static,
    ) {
        let Some(bus) = self.bus.borrow().clone() else {
            return;
        };
        let weak = Rc::downgrade(self);
        let player_name = player.to_owned();
        let method_name = method.to_owned();
        bus.call(
            Some(player),
            MPRIS_PATH,
            PROPERTIES_IFACE,
            method,
            Some(&arguments),
            None,
            gio::DBusCallFlags::NONE,
            -1,
            gio::Cancellable::NONE,
            move |reply| match reply {
                Ok(reply) => with(&weak, |source| on_reply(source, reply)),
                Err(e) => log::info!("media: {method_name} on {player_name} failed: {e}"),
            },
        );
    }
}

impl Drop for MediaSource {
    fn drop(&mut self) {
        // Signal subscriptions unsubscribe themselves when dropped.
        if let Some(timer) = self.timer.take() {
            timer.remove();
        }
    }
}

fn with(weak: &Weak<MediaSource>, f: impl FnOnce(&Rc<MediaSource>)) {
    if let Some(source) = weak.upgrade() {
        f(&source);
    }
}

fn is_mpris(name: &str) -> bool {
    name.strip_prefix(MPRIS_PREFIX)
        .is_some_and(|rest| rest.starts_with('.'))
}

/// The monotonic clock the model's progress estimate runs on.
fn now() -> Duration {
    Duration::from_micros(glib::monotonic_time().max(0) as u64)
}

/// An `a{sv}` as a model property map; anything else is empty.
fn map(variant: &glib::Variant) -> PropertyMap {
    let mut out = PropertyMap::new();
    if variant.type_().as_str() != "a{sv}" {
        return out;
    }
    for entry in variant.iter() {
        let key = entry.child_value(0);
        if let Some(key) = key.str() {
            out.insert(key.to_owned(), value(&entry.child_value(1)));
        }
    }
    out
}

/// Convert a D-Bus value to the model's plain [`Value`], unboxing variants.
fn value(variant: &glib::Variant) -> Value {
    use glib::VariantClass::*;
    match variant.classify() {
        Variant => variant.as_variant().map_or(Value::Other, |v| value(&v)),
        Boolean => variant.get().map_or(Value::Other, Value::Bool),
        Int16 => variant
            .get::<i16>()
            .map_or(Value::Other, |n| Value::I64(n.into())),
        Int32 => variant
            .get::<i32>()
            .map_or(Value::Other, |n| Value::I64(n.into())),
        Int64 => variant.get().map_or(Value::Other, Value::I64),
        Byte => variant
            .get::<u8>()
            .map_or(Value::Other, |n| Value::U64(n.into())),
        Uint16 => variant
            .get::<u16>()
            .map_or(Value::Other, |n| Value::U64(n.into())),
        Uint32 => variant
            .get::<u32>()
            .map_or(Value::Other, |n| Value::U64(n.into())),
        Uint64 => variant.get().map_or(Value::Other, Value::U64),
        Double => variant.get().map_or(Value::Other, Value::F64),
        String | ObjectPath => variant
            .str()
            .map_or(Value::Other, |s| Value::Str(s.to_owned())),
        Array => match variant.type_().as_str() {
            "as" | "ao" => Value::StrList(
                variant
                    .iter()
                    .filter_map(|item| item.str().map(str::to_owned))
                    .collect(),
            ),
            "a{sv}" => Value::Map(map(variant)),
            _ => Value::Other,
        },
        _ => Value::Other,
    }
}
