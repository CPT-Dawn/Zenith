//! `zenith-bench`: run a Zenith build inside a private headless Sway session
//! and report resident memory, idle CPU, wakeups/s and time to first frame.

use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use zenith_bench::budget::Budget;
use zenith_bench::cli::{Options, USAGE};
use zenith_bench::frame::FirstFrame;
use zenith_bench::measure::{Measurements, measure};
use zenith_bench::procfs;

const EXIT_BUDGET: u8 = 1;
const EXIT_ERROR: u8 = 2;
const EXIT_SKIP: u8 = 77;

/// How long to wait for Sway's socket and for the bar's first frame.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);

const SWAY_CONFIG: &str = "\
# zenith-bench: a bare headless session. No swaybar, no xwayland, one output.
xwayland disable
output HEADLESS-1 resolution 1920x1080 position 0 0 scale 1
default_border none
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let opts = match Options::parse(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("zenith-bench: {e}\n\n{USAGE}");
            return ExitCode::from(EXIT_ERROR);
        }
    };

    let Some(sway) = find_on_path("sway") else {
        if opts.require_compositor {
            eprintln!("zenith-bench: error: `sway` not found on PATH (required by --require-compositor)");
            return ExitCode::from(EXIT_ERROR);
        }
        eprintln!(
            "zenith-bench: skipped: `sway` not found on PATH. The harness needs Sway to host a \
             headless Wayland session; install it (e.g. `paru -S sway`) or rely on the CI benchmark job."
        );
        return ExitCode::from(EXIT_SKIP);
    };

    match run(&opts, &sway) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("zenith-bench: error: {e}");
            ExitCode::from(EXIT_ERROR)
        }
    }
}

fn run(opts: &Options, sway: &Path) -> Result<ExitCode, String> {
    let budget = match &opts.budget {
        Some(p) => Some(Budget::parse(&read(p)?)?),
        None => None,
    };
    let bin = resolve_bin(&opts.bin)?;

    let session = Session::start(sway, &opts.renderer)?;
    let result = session.benchmark(&bin, opts);
    match &result {
        Ok(_) => session.cleanup(),
        Err(_) => eprintln!("zenith-bench: logs kept in {}", session.dir.display()),
    }
    let m = result?;

    println!("zenith-bench: {}", bin.display());
    println!(
        "  settle {} s, window {} s, sway renderer {}, CLK_TCK {}",
        opts.settle.as_secs_f64(),
        opts.window.as_secs_f64(),
        opts.renderer,
        clk_tck()
    );
    println!("  resident memory      {:>8.1} MiB", m.rss_mib);
    println!("  idle CPU             {:>8.3} %", m.idle_cpu_percent);
    println!("  wakeups              {:>8.2} /s", m.wakeups_per_sec);
    match m.first_frame_ms {
        Some(ms) => println!("  time to first frame  {ms:>8.0} ms"),
        None => println!("  time to first frame       n/a (no frame within {} s)", STARTUP_TIMEOUT.as_secs()),
    }

    if let Some(budget) = budget {
        let violations = budget.check(&m);
        if !violations.is_empty() {
            for v in &violations {
                let measured = v.measured.map_or("not measured".to_owned(), |x| format!("{x:.3}"));
                eprintln!("zenith-bench: budget exceeded: {} = {measured} > {}", v.metric, v.limit);
            }
            return Ok(ExitCode::from(EXIT_BUDGET));
        }
        println!("  budget               ok");
    }
    Ok(ExitCode::SUCCESS)
}

/// A private headless Sway plus a private D-Bus session bus, in a temp dir.
struct Session {
    dir: PathBuf,
    runtime: PathBuf,
    wayland_display: String,
    dbus_address: Option<String>,
    sway: Killer,
    dbus: Option<Killer>,
}

