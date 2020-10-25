// Avoid spawning an console window for the program.
// This is ignored on other platforms.
// https://msdn.microsoft.com/en-us/library/4cc7ya5b.aspx for more information.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod gui;
mod update;

use ajour_core::error::ClientError;
use ajour_core::fs::CONFIG_DIR;
use ajour_core::Result;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn main() {
    let opts_result = cli::get_opts();

    #[cfg(debug_assertions)]
    let is_debug = true;
    #[cfg(not(debug_assertions))]
    let is_debug = false;

    // If this is a clap error, we map to None since we are going to exit and display
    // an error message anyway and this value won't matter. If it's not an error,
    // the underlying `command` will drive this variable. If a `command` is passed
    // on the command line, Ajour functions as a CLI instead of launching the GUI.
    let is_cli = opts_result
        .as_ref()
        .map(|o| &o.command)
        .unwrap_or(&None)
        .is_some();

    // This function validates whether or not we need to exit and print any message
    // due to arguments passed on the command line. If not, it will return a
    // parsed `Opts` struct. This also handles setting up our windows release build
    // fix that allows us to print to the console when not using the GUI.
    let opts = cli::validate_opts_or_exit(opts_result, is_cli, is_debug);

    setup_logger(is_cli, is_debug).expect("setup logging");

    if let Some(data_dir) = &opts.data_directory {
        let mut config_dir = CONFIG_DIR.lock().unwrap();

        *config_dir = data_dir.clone();
    }

    log_panics::init();

    log::info!("Ajour {} has started.", VERSION);

    match opts.command {
        Some(command) => {
            // Process the command and exit
            if let Err(e) = match command {
                cli::Command::Update => update::update_all_addons(),
            } {
                log_error(&e);
            }
        }
        None => {
            // Start the GUI
            gui::run(opts);
        }
    }
}

/// Log any errors
pub fn log_error(e: &ClientError) {
    log::error!("{}", e);
}

#[allow(clippy::unnecessary_operation)]
fn setup_logger(is_cli: bool, is_debug: bool) -> Result<()> {
    let mut logger = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}][{}] {}",
                chrono::Local::now().format("%H:%M:%S%.3f"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Off)
        .level_for("panic", log::LevelFilter::Error)
        .level_for("ajour", log::LevelFilter::Trace);

    if !is_cli {
        logger = logger.level_for("ajour_core", log::LevelFilter::Trace);
    }

    if is_cli || is_debug {
        logger = logger.chain(std::io::stdout());
    }

    if !is_cli && !is_debug {
        use std::fs::OpenOptions;

        let config_dir = ajour_core::fs::config_dir();

        let log_file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(false)
            .truncate(true)
            .open(config_dir.join("ajour.log"))?;

        logger = logger.chain(log_file);
    };

    logger.apply()?;
    Ok(())
}
