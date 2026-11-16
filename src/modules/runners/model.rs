//! Pure runners model: GitHub's API replies in, runner slots and the bar's view out.
//!
//! A **runner slot** is one long-lived runner place on a host. Ephemeral runners register
//! anew for every job (`ci-1-20261116-221235`, then `ci-1-20261116-223012`), so the
//! registrations of one slot are grouped by their name without its timestamp suffix.
//! A slot is running, idle or offline; its leftover offline registrations are **stale**.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use jiff::Timestamp;
use serde::Deserialize;

use crate::modules::ViewState;

/// One runner registration, from `GET /repos/{repo}/actions/runners`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Registration {
    pub name: String,
    pub status: String,
    pub busy: bool,
    #[serde(default)]
    pub labels: Vec<Label>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Label {
    pub name: String,
}

/// One job of a workflow run, from `GET /repos/{repo}/actions/runs/{id}/jobs`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ApiJob {
    pub name: String,
    pub status: String,
    pub runner_name: Option<String>,
    pub started_at: Option<String>,
    #[serde(default)]
    pub workflow_name: Option<String>,
    #[serde(default)]
    pub html_url: Option<String>,
    /// The job's `runs-on` labels.
    #[serde(default)]
    pub labels: Vec<String>,
}

/// What one poll learned about one repository.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RepoSnapshot {
    /// `owner/name`
    pub repo: String,
    pub registrations: Vec<Registration>,
    /// Jobs of the repository's active (queued or in-progress) runs.
    pub jobs: Vec<ApiJob>,
}

/// Whether the last poll reached GitHub.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Health {
    /// Nothing heard yet.
    #[default]
    Waiting,
    Ok,
    /// `token_command` failed or printed nothing.
    NoToken(String),
    /// GitHub answered 401 or 403.
    Unauthorized,
    /// No answer, or one that could not be read.
    Unreachable(String),
}

