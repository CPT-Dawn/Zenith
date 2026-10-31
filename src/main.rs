use std::process::ExitCode;

use zenith_bar::cli::{self, Command};

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::new().filter_or("ZENITH_BAR_LOG", "warn"))
        .init();

    match cli::parse(std::env::args().skip(1)) {
        Ok(Command::Run(options)) => {
            let current = std::env::var_os("GSK_RENDERER");
            if let Some(renderer) = zenith_bar::ui::renderer::default_for(current.as_deref()) {
                // SAFETY: GTK has not started and no other thread exists yet,
                // so nothing can be reading the environment concurrently.
                unsafe { std::env::set_var("GSK_RENDERER", renderer) };
            }
            let code = zenith_bar::ui::app::run(options);
            ExitCode::from(code.get() as u8)
        }
        Ok(Command::Check(options)) => ExitCode::from(zenith_bar::commands::check(&options)),
        Ok(Command::Init {
            options,
            preset,
            force,
        }) => ExitCode::from(zenith_bar::commands::init(&options, preset, force)),
        Ok(Command::Version) => {
            println!("zenith-bar {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Command::Help) => {
            print!("{}", cli::HELP);
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("zenith-bar: {message}");
            ExitCode::from(2)
        }
    }
}
