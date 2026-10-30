//! Command-line parsing. Pure: argument list in, [`Command`] out.

use std::path::PathBuf;

pub const HELP: &str = "\
zenith-bar: the lightest beautiful status bar for Wayland

Usage: zenith-bar [OPTIONS]

Options:
  -c, --config <FILE>  Config file (default: $ZENITH_BAR_CONFIG or ~/.config/zenith-bar/config.toml)
  -s, --style <FILE>   Extra stylesheet (default: $ZENITH_BAR_STYLE or ~/.config/zenith-bar/style.css)
  -h, --help           Print this help
  -V, --version        Print the version

Environment:
  ZENITH_BAR_LOG       Log level or filter, e.g. info or debug (default: warn)
";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Run(Options),
    Version,
    Help,
}

/// Options shared by every command that reads the config.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub config: Option<PathBuf>,
    pub style: Option<PathBuf>,
}

/// Parse arguments (without the program name).
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Command, String> {
    let mut options = Options::default();
    let mut args = args.into_iter();

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
            other => return Err(format!("unknown argument `{other}` (try --help)")),
        }
    }

    Ok(Command::Run(options))
}
