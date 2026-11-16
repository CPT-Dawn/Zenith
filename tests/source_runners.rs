//! The runners source against a fake GitHub: a tiny HTTP/1.1 server on localhost that
//! serves canned replies by path, honours `If-None-Match`, and records every request.
//! The source runs on a glib main loop, as on the bar.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use gtk4::glib;
use pretty_assertions::assert_eq;
use serde_json::json;
use zenith_bar::modules::runners::RunnersConfig;
use zenith_bar::modules::runners::model::{Health, SlotState};
use zenith_bar::modules::runners::source::RunnersSource;

#[derive(Clone, Debug)]
struct Request {
    path: String,
    authorization: Option<String>,
    if_none_match: Option<String>,
}

#[derive(Default)]
struct Book {
    /// path (with query) → (status, body)
    replies: HashMap<String, (u16, String)>,
    requests: Vec<Request>,
}

struct FakeGitHub {
    url: String,
    book: Arc<Mutex<Book>>,
}

impl FakeGitHub {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let book = Arc::new(Mutex::new(Book::default()));
        let shared = book.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let book = shared.clone();
                std::thread::spawn(move || serve(stream, &book));
            }
        });
        Self { url, book }
    }

    fn reply(&self, path: &str, status: u16, body: String) {
        self.book
            .lock()
            .unwrap()
            .replies
            .insert(path.to_string(), (status, body));
    }

    fn requests(&self) -> Vec<Request> {
        self.book.lock().unwrap().requests.clone()
    }
}

/// Keep-alive HTTP/1.1: requests in, canned replies out, ETag = hash of the body.
fn serve(stream: TcpStream, book: &Mutex<Book>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut out = stream;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let path = line.split_whitespace().nth(1).unwrap_or("").to_string();
        let mut headers = HashMap::new();
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).unwrap();
            let header = header.trim_end();
            if header.is_empty() {
                break;
            }
            if let Some((k, v)) = header.split_once(':') {
                headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
            }
        }
        let request = Request {
            path: path.clone(),
            authorization: headers.get("authorization").cloned(),
            if_none_match: headers.get("if-none-match").cloned(),
        };
        let (status, body) = {
            let mut book = book.lock().unwrap();
            book.requests.push(request.clone());
            book.replies
                .get(&path)
                .cloned()
                .unwrap_or((404, "{\"message\":\"Not Found\"}".into()))
        };
        let etag = format!(
            "\"{:x}\"",
            body.len() * 31 + body.bytes().map(usize::from).sum::<usize>()
        );
        let response = if status == 200 && request.if_none_match.as_deref() == Some(&etag) {
            format!("HTTP/1.1 304 Not Modified\r\nETag: {etag}\r\nContent-Length: 0\r\n\r\n")
        } else {
            format!(
                "HTTP/1.1 {status} X\r\nETag: {etag}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
        };
        if out.write_all(response.as_bytes()).is_err() {
            return;
        }
    }
}

const REPO: &str = "/repos/CPT-Dawn/PeekaBus/actions";

fn runners() -> String {
    json!({"total_count": 2, "runners": [
        {"id": 1, "name": "ci-1-20261116-221235", "status": "online", "busy": true, "labels": [{"name": "kvm"}]},
        {"id": 2, "name": "ci-2-20261116-221315", "status": "online", "busy": false, "labels": []},
    ]})
    .to_string()
}

fn fake_peekabus() -> FakeGitHub {
    let github = FakeGitHub::start();
    github.reply(&format!("{REPO}/runners?per_page=100"), 200, runners());
    github.reply(
        &format!("{REPO}/runs?status=in_progress&per_page=20"),
        200,
        json!({"total_count": 1, "workflow_runs": [{"id": 42}]}).to_string(),
    );
    github.reply(
        &format!("{REPO}/runs?status=queued&per_page=20"),
        200,
        json!({"total_count": 0, "workflow_runs": []}).to_string(),
    );
    github.reply(
        &format!("{REPO}/runs/42/jobs?filter=latest&per_page=100"),
        200,
        json!({"total_count": 2, "jobs": [
            {"name": "e2e-mobile", "status": "in_progress", "runner_name": "ci-1-20261116-221235",
             "started_at": "2026-11-16T22:12:47Z", "html_url": "https://github.com/x"},
            {"name": "deploy", "status": "queued", "runner_name": null, "started_at": null, "labels": ["self-hosted"]},
        ]})
        .to_string(),
    );
    github
}

