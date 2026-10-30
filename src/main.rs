use std::process::ExitCode;

use zenith_bar::cli::{self, Command};

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::new().filter_or("ZENITH_BAR_LOG", "warn"))
        .init();

    match cli::parse(std::env::args().skip(1)) {
        Ok(Command::Run(options)) => {
            let code = zenith_bar::ui::app::run(options);
            ExitCode::from(code.get() as u8)
        }
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
