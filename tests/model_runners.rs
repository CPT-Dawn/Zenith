//! The runners model: GitHub's API replies in, runner slots and the bar's view out.
//! Fixtures follow the shapes of `GET /repos/{repo}/actions/runners` and
//! `GET /repos/{repo}/actions/runs/{id}/jobs`.

use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use zenith_bar::modules::runners::model::{
    self, Fleet, Health, RepoSnapshot, SlotState, parse_jobs, parse_run_ids, parse_runners,
};

fn runner(id: u64, name: &str, status: &str, busy: bool) -> Value {
    json!({
        "id": id,
        "name": name,
        "os": "Linux",
        "status": status,
        "busy": busy,
        "labels": [
            {"id": 1, "name": "self-hosted", "type": "read-only"},
            {"id": 2, "name": "Linux", "type": "read-only"},
            {"id": 3, "name": "X64", "type": "read-only"},
            {"id": 4, "name": "small", "type": "custom"},
        ],
    })
}

fn runners(list: Vec<Value>) -> String {
    json!({"total_count": list.len(), "runners": list}).to_string()
}

fn job(name: &str, status: &str, runner: Option<&str>, started: &str) -> Value {
    json!({
        "id": 1,
        "run_id": 10,
        "name": name,
        "workflow_name": "CI",
        "status": status,
        "conclusion": null,
        "runner_name": runner,
        "started_at": started,
        "html_url": format!("https://github.com/o/r/actions/runs/10/job/{name}"),
    })
}

fn jobs(list: Vec<Value>) -> String {
    json!({"total_count": list.len(), "jobs": list}).to_string()
}

fn snapshot(runner_json: &str, job_json: &[String]) -> RepoSnapshot {
    RepoSnapshot {
        repo: "CPT-Dawn/PeekaBus".into(),
        registrations: parse_runners(runner_json).unwrap(),
        jobs: job_json
            .iter()
            .flat_map(|j| parse_jobs(j).unwrap())
            .collect(),
    }
}

fn states(fleet: &Fleet) -> Vec<(String, &'static str)> {
    fleet.repos[0]
        .slots
        .iter()
        .map(|s| {
            let state = match s.state {
                SlotState::Running { .. } => "running",
                SlotState::Idle => "idle",
                SlotState::Offline => "offline",
            };
            (s.name.clone(), state)
        })
        .collect()
}

/// The 2026-11-16 PeekaBus fleet: seven slots, slot 6 with a dead registration left over.
fn peekabus() -> String {
    runners(vec![
        runner(2987, "peekabus-ci-1-20261116-221235", "online", true),
        runner(2988, "peekabus-ci-2-20261116-221315", "online", false),
        runner(2965, "peekabus-ci-6-20261116-125333", "offline", false),
        runner(2980, "peekabus-ci-3-20261116-213233", "online", false),
        runner(2984, "peekabus-ci-4-20261116-220610", "online", false),
        runner(2985, "peekabus-ci-5-20261116-220629", "online", false),
        runner(2973, "peekabus-ci-6-20261116-180220", "online", true),
        runner(2986, "peekabus-ci-10-20261116-220637", "online", false),
    ])
}

#[test]
fn registrations_of_one_slot_collapse_into_that_slot_and_leftovers_count_as_stale() {
    let fleet = model::fleet(&[snapshot(&peekabus(), &[])], 0);

    assert_eq!(
        states(&fleet),
        vec![
            ("peekabus-ci-1".to_string(), "running"),
            ("peekabus-ci-2".to_string(), "idle"),
            ("peekabus-ci-3".to_string(), "idle"),
            ("peekabus-ci-4".to_string(), "idle"),
            ("peekabus-ci-5".to_string(), "idle"),
            ("peekabus-ci-6".to_string(), "running"),
            ("peekabus-ci-10".to_string(), "idle"),
        ]
    );
    assert_eq!(fleet.repos[0].stale, 1);
    assert_eq!(fleet.health, Health::Ok);
}

#[test]
fn a_runner_without_a_timestamp_suffix_is_its_own_slot() {
    let json = runners(vec![
        runner(1, "build-box", "offline", false),
        runner(2, "build-1", "online", false),
    ]);
    let fleet = model::fleet(&[snapshot(&json, &[])], 0);

    assert_eq!(
        states(&fleet),
        vec![
            ("build-1".to_string(), "idle"),
            ("build-box".to_string(), "offline"),
        ]
    );
    assert_eq!(fleet.repos[0].stale, 0);
}

