//! Pure media model: MPRIS property maps and a monotonic `now` in, a [`ViewState`] out.

use std::collections::BTreeMap;
use std::time::Duration;

use super::MediaConfig;
use crate::modules::{ViewState, truncate};

/// A D-Bus value, converted from a `Variant` at the source boundary.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    Str(String),
    StrList(Vec<String>),
    Map(PropertyMap),
    Other,
}

/// An MPRIS property map (`a{sv}`), e.g. the `changed` argument of `PropertiesChanged`.
pub type PropertyMap = BTreeMap<String, Value>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

impl PlaybackStatus {
    fn parse(value: &Value) -> Option<Self> {
        match value {
            Value::Str(s) if s == "Playing" => Some(Self::Playing),
            Value::Str(s) if s == "Paused" => Some(Self::Paused),
            Value::Str(s) if s == "Stopped" => Some(Self::Stopped),
            _ => None,
        }
    }
}

/// Steps assumed until the view reports the progress bar's width.
const UNMEASURED_STEPS: u32 = 200;
/// Even short tracks (or very wide bars) tick at most this often.
const MIN_TICK: Duration = Duration::from_millis(250);

/// What a player has loaded, from its `Metadata` map.
#[derive(Clone, Debug, Default, PartialEq)]
struct Track {
    id: String,
    title: String,
    artists: Vec<String>,
    album: String,
    length: Option<Duration>,
}

impl Track {
    fn parse(value: &Value) -> Self {
        let Value::Map(map) = value else {
            return Self::default();
        };
        Self {
            id: text(map.get("mpris:trackid")),
            title: text(map.get("xesam:title")),
            artists: list(map.get("xesam:artist")),
            album: text(map.get("xesam:album")),
            length: micros(map.get("mpris:length")).filter(|length| !length.is_zero()),
        }
    }

    /// Whether `other` is a different song, not just new metadata (art, rating, …) for this one.
    fn same_song(&self, other: &Track) -> bool {
        self.id == other.id && self.title == other.title && self.artists == other.artists
    }
}

#[derive(Clone, Debug)]
struct Player {
    name: String,
    status: PlaybackStatus,
    track: Track,
    rate: f64,
    /// The position at `anchored_at`; the estimate runs on from there while playing.
    position: Duration,
    anchored_at: Duration,
    /// When this player last appeared or started playing, as an event counter.
    active_since: u64,
}

impl Player {
    fn new(name: &str, now: Duration) -> Self {
        Self {
            name: name.into(),
            status: PlaybackStatus::Stopped,
            track: Track::default(),
            rate: 1.0,
            position: Duration::ZERO,
            anchored_at: now,
            active_since: 0,
        }
    }

    fn apply(&mut self, changed: &PropertyMap, now: Duration) {
        // Re-anchor first, so a status or rate change only affects time from now on.
        self.anchor(self.position(now), now);
        if let Some(status) = changed
            .get("PlaybackStatus")
            .and_then(PlaybackStatus::parse)
        {
            self.status = status;
        }
        if let Some(rate) = number(changed.get("Rate")) {
            self.rate = rate.max(0.0);
        }
        if let Some(metadata) = changed.get("Metadata") {
            let track = Track::parse(metadata);
            if !track.same_song(&self.track) {
                self.anchor(Duration::ZERO, now);
            }
            self.track = track;
        }
        if let Some(position) = micros(changed.get("Position")) {
            self.anchor(position, now);
        }
    }

    fn anchor(&mut self, position: Duration, now: Duration) {
        self.position = position;
        self.anchored_at = now;
    }

    /// Estimated position at `now`, never past the end of the track.
    fn position(&self, now: Duration) -> Duration {
        let mut position = self.position;
        if self.status == PlaybackStatus::Playing {
            position += now.saturating_sub(self.anchored_at).mul_f64(self.rate);
        }
        match self.track.length {
            Some(length) => position.min(length),
            None => position,
        }
    }

    fn progress(&self, now: Duration) -> Option<f64> {
        let length = self.track.length?;
        Some(self.position(now).as_secs_f64() / length.as_secs_f64())
    }