/// Everything the bar and the popover show.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fleet {
    pub health: Health,
    pub repos: Vec<RepoFleet>,
    /// Expected slots that have no registration at all.
    pub missing: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RepoFleet {
    pub repo: String,
    /// In natural name order (`ci-2` before `ci-10`).
    pub slots: Vec<Slot>,
    /// Offline registrations of slots that have a newer one.
    pub stale: usize,
    /// Jobs waiting for a runner.
    pub queued: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    pub name: String,
    pub state: SlotState,
    /// Custom labels only; GitHub's own (`self-hosted`, `Linux`, `X64`) are left out.
    pub labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SlotState {
    /// Busy; `job` is `None` until the job shows up in the run listings.
    Running {
        job: Option<Job>,
    },
    Idle,
    Offline,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    pub name: String,
    pub workflow: Option<String>,
    pub started: Option<Timestamp>,
    pub url: Option<String>,
}

#[derive(Deserialize)]
struct RunnersReply {
    runners: Vec<Registration>,
}

#[derive(Deserialize)]
struct RunsReply {
    workflow_runs: Vec<RunId>,
}

#[derive(Deserialize)]
struct RunId {
    id: u64,
}

#[derive(Deserialize)]
struct JobsReply {
    jobs: Vec<ApiJob>,
}

/// The `total_count` of a paged listing.
pub fn total_count(json: &str) -> Option<u64> {
    #[derive(Deserialize)]
    struct Total {
        total_count: u64,
    }
    serde_json::from_str::<Total>(json)
        .ok()
        .map(|t| t.total_count)
}

pub fn parse_runners(json: &str) -> Result<Vec<Registration>, String> {
    serde_json::from_str::<RunnersReply>(json)
        .map(|r| r.runners)
        .map_err(|e| e.to_string())
}

pub fn parse_run_ids(json: &str) -> Result<Vec<u64>, String> {
    serde_json::from_str::<RunsReply>(json)
        .map(|r| r.workflow_runs.into_iter().map(|r| r.id).collect())
        .map_err(|e| e.to_string())
}

pub fn parse_jobs(json: &str) -> Result<Vec<ApiJob>, String> {
    serde_json::from_str::<JobsReply>(json)
        .map(|r| r.jobs)
        .map_err(|e| e.to_string())
}

/// Labels GitHub gives every self-hosted runner; they say nothing about a slot.
const DEFAULT_LABELS: [&str; 6] = ["self-hosted", "Linux", "X64", "ARM64", "macOS", "Windows"];

/// Job states that mean "waiting for a runner".
const WAITING: [&str; 4] = ["queued", "waiting", "pending", "requested"];

/// Build the fleet from a successful poll. `expected` is the number of slots the user
/// runs (0: don't track missing ones).
pub fn fleet(repos: &[RepoSnapshot], expected: u32) -> Fleet {
    let repos: Vec<RepoFleet> = repos.iter().map(repo_fleet).collect();
    let present: usize = repos.iter().map(|r| r.slots.len()).sum();
    Fleet {
        health: Health::Ok,
        missing: (expected as usize).saturating_sub(present),
        repos,
    }
}

fn repo_fleet(snapshot: &RepoSnapshot) -> RepoFleet {
    let mut groups: BTreeMap<&str, Vec<&Registration>> = BTreeMap::new();
    for registration in &snapshot.registrations {
        groups
            .entry(slot_name(&registration.name))
            .or_default()
            .push(registration);
    }

    let mut stale = 0;
    let mut slots: Vec<Slot> = groups
        .into_iter()
        .map(|(name, mut registrations)| {
            registrations.sort_by_key(|r| rank(r));
            let primary = registrations[0];
            stale += registrations[1..]
                .iter()
                .filter(|r| r.status != "online")
                .count();
            Slot {
                name: name.to_string(),
                state: state(primary, &snapshot.jobs),
                labels: primary
                    .labels
                    .iter()
                    .map(|l| l.name.clone())
                    .filter(|l| !DEFAULT_LABELS.contains(&l.as_str()))
                    .collect(),
            }
        })
        .collect();
    slots.sort_by(|a, b| natural(&a.name, &b.name));

    RepoFleet {
        repo: snapshot.repo.clone(),
        slots,
        stale,
        queued: snapshot
            .jobs
            .iter()
            .filter(|j| WAITING.contains(&j.status.as_str()) && j.wants_self_hosted())
            .count(),
    }
}

impl ApiJob {
    /// Jobs for GitHub-hosted runners (`ubuntu-latest`, …) never wait for a slot.
    fn wants_self_hosted(&self) -> bool {
        self.labels
            .iter()
            .any(|l| l.eq_ignore_ascii_case("self-hosted"))
    }
}

/// Which registration speaks for its slot: busy, then online, then the newest name.
fn rank(r: &Registration) -> (u8, std::cmp::Reverse<String>) {
    let rank = match (r.status == "online", r.busy) {
        (true, true) => 0,
        (true, false) => 1,
        _ => 2,
    };
    (rank, std::cmp::Reverse(r.name.clone()))
}

fn state(primary: &Registration, jobs: &[ApiJob]) -> SlotState {
    match (primary.status == "online", primary.busy) {
        (true, true) => SlotState::Running {
            job: jobs
                .iter()
                .find(|j| {
                    j.status == "in_progress" && j.runner_name.as_deref() == Some(&primary.name)
                })
                .map(|j| Job {
                    name: j.name.clone(),
                    workflow: j.workflow_name.clone(),
                    started: j.started_at.as_deref().and_then(|s| s.parse().ok()),
                    url: j.html_url.clone(),
                }),
        },
        (true, false) => SlotState::Idle,
        _ => SlotState::Offline,
    }
}

/// A registration's slot: its name without a trailing timestamp, either
/// `-YYYYMMDD-HHMMSS` or `-YYYYMMDDHHMMSS`. So `ci-1-20261116-221235` is slot `ci-1`,
/// while `build-1` and `build-100001` stay themselves.
fn slot_name(name: &str) -> &str {
    let digits = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
    let Some((head, time)) = name.rsplit_once('-') else {
        return name;
    };
    if digits(time, 14) && !head.is_empty() {
        return head;
    }
    match head.rsplit_once('-') {
        Some((slot, date)) if digits(time, 6) && digits(date, 8) && !slot.is_empty() => slot,
        _ => name,
    }
}

/// Compare names with their digit runs as numbers.
fn natural(a: &str, b: &str) -> Ordering {
    fn parts(s: &str) -> Vec<Result<u64, &str>> {
        let mut out = Vec::new();
        let mut rest = s;
        while !rest.is_empty() {
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            let len = if digits > 0 {
                digits
            } else {
                rest.find(|c: char| c.is_ascii_digit())
                    .unwrap_or(rest.len())
            };
            let (part, tail) = rest.split_at(len);
            out.push(if digits > 0 && digits < 20 {
                Ok(part.parse().unwrap_or(u64::MAX))
            } else {
                Err(part)
            });
            rest = tail;
        }
        out
    }
    let order = |a: &Result<u64, &str>, b: &Result<u64, &str>| match (a, b) {
        (Ok(x), Ok(y)) => x.cmp(y),
        (Ok(_), Err(_)) => Ordering::Less,
        (Err(_), Ok(_)) => Ordering::Greater,
        (Err(x), Err(y)) => x.cmp(y),
    };
    let (a, b) = (parts(a), parts(b));
    a.iter()
        .zip(&b)
        .map(|(x, y)| order(x, y))
        .find(|o| o.is_ne())
        .unwrap_or_else(|| a.len().cmp(&b.len()))
}

/// Totals across every repository.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub running: usize,
    pub idle: usize,
    pub offline: usize,
    pub missing: usize,
    pub stale: usize,
    pub queued: usize,
}

