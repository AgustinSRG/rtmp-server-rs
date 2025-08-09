// Control client connection logic

use std::{sync::Arc, time::Duration};

use futures_util::StreamExt;
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;
use tungstenite::{client::IntoClientRequest, http::HeaderValue};

use crate::{
    log::Logger,
    server::{kill_publisher, remove_all_publishers, RtmpServerContext},
};

use super::{
    make_control_auth_token, spawn_task_control_client_heartbeat, ControlClientStatus,
    ControlKeyValidationResponse, ControlServerConnectionConfig, ControlServerMessage,
};

/// Timeout for read operations
const READ_TIMEOUT_SECONDS: u64 = 60;

/// Spawns task to communicate with the control server
///
/// # Arguments
///
/// * `logger` - The logger
/// * `config` - The control client configuration
/// * `status` - The control client status
/// * `server_context` - The RTMP server context
pub fn spawn_task_control_client(
    logger: Arc<Logger>,
    config: Arc<ControlServerConnectionConfig>,
    status: Arc<Mutex<ControlClientStatus>>,
    server_context: RtmpServerContext,
) {
    tokio::spawn(async move {
        let external_ip_header: HeaderValue = match config.external_ip.parse::<HeaderValue>() {
            Ok(v) => v,
            Err(e) => {
                logger.log_error(&format!("Error creating external ip header: {}", e));

                return;
            }
        };

        let external_port_header: HeaderValue = match config.external_port.parse::<HeaderValue>() {
            Ok(v) => v,
            Err(e) => {
                logger.log_error(&format!("Error creating external port header: {}", e));

                return;
            }
        };

        let external_ssl_header: HeaderValue = match config.external_ssl {
            true => match "true".parse::<HeaderValue>() {
                Ok(v) => v,
                Err(e) => {
                    logger.log_error(&format!("Error creating external ssl header: {}", e));

                    return;
                }
            },
            false => match "false".parse::<HeaderValue>() {
                Ok(v) => v,
                Err(e) => {
                    logger.log_error(&format!("Error creating external ssl header: {}", e));

                    return;
                }
            },
        };

        loop {
            // Prepare request

            let mut request = match config.connection_url.clone().into_client_request() {
                Ok(r) => r,
                Err(e) => {
                    logger.log_error(&format!("Error creating request: {}", e));

                    return;
                }
            };

            // Auth

            let auth_token_header: HeaderValue =
                match make_control_auth_token(&logger, &config).parse::<HeaderValue>() {
                    Ok(v) => v,
                    Err(e) => {
                        logger.log_error(&format!("Error creating auth header: {}", e));

                        return;
                    }
                };

            request
                .headers_mut()
                .insert("x-control-auth-token", auth_token_header);

            // External info

            if !config.external_ip.is_empty() {
                request
                    .headers_mut()
                    .insert("x-external-ip", external_ip_header.clone());
            }

            if !config.external_port.is_empty() {
                request
                    .headers_mut()
                    .insert("x-custom-port", external_port_header.clone());
            }

            if config.external_ssl {
                request
                    .headers_mut()
                    .insert("x-ssl-use", external_ssl_header.clone());
            }

            let (stream, _) = match connect_async(request).await {
                Ok((s, r)) => (s, r),
                Err(e) => {
                    logger.log_error(&format!("Could not connect to the server: {}", e));

                    // Wait
                    tokio::time::sleep(Duration::from_secs(10)).await;

                    // Reconnect
                    continue;
                }
            };

            // Connected, split the stream so multiple tasks can use it

            logger.log_info(&format!("Connected: {}", &config.connection_url));

            let (write_stream, mut read_stream) = stream.split();

            let write_stream_mu = Arc::new(Mutex::new(write_stream));

            // Set status

            ControlClientStatus::set_connected(&status, write_stream_mu).await;

            // Spawn task for heartbeat messages

            let (cancel_heartbeat_sender, cancel_heartbeat_receiver) =
                tokio::sync::mpsc::channel::<()>(1);

            spawn_task_control_client_heartbeat(
                logger.clone(),
                status.clone(),
                cancel_heartbeat_receiver,
            );

            // Read messages

            let mut read_loop_continue = true;

            while read_loop_continue {
                let msg = match tokio::time::timeout(
                    Duration::from_secs(READ_TIMEOUT_SECONDS),
                    read_stream.next(),
                )
                .await
                {
                    Ok(opt) => match opt {
                        Some(r) => match r {
                            Ok(m) => m,
                            Err(e) => {
                                logger.log_error(&format!("Disconnected from the server: {}", e));

                                read_loop_continue = false;
                                continue;
                            }
                        },
                        None => {
                            read_loop_continue = false;
                            continue;
                        }
                    },
                    Err(_) => {
                        logger.log_error("Connection timed out");

                        // Reconnect
                        continue;
                    }
                };

                match msg {
                    tungstenite::Message::Text(utf8_bytes) => {
                        let msg_parsed = ControlServerMessage::parse(&utf8_bytes);

                        if logger.config.trace_enabled {
                            logger.log_trace(&format!("RECEIVED: {}", msg_parsed.serialize()));
                        }

                        match msg_parsed.msg_type.as_str() {
                            "ERROR" => {
                                logger.log_error(&format!(
                                    "Remote error. Code={} / Details: {}",
                                    msg_parsed.get_parameter("Error-Code").unwrap_or(""),
                                    msg_parsed.get_parameter("Error-Message").unwrap_or("")
                                ));
                            }
                            "PUBLISH-ACCEPT" => {
                                let request_id = match msg_parsed.get_parameter("Request-Id") {
                                    Some(req_id_str) => match str::parse::<u64>(req_id_str) {
                                        Ok(id) => id,
                                        Err(_) => {
                                            logger.log_warning("Received a PUBLISH-ACCEPT message with an invalid Request-Id parameter.");
                                            read_loop_continue = false;
                                            continue;
                                        }
                                    },
                                    None => {
                                        logger.log_error("Received a PUBLISH-ACCEPT message with no Request-Id parameter.");
                                        read_loop_continue = false;
                                        continue;
                                    }
                                };

                                let stream_id = msg_parsed.get_parameter("Stream-Id").unwrap_or("");

                                ControlClientStatus::complete_request(
                                    &status,
                                    request_id,
                                    ControlKeyValidationResponse::Accepted {
                                        stream_id: stream_id.to_string(),
                                    },
                                )
                                .await;
                            }
                            "PUBLISH-DENY" => {
                                let request_id = match msg_parsed.get_parameter("Request-Id") {
                                    Some(req_id_str) => match str::parse::<u64>(req_id_str) {
                                        Ok(id) => id,
                                        Err(_) => {
                                            logger.log_warning("Received a PUBLISH-DENY message with an invalid Request-Id parameter.");
                                            read_loop_continue = false;
                                            continue;
                                        }
                                    },
                                    None => {
                                        logger.log_warning("Received a PUBLISH-DENY message with no Request-Id parameter.");
                                        read_loop_continue = false;
                                        continue;
                                    }
                                };

                                ControlClientStatus::complete_request(
                                    &status,
                                    request_id,
                                    ControlKeyValidationResponse::Rejected,
                                )
                                .await;
                            }
                            "STREAM-KILL" => {
                                let channel =
                                    msg_parsed.get_parameter("Stream-Channel").unwrap_or("");
                                let stream_id = msg_parsed
                                    .get_parameter("Stream-Id")
                                    .filter(|&s| !s.is_empty());

                                kill_publisher(&logger, &server_context, channel, stream_id).await;
                            }
                            "HEARTBEAT" => {}
                            _ => {
                                if logger.config.debug_enabled {
                                    logger.log_debug(&format!(
                                        "Unrecognized message type: {}",
                                        &msg_parsed.msg_type
                                    ));
                                }
                            }
                        }
                    }
                    _ => {
                        logger.log_debug("Unknown message type received from websocket");
                    }
                }
            }

            // Disconnected

            ControlClientStatus::set_disconnected(&status).await;

            // Cancel heartbeat

            _ = cancel_heartbeat_sender.send(()).await;

            // Reject all pending requests

            ControlClientStatus::clear_pending_requests(&status).await;

            // Kill all publishers

            remove_all_publishers(&server_context).await;
        }
    });
}
