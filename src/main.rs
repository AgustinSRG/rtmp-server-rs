// Main

mod amf;
mod callback;
mod control;
mod log;
mod redis;
mod rtmp;
mod server;
mod session;
mod utils;

use std::sync::Arc;

use control::{
    spawn_task_control_client, spawn_task_handle_control_key_validations, ControlClientStatus,
    ControlKeyValidationRequest, ControlServerConnectionConfig, KEY_VALIDATION_CHANNEL_BUFFER_SIZE,
};
use log::{LogConfig, Logger};
use redis::{spawn_task_redis_client, RedisConfiguration};
use server::{run_server, RtmpServerConfiguration, RtmpServerStatus};
use tokio::sync::{mpsc::Sender, Mutex};
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

    // Initialize server status

    let server_status = Arc::new(Mutex::new(RtmpServerStatus::new()));

    // Print version

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    logger.log_info(&format!("RTMP Server (Rust Implementation) ({VERSION})"));

    // Load configuration

    let server_config = match RtmpServerConfiguration::load_from_env(&logger) {
        Ok(c) => Arc::new(c),
        Err(_) => {
            std::process::exit(1);
        }
    };

    // Load and run control client

    let control_client_enabled = get_env_bool("CONTROL_USE", false);
    let control_key_validator_sender: Option<Sender<ControlKeyValidationRequest>>;

    if control_client_enabled {
        // Load config

        let control_config = match ControlServerConnectionConfig::load_from_env(&logger) {
            Ok(c) => Arc::new(c),
            Err(_) => {
                std::process::exit(1);
            }
        };

        // Initialize status

        let control_client_status = Arc::new(Mutex::new(ControlClientStatus::new()));

        // Create key validation channel

        let (kv_sender, kv_receiver) = tokio::sync::mpsc::channel::<ControlKeyValidationRequest>(
            KEY_VALIDATION_CHANNEL_BUFFER_SIZE,
        );

        control_key_validator_sender = Some(kv_sender);

        // Spawn client task

        spawn_task_control_client(
            Arc::new(logger.make_child_logger("[CONTROL/CLIENT] ")),
            control_config.clone(),
            control_client_status.clone(),
            server_config.clone(),
            server_status.clone(),
            control_key_validator_sender.clone(),
        );

        // Spawn task to handle key validations

        spawn_task_handle_control_key_validations(
            Arc::new(logger.make_child_logger("[CONTROL/KEY_VALIDATION] ")),
            control_client_status,
            kv_receiver,
        );
    } else {
        control_key_validator_sender = None;
    }

    // Redis feature

    let use_redis = get_env_bool("REDIS_USE", false);

    if use_redis {
        // Load config

        let redis_config = match RedisConfiguration::load_from_env(&logger) {
            Ok(c) => c,
            Err(_) => {
                std::process::exit(1);
            }
        };

        // Spawn task

        spawn_task_redis_client(
            logger.make_child_logger("[REDIS] "),
            redis_config,
            server_config.clone(),
            server_status.clone(),
            control_key_validator_sender.clone(),
        );
    }

    // Run server

    run_server(
        logger,
        server_config,
        server_status,
        control_key_validator_sender,
    )
    .await;

    // End of main

    Ok(())
}