impl Fleet {
    pub fn counts(&self) -> Counts {
        let mut counts = Counts {
            missing: self.missing,
            ..Counts::default()
        };
        for repo in &self.repos {
            counts.stale += repo.stale;
            counts.queued += repo.queued;
            for slot in &repo.slots {
                match slot.state {
                    SlotState::Running { .. } => counts.running += 1,
                    SlotState::Idle => counts.idle += 1,
                    SlotState::Offline => counts.offline += 1,
                }
            }
        }
        counts
    }
}

/// The bar's state: classes and a one-line tooltip. The view draws the slots itself.
pub fn view_state(fleet: &Fleet) -> ViewState {
    let (tooltip, classes) = match &fleet.health {
        Health::Ok => {
            let c = fleet.counts();
            let parts: Vec<String> = [
                (c.running, "running"),
                (c.idle, "idle"),
                (c.queued, "queued"),
                (c.offline, "offline"),
                (c.missing, "missing"),
                (c.stale, "stale"),
            ]
            .into_iter()
            .filter(|(n, _)| *n > 0)
            .map(|(n, what)| format!("{n} {what}"))
            .collect();
            let tooltip = if parts.is_empty() {
                "No self-hosted runners".to_string()
            } else {
                parts.join(" · ")
            };
            let mut classes = Vec::new();
            if c.running > 0 {
                classes.push("zb-busy");
            }
            if c.offline + c.missing + c.stale > 0 {
                classes.push("zb-degraded");
            }
            (tooltip, classes)
        }
        Health::Waiting => ("Checking runners…".to_string(), Vec::new()),
        Health::NoToken(why) => (format!("No GitHub token: {why}"), vec!["zb-error"]),
        Health::Unauthorized => (
            "GitHub refused the token (check token_command)".to_string(),
            vec!["zb-error"],
        ),
        Health::Unreachable(why) => (format!("Can't reach GitHub: {why}"), vec!["zb-error"]),
    };
    ViewState {
        tooltip: Some(tooltip),
        classes,
        ..ViewState::default()
    }
}

/// A running job's age: `12s`, `4m 12s`, `2h 01m`.
pub fn elapsed(started: Timestamp, now: Timestamp) -> String {
    let secs = now.as_second().saturating_sub(started.as_second()).max(0);
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    match (h, m) {
        (0, 0) => format!("{s}s"),
        (0, _) => format!("{m}m {s:02}s"),
        _ => format!("{h}h {m:02}m"),
    }
}
