//! The non-GUI commands: `check` and `init`. Each returns the process exit code.

use std::io::ErrorKind;

use crate::cli::Options;
use crate::config;
use crate::paths;

/// Validate the config file and print every diagnostic. 0 when valid, 1 otherwise.
pub fn check(options: &Options) -> u8 {
    let path = paths::config_file(options.config.as_deref());
    let shown = path.display();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("zenith-bar: cannot read {shown}: {e}");
            return 1;
        }
    };

    match config::resolve(&text, None) {
        Ok(_) => {
            println!("{shown} is valid");
            0
        }
        Err(diagnostics) => {
            for diagnostic in &diagnostics {
                eprintln!("{}", diagnostic.render(&shown.to_string()));
            }
            let count = diagnostics.len();
            let noun = if count == 1 { "problem" } else { "problems" };
            eprintln!("{count} {noun} in {shown}");
            1
        }
    }
}

/// Write the starter config. Refuses to replace an existing file unless `force`.
pub fn init(options: &Options, preset: Option<config::Preset>, force: bool) -> u8 {
    let path = paths::config_file(options.config.as_deref());
    let shown = path.display();
    let text = config::starter(preset);

    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty())
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        eprintln!("zenith-bar: cannot create {}: {e}", dir.display());
        return 1;
    }

    let written = if force {
        std::fs::write(&path, text)
    } else {
        // create_new: the existence check and the write are one atomic step.
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, text.as_bytes()))
    };

    match written {
        Ok(()) => {
            println!("wrote {shown}");
            0
        }
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            eprintln!("zenith-bar: {shown} already exists; use --force to overwrite it");
            1
        }
        Err(e) => {
            eprintln!("zenith-bar: cannot write {shown}: {e}");
            1
        }
    }
}
