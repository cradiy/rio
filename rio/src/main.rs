
// With the default subsystem, 'console', windows creates an additional console
// window for the program.
// This is silently ignored on non-windows systems.
// See https://msdn.microsoft.com/en-us/library/4cc7ya5b.aspx for more details.
#![windows_subsystem = "windows"]

mod ansi;
mod cli;
mod clipboard;
mod crosswords;
mod event;
mod ime;
mod logger;
mod performer;
mod platform;
mod scheduler;
mod screen;
mod selection;
mod sequencer;
mod utils;
#[cfg(windows)]
mod panic;
use crate::event::EventP;
use crate::sequencer::Sequencer;
use log::{info, LevelFilter, SetLoggerError};
use logger::Logger;
use std::str::FromStr;

#[cfg(windows)]
use windows_sys::Win32::System::Console::{AttachConsole, FreeConsole, ATTACH_PARENT_PROCESS};

pub fn setup_environment_variables(config: &config::Config) {
    #[cfg(unix)]
    let terminfo = if teletypewriter::terminfo_exists("rio") {
        "rio"
    } else {
        "xterm-256color"
    };

    #[cfg(unix)]
    info!("[setup_environment_variables] terminfo: {terminfo}");

    #[cfg(unix)]
    std::env::set_var("TERM", terminfo);
    
    #[cfg(target_os = "windows")] {
        std::env::set_var("TERM", "xterm-256color");
    }
    std::env::set_var("COLORTERM", "truecolor");
    std::env::remove_var("DESKTOP_STARTUP_ID");
    #[cfg(target_os = "macos")]
    {
        platform::macos::set_locale_environment();
        std::env::set_current_dir(dirs::home_dir().unwrap()).unwrap();
    }

    // Set env vars from config.
    for env_config in config.env_vars.iter() {
        let mut env_vec = vec![];
        for config in env_config.split('=') {
            env_vec.push(config);
        }

        if env_vec.len() == 2 {
            std::env::set_var(env_vec[0], env_vec[1]);
        }
    }
}

static LOGGER: Logger = Logger;

fn setup_logs_by_filter_level(log_level: LevelFilter) -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(log_level))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    panic::attach_handler();

    // When linked with the windows subsystem windows won't automatically attach
    // to the console of the parent process, so we do it explicitly. This fails
    // silently if the parent has no console.
    #[cfg(windows)]
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }

    // Load command line options.
    let options = cli::Options::new();
    let command = options.window_options.terminal_options.command;

    let config = config::Config::load();
    let filter_level =
        LevelFilter::from_str(&config.developer.log_level).unwrap_or(LevelFilter::Off);

    let setup_logs = setup_logs_by_filter_level(filter_level);
    if setup_logs.is_err() {
        println!("unable to configure log level");
    }

    setup_environment_variables(&config);

    let window_event_loop =
        winit::event_loop::EventLoopBuilder::<EventP>::with_user_event().build();

    let mut sequencer = Sequencer::new(config);
    let result = sequencer.run(window_event_loop, command);

    result.await
}
