// Main

mod log;
mod utils;

use std::error::Error;

use log::{LogConfig, Logger};
use utils::get_env_bool;

/// Main function
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load .env
    let _ = dotenvy::dotenv();

    // Initialize logger

    let logger = Logger::new(LogConfig{
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

    // TODO

    // End of main
    Ok(())
}
