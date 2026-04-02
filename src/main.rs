mod config;
mod modules;
mod style;
mod ui;

use anyhow::Result;
use gtk4::prelude::*;
use gtk4::Application;

const APP_ID: &str = "dev.zenith.bar";

fn main() -> Result<()> {
    // Initialise logging (respects RUST_LOG env var).
    let mut logger =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    logger
        .format_timestamp_secs()
        .format_module_path(false)
        .format_target(false)
        .init();

    // Prefer the GTK GL renderer unless the user explicitly chose one.
    // This avoids noisy Vulkan swapchain warnings on some Wayland/GPU setups.
    if std::env::var_os("GSK_RENDERER").is_none() {
        std::env::set_var("GSK_RENDERER", "gl");
        log::info!("GSK_RENDERER not set; defaulting to 'gl'");
    }

    // Load configuration early so we can report errors before GTK spins up.
    let cfg = config::load()?;
    log::debug!("Config: {cfg:#?}");

    let app = Application::builder().application_id(APP_ID).build();

    // Move the config into the activation closure.
    app.connect_activate(move |app| {
        if let Err(e) = ui::build_bar(app, &cfg) {
            log::error!("Failed to build bar: {e:#}");
        }
    });

    // GTK application main loop – passing empty args because we don't need
    // GTK to parse CLI flags.
    let exit_code = app.run_with_args::<String>(&[]);
    std::process::exit(exit_code.into());
}
