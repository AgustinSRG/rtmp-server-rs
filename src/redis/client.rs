// Redis client

use std::time::Duration;

use redis::{PushKind, Value};

use crate::{
    log::Logger,
    server::{kill_publisher, RtmpServerContext},
};

use super::{RedisConfiguration, RedisRtmpCommand};

/// Spawns a task for the Redis client
///
/// # Arguments
///
/// * `logger` - The logger
/// * `config` - The Redis client configuration
/// * `server_context` - The RTMP server context
pub fn spawn_task_redis_client(
    logger: Logger,
    config: RedisConfiguration,
    server_context: RtmpServerContext,
) {
    tokio::spawn(async move {
        loop {
            // Create client
            let client = match redis::Client::open(config.get_redis_url()) {
                Ok(c) => c,
                Err(e) => {
                    logger.log_error(&format!("Could not create a Redis client: {}", e));
                    return;
                }
            };

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let async_config = redis::AsyncConnectionConfig::new().set_push_sender(tx);

            // Connect
            let mut connection = match client
                .get_multiplexed_async_connection_with_config(&async_config)
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    logger.log_error(&format!("Could not connect to Redis server: {}", e));

                    // Wait
                    tokio::time::sleep(Duration::from_secs(10)).await;

                    continue;
                }
            };

            logger.log_info(&format!("Connected: {}", config.get_redis_url()));

            // Subscribe
            if let Err(e) = connection.subscribe(&config.channel).await {
                logger.log_error(&format!(
                    "Could not subscribe to {}: {}",
                    &config.channel, e
                ));

                // Wait
                tokio::time::sleep(Duration::from_secs(10)).await;

                continue;
            }

            logger.log_info(&format!("Subscribed: {}", &config.channel));

            // Read messages
            let mut continue_reading = true;

            while continue_reading {
                match rx.recv().await {
                    Some(msg) => match msg.kind {
                        PushKind::Message => {
                            if let Some(val) = msg.data.first() {
                                let msg_str = value_to_string(val);

                                if logger.config.trace_enabled {
                                    logger.log_trace(&format!("Received message: {}", &msg_str));
                                }

                                let cmd = RedisRtmpCommand::parse(&msg_str);

                                match cmd {
                                    RedisRtmpCommand::KillSession { channel } => {
                                        kill_publisher(&logger, &server_context, &channel, None)
                                            .await;
                                    }
                                    RedisRtmpCommand::CloseStream { channel, stream_id } => {
                                        kill_publisher(
                                            &logger,
                                            &server_context,
                                            &channel,
                                            Some(&stream_id),
                                        )
                                        .await;
                                    }
                                    RedisRtmpCommand::Unknown => {
                                        if logger.config.debug_enabled {
                                            logger.log_debug(&format!(
                                                "Unrecognized message: {}",
                                                &msg_str
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                        PushKind::Disconnection => {
                            continue_reading = false;
                        }
                        _ => {}
                    },
                    None => {
                        continue_reading = false;
                    }
                }
            }

            logger.log_error("Connection lost");
        }
    });
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::BulkString(items) => match String::from_utf8(items.clone()) {
            Ok(s) => s,
            Err(_) => "".to_string(),
        },
        Value::SimpleString(s) => s.clone(),
        _ => "".to_string(),
    }
}
