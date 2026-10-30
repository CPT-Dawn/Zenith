//! Command-line options for `zenith-bench`.

use std::path::PathBuf;
use std::time::Duration;

pub const USAGE: &str = "\
Usage: zenith-bench --bin <PATH> [options]

Starts a headless Sway session, launches the given Zenith build, lets it
settle, then reports resident memory, idle CPU, wakeups/s and time to first
frame.

Options:
  --bin <PATH>          Zenith binary to benchmark (v1 `zenith` or v2 `zenith-bar`)
  --config <FILE>       Config file (exported as ZENITH_CONFIG and ZENITH_BAR_CONFIG)
  --preset <NAME>       Run a v2 preset: the config comes from
                        `<bin> init --preset <NAME>`, and --budget reads the
                        file's [NAME] table on top of its shared ceilings
  --style <FILE>        Style file (exported as ZENITH_STYLE and ZENITH_BAR_STYLE)
  --settle <SECS>       Wait this long after launch before measuring [default: 10]
  --window <SECS>       Idle measurement window [default: 30]
  --budget <FILE>       Fail (exit 1) if any `metric = ceiling` line is exceeded
  --env <KEY=VALUE>     Extra environment for the bar; repeatable
  --renderer <NAME>     Sway's renderer (WLR_RENDERER): pixman (software, no GPU
                        needed, what CI uses), gles2 or vulkan [default: pixman]
  --smoke               Also smoke-test hot reload (v2): a style.css edit must
                        redraw, moving the bar to the bottom must re-anchor it,
                        and an invalid config must not kill it
  --require-compositor  Fail instead of skipping when `sway` is not installed
  -h, --help            Show this help

Exit status: 0 ok, 1 budget exceeded or smoke test failed, 2 error, 77 skipped (no sway).
";

/// Parsed command line.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub bin: PathBuf,
    pub config: Option<PathBuf>,
    pub style: Option<PathBuf>,
    /// v2 preset to generate the config from (and budget table to apply).
    pub preset: Option<String>,
    /// Run the hot-reload smoke test after measuring.
    pub smoke: bool,
    pub settle: Duration,
    pub window: Duration,
    pub budget: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub require_compositor: bool,
    /// Sway's `WLR_RENDERER`; decides whether the bar gets GPU buffers.
    pub renderer: String,
}

impl Options {
    /// Parse arguments (without the program name).
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
        let mut bin = None;
        let mut o = Options {
            bin: PathBuf::new(),
            config: None,
            style: None,
            preset: None,
            smoke: false,
            settle: Duration::from_secs(10),
            window: Duration::from_secs(30),
            budget: None,
            env: Vec::new(),
            require_compositor: false,
            renderer: "pixman".into(),
        };
        let mut args = args.into_iter();
        while let Some(flag) = args.next() {
            let mut value = || args.next().ok_or_else(|| format!("{flag} needs a value"));
            match flag.as_str() {
                "--bin" => bin = Some(PathBuf::from(value()?)),
                "--config" => o.config = Some(PathBuf::from(value()?)),
                "--style" => o.style = Some(PathBuf::from(value()?)),
                "--preset" => o.preset = Some(value()?),
                "--smoke" => o.smoke = true,
                "--budget" => o.budget = Some(PathBuf::from(value()?)),
                "--settle" => o.settle = seconds(&flag, &value()?)?,
                "--window" => o.window = seconds(&flag, &value()?)?,
                "--env" => {
                    let kv = value()?;
                    let (k, v) = kv.split_once('=').ok_or_else(|| format!("--env expects KEY=VALUE, got `{kv}`"))?;
                    o.env.push((k.to_owned(), v.to_owned()));
                }
                "--renderer" => o.renderer = value()?,
                "--require-compositor" => o.require_compositor = true,
                other => return Err(format!("unknown option `{other}`")),
            }
        }
        o.bin = bin.ok_or("missing required --bin <PATH>")?;
        if o.preset.is_some() && o.config.is_some() {
            return Err("--preset and --config are mutually exclusive (a preset is a config)".into());
        }
        Ok(o)
    }
}

fn seconds(flag: &str, v: &str) -> Result<Duration, String> {
    match v.parse::<f64>() {
        Ok(s) if s > 0.0 && s.is_finite() => Ok(Duration::from_secs_f64(s)),
        _ => Err(format!("{flag} expects a positive number of seconds, got `{v}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Options, String> {
        Options::parse(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn only_the_binary_is_required_and_timing_defaults_are_fixed() {
        let o = parse(&["--bin", "target/release/zenith"]).unwrap();
        assert_eq!(o.bin, PathBuf::from("target/release/zenith"));
        assert_eq!(o.settle, Duration::from_secs(10));
        assert_eq!(o.window, Duration::from_secs(30));
        assert_eq!(o.config, None);
        assert_eq!(o.style, None);
        assert_eq!(o.budget, None);
        assert!(o.env.is_empty());
        assert!(!o.require_compositor);
        assert_eq!(o.renderer, "pixman");
    }

    #[test]
    fn every_option_is_accepted() {
        let o = parse(&[
            "--bin", "zenith-bar",
            "--config", "presets/minimal.toml",
            "--style", "my.css",
            "--settle", "5",
            "--window", "60",
            "--budget", "bench/budgets/minimal.toml",
            "--env", "GSK_RENDERER=cairo",
            "--env", "RUST_LOG=warn",
            "--require-compositor",
            "--renderer", "gles2",
        ])
        .unwrap();
        assert_eq!(o.bin, PathBuf::from("zenith-bar"));
        assert_eq!(o.config, Some(PathBuf::from("presets/minimal.toml")));
        assert_eq!(o.style, Some(PathBuf::from("my.css")));
        assert_eq!(o.settle, Duration::from_secs(5));
        assert_eq!(o.window, Duration::from_secs(60));
        assert_eq!(o.budget, Some(PathBuf::from("bench/budgets/minimal.toml")));
        assert_eq!(
            o.env,
            vec![("GSK_RENDERER".into(), "cairo".into()), ("RUST_LOG".into(), "warn".into())]
        );
        assert!(o.require_compositor);
        assert_eq!(o.renderer, "gles2");
    }

    #[test]
    fn preset_runs_and_smoke_tests_are_opt_in() {
        let o = parse(&["--bin", "zenith-bar"]).unwrap();
        assert_eq!(o.preset, None);
        assert!(!o.smoke);

        let o = parse(&["--bin", "zenith-bar", "--preset", "minimal", "--smoke"]).unwrap();
        assert_eq!(o.preset.as_deref(), Some("minimal"));
        assert!(o.smoke);
    }

    #[test]
    fn preset_and_config_are_mutually_exclusive() {
        let err = parse(&["--bin", "z", "--preset", "bloom", "--config", "c.toml"]).unwrap_err();
        assert!(err.contains("--preset") && err.contains("--config"), "{err}");
    }

    #[test]
    fn missing_binary_and_unknown_flags_are_errors() {
        assert!(parse(&[]).unwrap_err().contains("--bin"));
        assert!(parse(&["--bin", "z", "--windw", "3"]).unwrap_err().contains("--windw"));
        assert!(parse(&["--bin", "z", "--env", "NOEQUALS"]).unwrap_err().contains("KEY=VALUE"));
    }
}