impl Session {
    fn start(sway: &Path, renderer: &str) -> Result<Session, String> {
        let dir = std::env::temp_dir().join(format!("zenith-bench-{}", std::process::id()));
        let runtime = dir.join("runtime");
        for sub in ["runtime", "config", "data", "cache", "state"] {
            fs::create_dir_all(dir.join(sub)).map_err(|e| format!("creating {}: {e}", dir.display()))?;
        }
        set_private(&runtime)?;
        let sway_config = dir.join("sway.conf");
        fs::write(&sway_config, SWAY_CONFIG).map_err(|e| e.to_string())?;

        // A private bus so the bar can't hand off to an instance already
        // running on the desktop (GApplication single-instance) and sees no
        // media players.
        let (dbus, dbus_address) = match start_dbus(&dir) {
            Ok((k, a)) => (Some(k), Some(a)),
            Err(e) => {
                eprintln!("zenith-bench: warning: no private D-Bus session ({e}); using the inherited one");
                (None, None)
            }
        };

        let log = fs::File::create(dir.join("sway.log")).map_err(|e| e.to_string())?;
        let mut cmd = Command::new(sway);
        cmd.arg("-c")
            .arg(&sway_config)
            .env("XDG_RUNTIME_DIR", &runtime)
            .env("WLR_BACKENDS", "headless")
            .env("WLR_HEADLESS_OUTPUTS", "1")
            .env("WLR_LIBINPUT_NO_DEVICES", "1")
            .env("WLR_RENDERER", renderer)
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("DISPLAY")
            .env_remove("SWAYSOCK")
            .stdin(Stdio::null())
            .stdout(log.try_clone().map_err(|e| e.to_string())?)
            .stderr(log);
        if let Some(a) = &dbus_address {
            cmd.env("DBUS_SESSION_BUS_ADDRESS", a);
        }
        let mut sway = Killer(cmd.spawn().map_err(|e| {
            let hint = if e.kind() == std::io::ErrorKind::PermissionDenied {
                " (in a container, sway's file capabilities can't be granted; try `setcap -r $(command -v sway)`)"
            } else {
                ""
            };
            format!("starting sway: {e}{hint}")
        })?);

        let deadline = Instant::now() + STARTUP_TIMEOUT;
        let wayland_display = loop {
            if let Some(name) = wayland_socket(&runtime) {
                break name;
            }
            if let Ok(Some(status)) = sway.0.try_wait() {
                return Err(format!(
                    "sway exited ({status}) before creating a socket:\n{}",
                    tail(&dir.join("sway.log"))
                ));
            }
            if Instant::now() > deadline {
                return Err(format!("sway created no Wayland socket:\n{}", tail(&dir.join("sway.log"))));
            }
            thread::sleep(Duration::from_millis(50));
        };

        Ok(Session { dir, runtime, wayland_display, dbus_address, sway, dbus })
    }

    fn bar_command(&self, bin: &Path, opts: &Options) -> Command {
        let mut cmd = Command::new(bin);
        cmd.env("WAYLAND_DISPLAY", &self.wayland_display)
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .env("XDG_CONFIG_HOME", self.dir.join("config"))
            .env("XDG_DATA_HOME", self.dir.join("data"))
            .env("XDG_CACHE_HOME", self.dir.join("cache"))
            .env("XDG_STATE_HOME", self.dir.join("state"))
            .env_remove("DISPLAY")
            .env_remove("HYPRLAND_INSTANCE_SIGNATURE")
            .env_remove("NIRI_SOCKET")
            .stdin(Stdio::null())
            .stdout(Stdio::null());
        if let Some(sock) = swaysock(&self.runtime) {
            cmd.env("SWAYSOCK", sock);
        }
        if let Some(a) = &self.dbus_address {
            cmd.env("DBUS_SESSION_BUS_ADDRESS", a);
        }
        if let Some(c) = &opts.config {
            cmd.env("ZENITH_CONFIG", absolute(c)).env("ZENITH_BAR_CONFIG", absolute(c));
        }
        if let Some(s) = &opts.style {
            cmd.env("ZENITH_STYLE", absolute(s)).env("ZENITH_BAR_STYLE", absolute(s));
        }
        for (k, v) in &opts.env {
            cmd.env(k, v);
        }
        cmd
    }

    fn benchmark(&self, bin: &Path, opts: &Options) -> Result<Measurements, String> {
        let first_frame_ms = self.time_to_first_frame(bin, opts)?;

        // Steady state: a fresh launch without protocol logging, so logging
        // overhead never inflates idle numbers.
        let log = fs::File::create(self.dir.join("bar.log")).map_err(|e| e.to_string())?;
        let mut bar = Killer(
            self.bar_command(bin, opts)
                .stderr(log)
                .spawn()
                .map_err(|e| format!("starting {}: {e}", bin.display()))?,
        );
        let pid = bar.0.id();
        let alive = |bar: &mut Killer| match bar.0.try_wait() {
            Ok(None) => Ok(()),
            Ok(Some(status)) => Err(format!(
                "{} exited ({status}) during the run:\n{}",
                bin.display(),
                tail(&self.dir.join("bar.log"))
            )),
            Err(e) => Err(e.to_string()),
        };

        thread::sleep(opts.settle);
        alive(&mut bar)?;
        let before = procfs::sample(pid).map_err(|e| e.to_string())?;
        let start = Instant::now();
        thread::sleep(opts.window);
        let after = procfs::sample(pid).map_err(|e| e.to_string())?;
        let elapsed = start.elapsed();
        alive(&mut bar)?;

        let mut m = measure(&before, &after, elapsed, clk_tck());
        m.first_frame_ms = first_frame_ms;
        Ok(m)
    }

