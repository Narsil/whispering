use env_logger::Builder;
use log::{LevelFilter, info};
// use whisper_rs::install_logging_hooks;

pub fn init_logging() {
    // install_logging_hooks();
    Builder::from_default_env()
        .filter_level(LevelFilter::Off)
        .filter_module("whispering", LevelFilter::Debug)
        .format_timestamp_secs()
        .format_module_path(false)
        .init();

    info!("Logging system initialized");
}
