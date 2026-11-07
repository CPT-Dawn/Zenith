//! Pure clipboard model: what the Daemon said in, a [`ViewState`] out.

use std::time::Duration;

use hoarder_ipc::{ClipSummary, Event, Op, Recording, State};
use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::modules::{ViewState, truncate};

/// How many Clips the popover lists: what the source asks `list` for.
pub const RECENT: u32 = 10;

/// Longest Clip label in the popover, in characters.
const LABEL_MAX: usize = 48;

/// What the popover shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Recent {
    /// Whether a Daemon is there to list and Restore Clips.
    pub online: bool,
    /// The most recent Clips, newest first.
    pub rows: Vec<Row>,
}

/// One Clip in the popover.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// The Clip's id, for `restore` and `thumb`.
    pub id: i64,
    pub kind: Kind,
    /// Its preview on one line, cut to fit.
    pub label: String,
}

/// What a Clip holds. Kinds newer than this module are [`Kind::Other`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    /// Has a thumbnail.
    Image,
    Files,
    Other,
}

impl Kind {
    fn parse(kind: &str) -> Self {
        match kind {
            "text" => Kind::Text,
            "image" => Kind::Image,
            "files" => Kind::Files,
            _ => Kind::Other,
        }
    }
}

#[derive(Default)]
pub struct Clipboard {
    /// `None` while there is no Daemon to talk to.
    recording: Option<Recording>,
    clips: Vec<ClipSummary>,
}

impl Clipboard {
    pub fn new() -> Self {
        Self::default()
    }

    /// A Daemon answered: how it records, and its most recent Clips.
    pub fn connected(&mut self, recording: Recording, mut clips: Vec<ClipSummary>) {
        clips.truncate(RECENT as usize);
        self.recording = Some(recording);
        self.clips = clips;
    }

    /// The Daemon went away (or never came).
    pub fn disconnected(&mut self) {
        *self = Self::default();
    }

    /// Something the Daemon pushed. Selection changes, and events newer than this
    /// module, change nothing here.
    pub fn apply(&mut self, event: Event) {
        let Some(recording) = &mut self.recording else {
            return;
        };
        match event {
            Event::State(new) => *recording = new,
            Event::ClipAdded { clip } => {
                self.clips.retain(|c| c.id != clip.id);
                self.clips.insert(0, clip);
                self.clips.truncate(RECENT as usize);
            }
            Event::ClipRemoved { clip } => self.clips.retain(|c| c.id != clip),
            _ => {}
        }
    }

    /// The right-click action: Pause an active Daemon (until resumed), resume a paused
    /// one. `None` while offline.
    pub fn toggle_pause(&self) -> Option<Op> {
        Some(match self.recording.as_ref()?.state {
            State::Paused => Op::Resume,
            _ => Op::Pause { secs: None },
        })
    }

    pub fn recent(&self) -> Recent {
        Recent {
            online: self.recording.is_some(),
            rows: self
                .clips
                .iter()
                .map(|clip| Row {
                    id: clip.id,
                    kind: Kind::parse(&clip.kind),
                    label: one_line(&clip.preview),
                })
                .collect(),
        }
    }

    /// `tz` is only for showing when a timed Pause ends.
    pub fn view_state(&self, tz: &TimeZone) -> ViewState {
        let (icon, class, tooltip) = match &self.recording {
            None => (
                "zb-clipboard-offline-symbolic",
                "zb-clipboard-offline",
                "Clipboard history is offline".to_string(),
            ),
            Some(Recording {
                state: State::Paused,
                until,
            }) => (
                "zb-clipboard-paused-symbolic",
                "zb-clipboard-paused",
                format!(
                    "Clipboard history is paused{}\nRight-click to resume",
                    until_label(until.as_deref(), tz)
                ),
            ),
            Some(_) => (
                "zb-clipboard-symbolic",
                "zb-clipboard-active",
                "Clipboard history".to_string(),
            ),
        };
        ViewState {
            icon: Some(icon.into()),
            classes: vec![class],
            tooltip: Some(tooltip),
            ..ViewState::default()
        }
    }
}

/// ` until 14:05` in local time, or nothing for a Pause without an end.
fn until_label(until: Option<&str>, tz: &TimeZone) -> String {
    let Some(at) = until.and_then(|until| until.parse::<Timestamp>().ok()) else {
        return String::new();
    };
    format!(" until {}", at.to_zoned(tz.clone()).strftime("%H:%M"))
}

/// Whitespace runs (newlines and indentation too) as single spaces, cut to fit.
fn one_line(preview: &str) -> String {
    let line = preview.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate(&line, LABEL_MAX)
}

/// How long to wait before trying the Daemon again: doubling from a second up to half
/// a minute, so an absent Daemon costs at most two wakeups a minute.
#[derive(Debug, Default)]
pub struct Backoff {
    failures: u32,
}

impl Backoff {
    const FIRST: Duration = Duration::from_secs(1);
    const MAX: Duration = Duration::from_secs(30);

    /// The delay before the next attempt, after one more failure.
    pub fn after_failure(&mut self) -> Duration {
        let delay = Self::FIRST
            .saturating_mul(1 << self.failures.min(16))
            .min(Self::MAX);
        self.failures = self.failures.saturating_add(1);
        delay
    }

    /// A connection worked: start from the shortest delay again.
    pub fn reset(&mut self) {
        self.failures = 0;
    }
}
