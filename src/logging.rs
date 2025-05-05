use env_logger::Builder;
use log::{LevelFilter, info};

pub fn init_logging() {
    Builder::from_default_env()
        .filter_level(LevelFilter::Off)
        .filter_module("whispering", LevelFilter::Info)
        .format_timestamp_secs()
        .format_module_path(false)
        .init();

    info!("Logging system initialized");
}