fn config(github: &FakeGitHub, token_command: &str) -> RunnersConfig {
    RunnersConfig {
        repos: vec!["CPT-Dawn/PeekaBus".into()],
        expected: 3,
        interval: 10,
        token_command: token_command.into(),
        api_url: github.url.clone(),
        ..RunnersConfig::default()
    }
}

static MAIN_CONTEXT: Mutex<()> = Mutex::new(());

fn on_main_loop(test: impl FnOnce(&glib::MainContext)) {
    let _turn = MAIN_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    let context = glib::MainContext::default();
    let _owner = context.acquire().expect("own the default main context");
    test(&context);
    while context.iteration(false) {}
}

fn wait_until(context: &glib::MainContext, what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting until {what}");
        if !context.iteration(false) {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

#[test]
fn a_poll_publishes_slots_jobs_and_queue_using_the_commands_token() {
    let github = fake_peekabus();
    on_main_loop(|context| {
        let source = RunnersSource::start(config(&github, "echo s3cret"));
        let fleet = source.fleet();
        wait_until(context, "the first poll lands", || {
            fleet.with(|f| f.health == Health::Ok)
        });

        let fleet = fleet.with(Clone::clone);
        let repo = &fleet.repos[0];
        assert_eq!(repo.repo, "CPT-Dawn/PeekaBus");
        let SlotState::Running { job: Some(job) } = &repo.slots[0].state else {
            panic!("ci-1 runs a job: {:?}", repo.slots[0]);
        };
        assert_eq!(job.name, "e2e-mobile");
        assert_eq!(repo.slots[0].labels, ["kvm"]);
        assert_eq!(repo.slots[1].state, SlotState::Idle);
        assert_eq!(repo.queued, 1);
        assert_eq!(fleet.missing, 1);
        assert_eq!(
            source.state().with(|s| s.tooltip.clone()).as_deref(),
            Some("1 running · 1 idle · 1 queued · 1 missing")
        );
        let requests = github.requests();
        assert!(
            requests
                .iter()
                .all(|r| r.authorization.as_deref() == Some("Bearer s3cret"))
        );
        assert_eq!(
            requests.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
            [
                format!("{REPO}/runners?per_page=100"),
                format!("{REPO}/runs?status=in_progress&per_page=20"),
                format!("{REPO}/runs?status=queued&per_page=20"),
                format!("{REPO}/runs/42/jobs?filter=latest&per_page=100"),
            ]
        );
    });
}

#[test]
fn a_repoll_sends_the_etag_and_reuses_the_cached_reply() {
    let github = fake_peekabus();
    on_main_loop(|context| {
        let source = RunnersSource::start(config(&github, "echo t"));
        let fleet = source.fleet();
        wait_until(context, "the first poll lands", || {
            fleet.with(|f| f.health == Health::Ok)
        });
        let first = fleet.with(Clone::clone);

        // A reconfigure polls again at once; the same token and URL keep the cache.
        let mut again = config(&github, "echo t");
        again.expected = 2;
        source.reconfigure(again);
        wait_until(context, "the second poll lands", || {
            fleet.with(|f| f.missing == 0)
        });

        let second_round: Vec<_> = github.requests().into_iter().skip(4).collect();
        assert_eq!(second_round.len(), 4);
        assert!(second_round.iter().all(|r| r.if_none_match.is_some()));
        assert_eq!(fleet.with(|f| f.repos.clone()), first.repos);
    });
}

#[test]
fn a_failing_token_command_says_so_without_calling_github() {
    let github = fake_peekabus();
    on_main_loop(|context| {
        let source = RunnersSource::start(config(&github, "echo locked >&2; exit 1"));
        let fleet = source.fleet();
        wait_until(context, "the token fails", || {
            fleet.with(|f| matches!(f.health, Health::NoToken(_)))
        });

        assert_eq!(
            fleet.with(|f| f.health.clone()),
            Health::NoToken("`echo locked >&2; exit 1` failed: locked".into())
        );
        assert_eq!(github.requests().len(), 0);
    });
}

#[test]
fn a_refused_token_is_reported_and_fetched_again_next_time() {
    let github = fake_peekabus();
    github.reply(&format!("{REPO}/runners?per_page=100"), 401, "{}".into());
    on_main_loop(|context| {
        let marker = std::env::temp_dir().join(format!("zb-token-{}", std::process::id()));
        let _ = std::fs::remove_file(&marker);
        let command = format!("echo x >> {}; echo t", marker.display());
        let source = RunnersSource::start(config(&github, &command));
        let fleet = source.fleet();
        wait_until(context, "GitHub refuses", || {
            fleet.with(|f| f.health == Health::Unauthorized)
        });
        assert!(source.state().with(|s| s.classes.contains(&"zb-error")));

        // The next poll asks the command again instead of reusing the refused token.
        github.reply(&format!("{REPO}/runners?per_page=100"), 200, runners());
        source.reconfigure(config(&github, &command));
        wait_until(context, "the new token works", || {
            fleet.with(|f| f.health == Health::Ok)
        });
        let runs = std::fs::read_to_string(&marker).unwrap_or_default();
        let _ = std::fs::remove_file(&marker);
        assert_eq!(runs.lines().count(), 2);
    });
}

#[test]
fn every_page_of_runners_is_read_and_busy_repos_still_see_queued_runs() {
    let github = fake_peekabus();
    // 100 dead registrations of slot ci-1 fill page one; its live one is on page two.
    let dead: Vec<_> = (0..100)
        .map(|i| json!({"id": 100 + i, "name": format!("ci-1-20261101-{:06}", i), "status": "offline", "busy": false, "labels": []}))
        .collect();
    github.reply(
        &format!("{REPO}/runners?per_page=100"),
        200,
        json!({"total_count": 101, "runners": dead}).to_string(),
    );
    github.reply(
        &format!("{REPO}/runners?per_page=100&page=2"),
        200,
        json!({"total_count": 101, "runners": [
            {"id": 1, "name": "ci-1-20261116-221235", "status": "online", "busy": false, "labels": []},
        ]})
        .to_string(),
    );
    // Twenty runs in progress don't push the queued run out of the poll.
    let busy: Vec<_> = (0..20).map(|i| json!({"id": 1000 + i})).collect();
    github.reply(
        &format!("{REPO}/runs?status=in_progress&per_page=20"),
        200,
        json!({"total_count": 20, "workflow_runs": busy}).to_string(),
    );
    for i in 0..20 {
        github.reply(
            &format!("{REPO}/runs/{}/jobs?filter=latest&per_page=100", 1000 + i),
            200,
            json!({"total_count": 0, "jobs": []}).to_string(),
        );
    }
    github.reply(
        &format!("{REPO}/runs?status=queued&per_page=20"),
        200,
        json!({"total_count": 1, "workflow_runs": [{"id": 7}]}).to_string(),
    );
    github.reply(
        &format!("{REPO}/runs/7/jobs?filter=latest&per_page=100"),
        200,
        json!({"total_count": 1, "jobs": [
            {"name": "deploy", "status": "queued", "runner_name": null, "started_at": null, "labels": ["self-hosted"]},
        ]})
        .to_string(),
    );
    on_main_loop(|context| {
        let source = RunnersSource::start(config(&github, "echo t"));
        let fleet = source.fleet();
        wait_until(context, "the poll lands", || {
            fleet.with(|f| f.health == Health::Ok)
        });

        let repo = fleet.with(|f| f.repos[0].clone());
        assert_eq!(repo.slots.len(), 1);
        assert_eq!(repo.slots[0].state, SlotState::Idle);
        assert_eq!(repo.stale, 100);
        assert_eq!(repo.queued, 1);
    });
}
