//! Pure cockpit model: feed documents in, glance chips and countdowns out.
//!
//! Feed contract v1, what a feed command prints:
//!
//! ```text
//! { "version": 1, "generated": RFC 3339,
//!   "glance":   [{ id, label, text, tone, tooltip? }],
//!   "sections": [{ id, title, note?, error?, stale_since?, rows: [
//!       { kind: "meter", label, value: 0..1, text, tone, until? }
//!     | { kind: "text",  label, text, tone?, url? } ] }] }
//! ```
//!
//! Tones are `ok`, `busy`, `warn`, `critical` and `muted`. Unknown fields and row
//! kinds are ignored and an unknown tone reads as `muted`, so the format can grow.

use jiff::Timestamp;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// The one feed version this build reads.
pub const VERSION: u64 = 1;

/// At most this many chips on the bar; a failed feed's `!` chip is never cut.
pub const MAX_CHIPS: usize = 6;

/// How a value should look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Ok,
    Busy,
    Warn,
    Critical,
    Muted,
}

impl Tone {
    /// Unknown names read as `Muted`: a newer feed must not break an older bar.
    pub fn parse(name: &str) -> Tone {
        match name {
            "ok" => Tone::Ok,
            "busy" => Tone::Busy,
            "warn" => Tone::Warn,
            "critical" => Tone::Critical,
            _ => Tone::Muted,
        }
    }

    /// Its CSS state class.
    pub fn css_class(self) -> &'static str {
        match self {
            Tone::Ok => "zb-tone-ok",
            Tone::Busy => "zb-tone-busy",
            Tone::Warn => "zb-tone-warn",
            Tone::Critical => "zb-tone-critical",
            Tone::Muted => "zb-tone-muted",
        }
    }
}

impl<'de> Deserialize<'de> for Tone {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Tone::parse(&String::deserialize(d)?))
    }
}

/// A parsed feed document. `generated` is checked but not kept, so two runs that
/// say the same thing compare equal and nothing redraws.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Feed {
    pub glance: Vec<GlanceItem>,
    pub sections: Vec<Section>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct GlanceItem {
    pub id: String,
    pub label: String,
    pub text: String,
    pub tone: Tone,
    #[serde(default)]
    pub tooltip: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub id: String,
    pub title: String,
    pub note: Option<String>,
    /// This section's source failed.
    pub error: Option<String>,
    /// The rows are an older answer, kept since this time after a failure.
    pub stale_since: Option<Timestamp>,
    pub rows: Vec<Row>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    Meter(Meter),
    Text(TextRow),
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Meter {
    pub label: String,
    /// 0.0..=1.0 (clamped).
    pub value: f64,
    pub text: String,
    pub tone: Tone,
    #[serde(default, deserialize_with = "optional_time")]
    pub until: Option<Timestamp>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct TextRow {
    pub label: String,
    pub text: String,
    #[serde(default)]
    pub tone: Option<Tone>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Deserialize)]
struct RawFeed {
    generated: String,
    #[serde(default)]
    glance: Vec<GlanceItem>,
    #[serde(default)]
    sections: Vec<RawSection>,
}

#[derive(Deserialize)]
struct RawSection {
    id: String,
    title: String,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default, deserialize_with = "optional_time")]
    stale_since: Option<Timestamp>,
    #[serde(default)]
    rows: Vec<Value>,
}

fn optional_time<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Timestamp>, D::Error> {
    match Option::<String>::deserialize(d)? {
        None => Ok(None),
        Some(text) => text
            .parse()
            .map(Some)
            .map_err(|_| serde::de::Error::custom(format!("`{text}` is not an RFC 3339 time"))),
    }
}

