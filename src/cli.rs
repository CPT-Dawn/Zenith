//! Command-line parsing. Pure: argument list in, [`Command`] out.

use std::path::PathBuf;

use crate::config::Preset;

pub const HELP: &str = "\
zenith-bar: the lightest beautiful status bar for Wayland

Usage: zenith-bar [COMMAND] [OPTIONS]

Commands:
  (none)               Run the bar
  check                Validate the config and print every problem with its position
  init                 Write a commented starter config

Options:
  -c, --config <FILE>  Config file (default: $ZENITH_BAR_CONFIG or ~/.config/zenith-bar/config.toml)
  -s, --style <FILE>   Extra stylesheet (default: $ZENITH_BAR_STYLE or ~/.config/zenith-bar/style.css)
      --preset <NAME>  init: start from a built-in look: bloom, minimal or classic
      --force          init: overwrite an existing config
  -h, --help           Print this help
  -V, --version        Print the version

Environment:
  ZENITH_BAR_LOG       Log level or filter, e.g. info or debug (default: warn)
";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Run(Options),
    Check(Options),
    Init {
        options: Options,
        /// Preset to write; `None` writes the default starter config.
        preset: Option<Preset>,
        force: bool,
    },
    Version,
    Help,
}

/// Options shared by every command that reads the config.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub config: Option<PathBuf>,
    pub style: Option<PathBuf>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sub {
    Run,
    Check,
    Init,
}

/// Parse arguments (without the program name).
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Command, String> {
    let mut options = Options::default();
    let mut force = false;
    let mut preset = None;
    let mut args = args.into_iter().peekable();

    let sub = match args.peek().map(String::as_str) {
        Some("check") => Sub::Check,
        Some("init") => Sub::Init,
        _ => Sub::Run,
    };
    if sub != Sub::Run {
        args.next();
    }

    while let Some(arg) = args.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => (flag.to_string(), Some(value.into())),
            _ => (arg, None),
        };
        let mut value = |name: &str| {
            inline
                .clone()
                .or_else(|| args.next())
                .map(PathBuf::from)
                .ok_or_else(|| format!("{name} needs a file path"))
        };
        match flag.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "-c" | "--config" => options.config = Some(value("--config")?),
            "-s" | "--style" => options.style = Some(value("--style")?),
            "--force" if sub == Sub::Init => force = true,
            "--preset" if sub == Sub::Init => {
                let name = inline
                    .or_else(|| args.next())
                    .ok_or("--preset needs a name (bloom, minimal or classic)")?;
                preset = Some(name.parse::<Preset>().map_err(|()| {
                    format!("unknown preset `{name}` (choose bloom, minimal or classic)")
                })?);
            }
            "--preset" => return Err("--preset only applies to `init`".into()),
            "--force" => return Err("--force only applies to `init`".into()),
            other => return Err(format!("unknown argument `{other}` (try --help)")),
        }
    }

    Ok(match sub {
        Sub::Run => Command::Run(options),
        Sub::Check => Command::Check(options),
        Sub::Init => Command::Init {
            options,
            preset,
            force,
        },
    })
}