#[test]
fn a_busy_slot_shows_the_job_running_on_its_registration() {
    let job_json = jobs(vec![
        job(
            "e2e-mobile",
            "in_progress",
            Some("peekabus-ci-1-20261116-221235"),
            "2026-11-16T22:12:47Z",
        ),
        job("open-pr", "completed", None, "2026-11-16T22:05:03Z"),
        job("deploy", "queued", None, "2026-11-16T22:20:00Z"),
        job("check", "waiting", None, "2026-11-16T22:20:00Z"),
    ]);
    let fleet = model::fleet(&[snapshot(&peekabus(), &[job_json])], 0);
    let repo = &fleet.repos[0];

    let SlotState::Running { job } = &repo.slots[0].state else {
        panic!("slot 1 should be running: {:?}", repo.slots[0]);
    };
    let job = job.as_ref().expect("slot 1's job");
    assert_eq!(job.name, "e2e-mobile");
    assert_eq!(job.started.unwrap().to_string(), "2026-11-16T22:12:47Z");
    // Slot 6 is busy, but its job is not in the replies yet.
    assert_eq!(repo.slots[5].state, SlotState::Running { job: None });
    assert_eq!(repo.queued, 2);
}

#[test]
fn expected_slots_that_are_not_registered_count_as_missing() {
    let json = runners(vec![
        runner(1, "ci-1-20261116-221235", "online", false),
        runner(2, "ci-2-20261116-221235", "online", true),
    ]);
    let fleet = model::fleet(&[snapshot(&json, &[])], 4);
    assert_eq!(fleet.missing, 2);

    // Never negative, and 0 turns the check off.
    assert_eq!(model::fleet(&[snapshot(&json, &[])], 1).missing, 0);
    assert_eq!(model::fleet(&[snapshot(&json, &[])], 0).missing, 0);
}

#[test]
fn run_ids_come_from_a_runs_listing() {
    let json = json!({
        "total_count": 2,
        "workflow_runs": [{"id": 19, "status": "in_progress"}, {"id": 23, "status": "in_progress"}],
    })
    .to_string();
    assert_eq!(parse_run_ids(&json).unwrap(), vec![19, 23]);
}

#[test]
fn malformed_replies_are_errors_not_panics() {
    assert!(parse_runners("{").is_err());
    assert!(parse_jobs("[]").is_err());
    assert!(parse_run_ids("null").is_err());
}

// ---------------------------------------------------------------------------
// The bar
// ---------------------------------------------------------------------------

#[test]
fn the_bar_summarises_the_fleet_in_its_tooltip() {
    let job_json = jobs(vec![job("deploy", "queued", None, "2026-11-16T22:20:00Z")]);
    let fleet = model::fleet(&[snapshot(&peekabus(), &[job_json])], 8);
    let view = model::view_state(&fleet);

    assert_eq!(
        view.tooltip.as_deref(),
        Some("2 running · 5 idle · 1 queued · 1 missing · 1 stale")
    );
    assert!(view.classes.contains(&"zb-busy"));
    assert!(view.classes.contains(&"zb-degraded"));
    assert!(view.visible);
}

#[test]
fn a_quiet_fleet_has_no_busy_or_degraded_class() {
    let json = runners(vec![runner(1, "ci-1", "online", false)]);
    let view = model::view_state(&model::fleet(&[snapshot(&json, &[])], 1));

    assert_eq!(view.tooltip.as_deref(), Some("1 idle"));
    assert_eq!(view.classes, Vec::<&str>::new());
}

#[test]
fn trouble_reaching_github_shows_on_the_bar() {
    let fleet = Fleet {
        health: Health::Unauthorized,
        ..Fleet::default()
    };
    let view = model::view_state(&fleet);
    assert_eq!(view.classes, vec!["zb-error"]);
    assert_eq!(
        view.tooltip.as_deref(),
        Some("GitHub refused the token (check token_command)")
    );

    let fleet = Fleet {
        health: Health::Unreachable("connection refused".into()),
        ..Fleet::default()
    };
    assert_eq!(
        model::view_state(&fleet).tooltip.as_deref(),
        Some("Can't reach GitHub: connection refused")
    );

    let fleet = Fleet {
        health: Health::Waiting,
        ..Fleet::default()
    };
    assert_eq!(
        model::view_state(&fleet).tooltip.as_deref(),
        Some("Checking runners…")
    );
}

#[test]
fn elapsed_time_reads_like_a_stopwatch() {
    let at = |s: &str| s.parse::<jiff::Timestamp>().unwrap();
    let start = at("2026-11-16T22:12:47Z");
    assert_eq!(model::elapsed(start, at("2026-11-16T22:12:59Z")), "12s");
    assert_eq!(model::elapsed(start, at("2026-11-16T22:16:59Z")), "4m 12s");
    assert_eq!(model::elapsed(start, at("2026-11-17T00:13:47Z")), "2h 01m");
    // A clock that is behind GitHub's never shows negative time.
    assert_eq!(model::elapsed(start, at("2026-11-16T22:12:00Z")), "0s");
}
