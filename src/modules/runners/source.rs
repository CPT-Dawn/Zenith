//! The one runners source per process: polls GitHub's REST API with libsoup on the main
//! loop (no thread), one poll at a time, every `interval` seconds.
//!
//! Each poll asks, per repo, for the runners, the in-progress and queued runs, and the
//! jobs of those runs. Replies are cached by URL with their `ETag`, so an unchanged
//! answer is a `304` that GitHub doesn't count against the rate limit. The token comes
//! from `token_command`, run once and again after GitHub refuses it.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::time::Duration;

use gtk4::gio;
use gtk4::glib::{self, translate::IntoGlib};
use soup::prelude::*;

use super::RunnersConfig;
use super::model::{self, Fleet, Health, RepoSnapshot};
use crate::modules::{Shared, ViewState, if_alive};

/// At most this many runs of each status (in progress, queued) per repo get their jobs
/// fetched in one poll; each status has its own cap, so busy repos still see queued runs.
const MAX_RUNS: usize = 20;
/// GitHub's largest page.
const PAGE: u64 = 100;
/// Listings longer than this many pages are cut (10,000 runners or jobs of one run).
const MAX_PAGES: u64 = 100;
/// Per-request timeout, in seconds.
const TIMEOUT_S: u32 = 15;

pub struct RunnersSource {
    state: Rc<Shared<ViewState>>,
    fleet: Rc<Shared<Fleet>>,
    session: soup::Session,
    inner: RefCell<Inner>,
}

#[derive(Default)]
struct Inner {
    config: RunnersConfig,
    token: Option<String>,
    /// URL → (ETag, body) of the last good reply.
    cache: HashMap<String, (String, String)>,
    timer: Option<glib::SourceId>,
    /// Bumped by `reconfigure`, so a poll started under an old config drops its result.
    generation: u64,
}

enum Failure {
    Unauthorized,
    /// The source was dropped or reconfigured mid-poll.
    Cancelled,
    Other(String),
}

impl RunnersSource {
    pub fn start(config: RunnersConfig) -> Rc<Self> {
        let session = soup::Session::new();
        session.set_timeout(TIMEOUT_S);
        session.set_user_agent(concat!("zenith-bar/", env!("CARGO_PKG_VERSION")));
        let source = Rc::new(Self {
            state: Shared::new(model::view_state(&Fleet::default())),
            fleet: Shared::new(Fleet::default()),
            session,
            inner: RefCell::new(Inner {
                config,
                ..Inner::default()
            }),
        });
        source.poll_now();
        source
    }

    pub fn state(&self) -> Rc<Shared<ViewState>> {
        self.state.clone()
    }

    /// Slots, jobs and counts, for the view's dots and popover.
    pub fn fleet(&self) -> Rc<Shared<Fleet>> {
        self.fleet.clone()
    }

