// Main

mod amf;
mod callback;
mod control;
mod log;
mod rtmp;
mod session;
mod server;
mod utils;

use std::sync::Arc;

use log::{LogConfig, Logger};
use server::{run_server, RtmpServerConfiguration};
use utils::get_env_bool;

/// Main function
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load .env
    let _ = dotenvy::dotenv();

    // Initialize logger

    let logger = Logger::new(LogConfig {
        prefix: "".to_string(),
        error_enabled: get_env_bool("LOG_ERROR", true),
        warning_enabled: get_env_bool("LOG_WARNING", true),
        info_enabled: get_env_bool("LOG_INFO", true),
        debug_enabled: get_env_bool("LOG_DEBUG", false),
        trace_enabled: get_env_bool("LOG_TRACE", get_env_bool("LOG_DEBUG", false)),
    });

    // Print version

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    logger.log_info(&format!("RTMP Server (Rust Implementation) ({VERSION})"));

    // Load configuration

    let server_config_res = RtmpServerConfiguration::load_from_env(&logger);
    let server_config;

    match server_config_res {
        Ok(c) => {
            server_config = c;
        }
        Err(_) => {
            std::process::exit(1);
        }
    }

    // Run server

    run_server(logger, Arc::new(server_config)).await;

    // End of main

    Ok(())
}
