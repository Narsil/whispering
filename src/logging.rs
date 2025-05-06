use env_logger::Builder;
use log::{LevelFilter, info};
use whisper_rs::install_logging_hooks;

pub fn init_logging() {
    install_logging_hooks();

    #[cfg(debug_assertions)]
    let default_level = LevelFilter::Debug;
    #[cfg(not(debug_assertions))]
    let default_level = LevelFilter::Info;
    Builder::from_default_env()
        .filter_level(LevelFilter::Off)
        .filter_module("whispering", default_level)
        .format_timestamp_secs()
        .format_module_path(false)
        .init();

    info!("Logging system initialized");
}