    pub fn reconfigure(self: &Rc<Self>, config: RunnersConfig) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.config.token_command != config.token_command
                || inner.config.api_url != config.api_url
            {
                inner.token = None;
                inner.cache.clear();
            }
            inner.config = config;
        }
        self.poll_now();
    }

    /// The web page of a repo's Actions tab.
    pub fn actions_url(&self, repo: &str) -> String {
        let api = self.inner.borrow().config.api_url.clone();
        let web = match api.trim_end_matches('/') {
            "https://api.github.com" => "https://github.com".to_string(),
            // GitHub Enterprise Server: https://HOST/api/v3
            other => other.trim_end_matches("/api/v3").to_string(),
        };
        format!("{web}/{repo}/actions")
    }

    /// Drop the pending timer and any poll in flight, and start a fresh poll.
    fn poll_now(self: &Rc<Self>) {
        let generation = {
            let mut inner = self.inner.borrow_mut();
            if let Some(timer) = inner.timer.take() {
                timer.remove();
            }
            inner.generation += 1;
            inner.generation
        };
        let weak = Rc::downgrade(self);
        glib::MainContext::default().spawn_local(poll(weak, generation));
    }

    fn publish(&self, fleet: Fleet) {
        self.state.set(model::view_state(&fleet));
        self.fleet.set(fleet);
    }

    /// Publish a failed poll: the last slots stay, marked with what went wrong.
    fn publish_failure(&self, health: Health) {
        let mut fleet = self.fleet.with(Fleet::clone);
        fleet.health = health;
        self.publish(fleet);
    }

    fn schedule(self: &Rc<Self>, generation: u64) {
        let mut inner = self.inner.borrow_mut();
        if inner.generation != generation {
            return;
        }
        let weak = Rc::downgrade(self);
        let delay = Duration::from_secs(inner.config.interval.max(10).into());
        inner.timer = Some(glib::timeout_add_local_once(delay, move || {
            if_alive(&weak, |source| {
                source.inner.borrow_mut().timer = None;
                glib::MainContext::default().spawn_local(poll(Rc::downgrade(source), generation));
            });
        }));
    }

    fn current(&self, generation: u64) -> bool {
        self.inner.borrow().generation == generation
    }

    /// A GET with the token and the cached ETag; a `304` answers from the cache.
    async fn get(&self, url: &str, token: &str) -> Result<String, Failure> {
        let message = soup::Message::new("GET", url).map_err(|e| Failure::Other(e.to_string()))?;
        if let Some(headers) = message.request_headers() {
            headers.append("Authorization", &format!("Bearer {token}"));
            headers.append("Accept", "application/vnd.github+json");
            headers.append("X-GitHub-Api-Version", "2022-11-28");
            if let Some((etag, _)) = self.inner.borrow().cache.get(url) {
                headers.append("If-None-Match", etag);
            }
        }
        let bytes = self
            .session
            .send_and_read_future(&message, glib::Priority::DEFAULT)
            .await
            .map_err(|e| Failure::Other(e.message().to_string()))?;
        let status = message.status().into_glib();
        let header = |name| {
            message
                .response_headers()
                .and_then(|h| h.one(name))
                .map(|v| v.to_string())
        };
        match status {
            200 => {
                let body = String::from_utf8_lossy(&bytes).into_owned();
                if let Some(etag) = header("ETag") {
                    self.inner
                        .borrow_mut()
                        .cache
                        .insert(url.to_string(), (etag, body.clone()));
                }
                Ok(body)
            }
            304 => self
                .inner
                .borrow()
                .cache
                .get(url)
                .map(|(_, body)| body.clone())
                .ok_or_else(|| Failure::Other("304 without a cached reply".into())),
            403 if header("x-ratelimit-remaining").as_deref() == Some("0") => {
                Err(Failure::Other("rate limited".into()))
            }
            401 | 403 => Err(Failure::Unauthorized),
            404 => Err(Failure::Other(format!(
                "{} not found (or the token can't see its runners)",
                url_path(url)
            ))),
            other => Err(Failure::Other(format!("HTTP {other}"))),
        }
    }
}

impl Drop for RunnersSource {
    fn drop(&mut self) {
        if let Some(timer) = self.inner.get_mut().timer.take() {
            timer.remove();
        }
    }
}

/// One poll: token, then every repo, then publish and schedule the next.
async fn poll(weak: Weak<RunnersSource>, generation: u64) {
    let outcome = poll_once(&weak, generation).await;
    let Some(source) = weak.upgrade() else { return };
    if !source.current(generation) {
        return;
    }
    match outcome {
        Ok(fleet) => source.publish(fleet),
        Err(health) => {
            if health == Health::Unauthorized {
                source.inner.borrow_mut().token = None;
            }
            log::warn!("runners: {health:?}");
            source.publish_failure(health);
        }
    }
    source.schedule(generation);
}