/// Parse a feed command's output. The error says what is wrong in one line.
pub fn parse(json: &str) -> Result<Feed, String> {
    let value: Value =
        serde_json::from_str(json.trim()).map_err(|e| format!("not a JSON document: {e}"))?;
    match value.get("version") {
        Some(v) if v.as_u64() == Some(VERSION) => {}
        Some(v) => {
            return Err(format!(
                "feed version {v} is not supported (this zenith-bar reads version {VERSION})"
            ));
        }
        None => return Err(format!("no `version` (expected {VERSION})")),
    }
    let raw: RawFeed = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if raw.generated.parse::<Timestamp>().is_err() {
        return Err(format!(
            "`generated` ({}) is not an RFC 3339 time",
            raw.generated
        ));
    }
    let sections = raw
        .sections
        .into_iter()
        .map(|s| {
            let rows = rows(&s.rows).map_err(|e| format!("section `{}`: {e}", s.id))?;
            Ok(Section {
                id: s.id,
                title: s.title,
                note: s.note,
                error: s.error,
                stale_since: s.stale_since,
                rows,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(Feed {
        glance: raw.glance,
        sections,
    })
}

/// The rows of known kinds, in order; others are skipped.
fn rows(values: &[Value]) -> Result<Vec<Row>, String> {
    let mut out = Vec::new();
    for (i, value) in values.iter().enumerate() {
        let failed = |e: serde_json::Error| format!("row {i}: {e}");
        match value.get("kind").and_then(Value::as_str) {
            Some("meter") => {
                let mut meter = Meter::deserialize(value).map_err(failed)?;
                meter.value = meter.value.clamp(0.0, 1.0);
                out.push(Row::Meter(meter));
            }
            Some("text") => out.push(Row::Text(TextRow::deserialize(value).map_err(failed)?)),
            _ => {}
        }
    }
    Ok(out)
}

/// What one feed has produced so far.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FeedState {
    pub command: String,
    /// The last document that parsed; kept through failures.
    pub last: Option<Feed>,
    /// Why the last run failed, if it did.
    pub failure: Option<String>,
}

/// One small labelled chip on the bar.
#[derive(Clone, Debug, PartialEq)]
pub struct Chip {
    pub label: String,
    pub text: String,
    pub tone: Tone,
    pub tooltip: Option<String>,
    /// The muted `!` chip of a failed feed.
    pub failed: bool,
}

/// The bar's chips: each feed's glance items in feed order, then its `!` chip if its
/// last run failed. At most [`MAX_CHIPS`]; glance items give way to failure chips.
pub fn glance(feeds: &[FeedState]) -> Vec<Chip> {
    let failures = feeds.iter().filter(|f| f.failure.is_some()).count();
    let mut room = MAX_CHIPS.saturating_sub(failures);
    let mut chips = Vec::new();
    for feed in feeds {
        for item in feed.last.iter().flat_map(|f| &f.glance) {
            if room == 0 {
                break;
            }
            room -= 1;
            chips.push(Chip {
                label: item.label.clone(),
                text: item.text.clone(),
                tone: item.tone,
                tooltip: item.tooltip.clone(),
                failed: false,
            });
        }
        if let Some(why) = &feed.failure {
            chips.push(failure_chip(&feed.command, why));
        }
    }
    chips
}

fn failure_chip(command: &str, why: &str) -> Chip {
    Chip {
        label: String::new(),
        text: "!".into(),
        tone: Tone::Muted,
        tooltip: Some(format!("Feed `{command}` failed: {why}")),
        failed: true,
    }
}

/// Time left until `until`: `45s`, `12m`, `3h36m`, `2d10h`; `now` once it has passed.
pub fn countdown(until: Timestamp, now: Timestamp) -> String {
    let secs = until.as_second() - now.as_second();
    if secs <= 0 {
        return "now".into();
    }
    let (d, h, m, s) = (secs / 86_400, secs / 3600 % 24, secs / 60 % 60, secs % 60);
    match (d, h, m) {
        (0, 0, 0) => format!("{s}s"),
        (0, 0, _) => format!("{m}m"),
        (0, _, _) => format!("{h}h{m:02}m"),
        _ => format!("{d}d{h}h"),
    }
}

/// How old something from `since` is, in its largest unit: `45s`, `10m`, `3h`, `2d`.
pub fn age(since: Timestamp, now: Timestamp) -> String {
    let secs = (now.as_second() - since.as_second()).max(0);
    match secs {
        0..60 => format!("{secs}s"),
        60..3600 => format!("{}m", secs / 60),
        3600..86_400 => format!("{}h", secs / 3600),
        _ => format!("{}d", secs / 86_400),
    }
}