    /// How long until a progress bar `steps` wide can move by one step, if it is moving.
    fn next_tick(&self, now: Duration, steps: u32) -> Option<Duration> {
        let length = self.track.length?;
        let moving = self.status == PlaybackStatus::Playing && self.rate > 0.0;
        if !moving || self.position(now) >= length {
            return None;
        }
        Some((length.div_f64(self.rate) / steps.max(1)).max(MIN_TICK))
    }
}

/// Every player on the bus, and which one the bar follows.
#[derive(Clone, Debug, Default)]
pub struct Media {
    players: Vec<Player>,
    events: u64,
    /// The progress bar's width in pixels, once a view has been laid out.
    progress_width: Option<u32>,
}

impl Media {
    pub fn new() -> Self {
        Self::default()
    }

    /// The progress bar is `pixels` wide: it can show one step per pixel, so the
    /// tick never runs finer than that. A hidden (zero-width) bar keeps the last width.
    pub fn set_progress_width(&mut self, pixels: u32) {
        if pixels > 0 {
            self.progress_width = Some(pixels);
        }
    }

    /// A player appeared; `properties` is its `GetAll` reply. `name` is its unique bus name.
    pub fn player_added(&mut self, name: &str, properties: &PropertyMap, now: Duration) {
        self.player_removed(name);
        let mut player = Player::new(name, now);
        player.apply(properties, now);
        player.active_since = self.tick();
        self.players.push(player);
    }

    /// A player left the bus (it quit, crashed or is restarting).
    pub fn player_removed(&mut self, name: &str) {
        self.players.retain(|p| p.name != name);
    }

    /// A player's `PropertiesChanged` signal. Unknown senders are ignored.
    pub fn properties_changed(&mut self, name: &str, changed: &PropertyMap, now: Duration) {
        let event = self.tick();
        if let Some(player) = self.player_mut(name) {
            let was_playing = player.status == PlaybackStatus::Playing;
            player.apply(changed, now);
            if !was_playing && player.status == PlaybackStatus::Playing {
                player.active_since = event;
            }
        }
    }

    /// A player's `Seeked` signal: it jumped to `position` microseconds.
    pub fn seeked(&mut self, name: &str, position: i64, now: Duration) {
        let changed = PropertyMap::from([("Position".to_string(), Value::I64(position))]);
        self.properties_changed(name, &changed, now);
    }

    /// The unique bus name of the player the bar follows: the one that most recently
    /// started playing, otherwise the most recently active one. Stopped players never count.
    pub fn active_player(&self) -> Option<&str> {
        self.active().map(|p| p.name.as_str())
    }

    fn tick(&mut self) -> u64 {
        self.events += 1;
        self.events
    }

    fn player_mut(&mut self, name: &str) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.name == name)
    }

    fn active(&self) -> Option<&Player> {
        self.players
            .iter()
            .filter(|p| p.status != PlaybackStatus::Stopped)
            .max_by_key(|p| (p.status == PlaybackStatus::Playing, p.active_since))
    }

    /// `now` is a monotonic clock reading; only differences between readings matter.
    pub fn view_state(&self, config: &MediaConfig, now: Duration) -> ViewState {
        let Some(player) = self.active() else {
            return ViewState {
                label: "No media".into(),
                icon: Some("zb-media-symbolic".into()),
                visible: !config.hide_when_idle,
                ..ViewState::default()
            };
        };
        let (icon, class) = match player.status {
            PlaybackStatus::Paused => ("zb-media-pause-symbolic", "zb-paused"),
            _ => ("zb-media-play-symbolic", "zb-playing"),
        };
        let full = label(&config.format, &player.track);
        ViewState {
            label: truncate(&full, config.max_length as usize),
            tooltip: Some(full),
            icon: Some(icon.into()),
            classes: vec![class],
            progress: player.progress(now),
            next_wake: player.next_tick(now, self.progress_width.unwrap_or(UNMEASURED_STEPS)),
            ..ViewState::default()
        }
    }
}