async fn poll_once(weak: &Weak<RunnersSource>, generation: u64) -> Result<Fleet, Health> {
    let alive = || weak.upgrade().filter(|s| s.current(generation));
    let gone = || Health::Waiting;

    let token = {
        let source = alive().ok_or_else(gone)?;
        let cached = source.inner.borrow().token.clone();
        match cached {
            Some(token) => token,
            None => {
                let command = source.inner.borrow().config.token_command.clone();
                drop(source);
                let token = run_token_command(&command).await.map_err(Health::NoToken)?;
                alive().ok_or_else(gone)?.inner.borrow_mut().token = Some(token.clone());
                token
            }
        }
    };

    let (repos, expected, api) = {
        let source = alive().ok_or_else(gone)?;
        let inner = source.inner.borrow();
        (
            inner.config.repos.clone(),
            inner.config.expected,
            inner.config.api_url.trim_end_matches('/').to_string(),
        )
    };

    let mut used = Vec::new();
    let mut snapshots = Vec::new();
    for repo in repos {
        let base = format!("{api}/repos/{repo}/actions");
        let mut fetch = |path: String| {
            let url = format!("{base}/{path}");
            used.push(url.clone());
            let source = alive();
            let token = token.clone();
            async move {
                let source = source.ok_or(Failure::Cancelled)?;
                source.get(&url, &token).await
            }
        };
        let failed = |failure: Failure| match failure {
            Failure::Unauthorized => Health::Unauthorized,
            Failure::Cancelled => gone(),
            Failure::Other(why) => Health::Unreachable(why),
        };

        let mut registrations = Vec::new();
        for page in pages(&mut fetch, &format!("runners?per_page={PAGE}"))
            .await
            .map_err(failed)?
        {
            registrations.extend(model::parse_runners(&page).map_err(Health::Unreachable)?);
        }
        let mut runs = Vec::new();
        for status in ["in_progress", "queued"] {
            let reply = fetch(format!("runs?status={status}&per_page={MAX_RUNS}"))
                .await
                .map_err(failed)?;
            runs.extend(model::parse_run_ids(&reply).map_err(Health::Unreachable)?);
        }
        let mut jobs = Vec::new();
        for run in runs {
            let path = format!("runs/{run}/jobs?filter=latest&per_page={PAGE}");
            for page in pages(&mut fetch, &path).await.map_err(failed)? {
                jobs.extend(model::parse_jobs(&page).map_err(Health::Unreachable)?);
            }
        }
        snapshots.push(RepoSnapshot {
            repo,
            registrations,
            jobs,
        });
    }

    // Replies no longer asked for (finished runs) leave the cache.
    if let Some(source) = alive() {
        source
            .inner
            .borrow_mut()
            .cache
            .retain(|url, _| used.contains(url));
    }
    Ok(model::fleet(&snapshots, expected))
}

/// Every page of a listing whose first page is `path` (which sets `per_page={PAGE}`).
async fn pages<F, Fut>(fetch: &mut F, path: &str) -> Result<Vec<String>, Failure>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Result<String, Failure>>,
{
    let first = fetch(path.to_string()).await?;
    let count = model::total_count(&first).unwrap_or(0).div_ceil(PAGE);
    let mut bodies = vec![first];
    for page in 2..=count.min(MAX_PAGES) {
        bodies.push(fetch(format!("{path}&page={page}")).await?);
    }
    Ok(bodies)
}

/// Run `command` with `sh -c`; its first line of output is the token.
async fn run_token_command(command: &str) -> Result<String, String> {
    let process = gio::Subprocess::newv(
        &["sh".as_ref(), "-c".as_ref(), command.as_ref()],
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE,
    )
    .map_err(|e| e.message().to_string())?;
    let (stdout, stderr) = process
        .communicate_utf8_future(None)
        .await
        .map_err(|e| e.message().to_string())?;
    let token = stdout
        .as_deref()
        .and_then(|s| s.lines().next())
        .unwrap_or("")
        .trim()
        .to_string();
    if process.is_successful() && !token.is_empty() {
        return Ok(token);
    }
    let why = stderr
        .as_deref()
        .and_then(|s| s.lines().find(|l| !l.trim().is_empty()))
        .unwrap_or("it printed nothing")
        .trim()
        .to_string();
    Err(format!("`{command}` failed: {why}"))
}

/// `https://api.github.com/repos/o/r/actions/runners?x` → `repos/o/r`.
fn url_path(url: &str) -> &str {
    let path = url.split_once("/repos/").map_or(url, |(_, p)| p);
    let end = path.find("/actions").unwrap_or(path.len());
    &path[..end]
}