    /// Launch with `WAYLAND_DEBUG=client` and time spawn → first buffer commit.
    fn time_to_first_frame(&self, bin: &Path, opts: &Options) -> Result<Option<f64>, String> {
        let mut cmd = self.bar_command(bin, opts);
        cmd.env("WAYLAND_DEBUG", "client").stderr(Stdio::piped());
        let start = Instant::now();
        let mut bar = Killer(cmd.spawn().map_err(|e| format!("starting {}: {e}", bin.display()))?);
        let stderr = bar.0.stderr.take().expect("piped stderr");

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut detector = FirstFrame::default();
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            // Keep draining after the frame so the bar never blocks on a full pipe.
            while reader.read_line(&mut line).map(|n| n > 0).unwrap_or(false) {
                if detector.feed(&line) {
                    let _ = tx.send(Instant::now());
                }
                line.clear();
            }
        });

        let result = match rx.recv_timeout(STARTUP_TIMEOUT) {
            Ok(at) => Some(at.duration_since(start).as_secs_f64() * 1000.0),
            Err(_) => {
                if let Ok(Some(status)) = bar.0.try_wait() {
                    return Err(format!("{} exited ({status}) before drawing a frame", bin.display()));
                }
                None
            }
        };
        drop(bar);
        Ok(result)
    }

    fn cleanup(self) {
        drop(self.sway);
        drop(self.dbus);
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Kills (SIGKILL) and reaps its child when dropped, whatever path we leave by.
struct Killer(Child);

impl Drop for Killer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start_dbus(dir: &Path) -> Result<(Killer, String), String> {
    let daemon = find_on_path("dbus-daemon").ok_or("`dbus-daemon` not found")?;
    let mut child = Killer(
        Command::new(daemon)
            .args(["--session", "--nofork", "--nopidfile", "--print-address=1"])
            .current_dir(dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?,
    );
    let mut out = child.0.stdout.take().expect("piped stdout");
    let mut buf = Vec::new();
    let mut byte = [0u8];
    while out.read(&mut byte).map_err(|e| e.to_string())? == 1 && byte[0] != b'\n' {
        buf.push(byte[0]);
    }
    let address = String::from_utf8(buf).map_err(|e| e.to_string())?;
    if address.is_empty() {
        return Err("dbus-daemon printed no address".into());
    }
    Ok((child, address))
}

fn wayland_socket(runtime: &Path) -> Option<String> {
    fs::read_dir(runtime).ok()?.flatten().find_map(|e| {
        let name = e.file_name().into_string().ok()?;
        (name.starts_with("wayland-") && !name.ends_with(".lock")).then_some(name)
    })
}

fn swaysock(runtime: &Path) -> Option<PathBuf> {
    fs::read_dir(runtime).ok()?.flatten().map(|e| e.path()).find(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("sway-ipc.") && n.ends_with(".sock"))
    })
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

fn resolve_bin(bin: &Path) -> Result<PathBuf, String> {
    if bin.components().count() > 1 {
        let p = absolute(bin);
        return if p.is_file() { Ok(p) } else { Err(format!("binary not found: {}", bin.display())) };
    }
    find_on_path(&bin.to_string_lossy())
        .or_else(|| bin.is_file().then(|| absolute(bin)))
        .ok_or_else(|| format!("binary not found: {}", bin.display()))
}

fn absolute(p: &Path) -> PathBuf {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf())
}

fn set_private(dir: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())
}

fn read(p: &Path) -> Result<String, String> {
    fs::read_to_string(p).map_err(|e| format!("reading {}: {e}", p.display()))
}

fn clk_tck() -> u64 {
    Command::new("getconf")
        .arg("CLK_TCK")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(100)
}

fn tail(log: &Path) -> String {
    let text = fs::read_to_string(log).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(20)..].join("\n")
}