/// One piece of a label format.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Token {
    Text(String),
    Field(Field),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Field {
    Title,
    Artist,
    Album,
}

const FIELDS: [(&str, Field); 3] = [
    ("title", Field::Title),
    ("artist", Field::Artist),
    ("album", Field::Album),
];

/// Split a label format into text and placeholders. `{{` and `}}` are literal braces.
pub(crate) fn parse_format(format: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut text = String::new();
    let mut chars = format.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                text.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                text.push('}');
            }
            '{' => {
                let mut name = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some(c) => name.push(c),
                        None => {
                            return Err("unclosed `{` in media format, \
                                        write `{{` for a literal brace"
                                .into());
                        }
                    }
                }
                let Some(&(_, field)) = FIELDS.iter().find(|(n, _)| *n == name) else {
                    return Err(format!(
                        "unknown placeholder `{{{name}}}` in media format, \
                         expected `{{title}}`, `{{artist}}` or `{{album}}`"
                    ));
                };
                if !text.is_empty() {
                    tokens.push(Token::Text(std::mem::take(&mut text)));
                }
                tokens.push(Token::Field(field));
            }
            '}' => {
                return Err("unmatched `}` in media format, write `}}` for a literal brace".into());
            }
            c => text.push(c),
        }
    }
    if !text.is_empty() {
        tokens.push(Token::Text(text));
    }
    Ok(tokens)
}

/// Render the format for a track. Text between two shown fields is the text right before
/// the later one; text before the first or after the last field stays only if that field
/// is shown. So separators and brackets disappear together with an empty field.
fn label(format: &str, track: &Track) -> String {
    // The config resolver rejects invalid formats; this is only a last resort.
    let tokens = parse_format(format).unwrap_or_else(|_| vec![Token::Field(Field::Title)]);
    let value = |field| match field {
        Field::Title if track.title.is_empty() => "Unknown title".to_string(),
        Field::Title => track.title.clone(),
        Field::Artist => track.artists.join(", "),
        Field::Album => track.album.clone(),
    };
    let fields: Vec<usize> = (0..tokens.len())
        .filter(|&i| matches!(tokens[i], Token::Field(_)))
        .collect();
    let shown: Vec<(usize, String)> = fields
        .iter()
        .filter_map(|&i| match tokens[i] {
            Token::Field(field) => Some((i, value(field))).filter(|(_, v)| !v.is_empty()),
            Token::Text(_) => None,
        })
        .collect();
    let text_at = |i: Option<usize>| match i.and_then(|i| tokens.get(i)) {
        Some(Token::Text(text)) => text.as_str(),
        _ => "",
    };

    let mut out = String::new();
    for (n, (i, value)) in shown.iter().enumerate() {
        let before = text_at(i.checked_sub(1));
        if n > 0 || fields.first() == Some(i) {
            out.push_str(before);
        }
        out.push_str(value);
    }
    if let Some((last, _)) = shown.last()
        && fields.last() == Some(last)
    {
        out.push_str(text_at(Some(last + 1)));
    }
    out
}

fn text(value: Option<&Value>) -> String {
    match value {
        Some(Value::Str(s)) => s.trim().to_string(),
        _ => String::new(),
    }
}

/// A duration in microseconds, as MPRIS sends `Position` and `mpris:length`.
fn micros(value: Option<&Value>) -> Option<Duration> {
    match value? {
        Value::I64(us) => Some(Duration::from_micros((*us).max(0) as u64)),
        Value::U64(us) => Some(Duration::from_micros(*us)),
        Value::F64(us) if us.is_finite() => Some(Duration::from_micros(us.max(0.0) as u64)),
        _ => None,
    }
}

fn number(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::F64(n) if n.is_finite() => Some(*n),
        Value::I64(n) => Some(*n as f64),
        Value::U64(n) => Some(*n as f64),
        _ => None,
    }
}

fn list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::StrList(items)) => items
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Some(Value::Str(s)) if !s.trim().is_empty() => vec![s.trim().to_string()],
        _ => Vec::new(),
    }
}
