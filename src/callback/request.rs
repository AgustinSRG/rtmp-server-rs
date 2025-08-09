// Callback requests

use std::net::IpAddr;

use reqwest::StatusCode;

use crate::{log::Logger, log_debug};

use super::{make_callback_jwt, CallbackConfiguration, CallbackEvent};

/// Makes start event callback
/// logger - The logger
/// config - Callback config
/// channel - The channel
/// key - The streaming key
/// client_ip - The IP of the publisher
/// Returns the stream id, or None if invalid key / error
pub async fn make_start_callback(
    logger: &Logger,
    config: &CallbackConfiguration,
    channel: &str,
    key: &str,
    client_ip: &IpAddr,
) -> Option<String> {
    let callback_url = &config.callback_url;

    if callback_url.is_empty() {
        return Some(key.to_string());
    }

    log_debug!(
        logger,
        format!(
            "POST {} | | Event: START | Channel: {}",
            callback_url, channel
        )
    );

    // Generate token

    let token = make_callback_jwt(
        logger,
        config,
        channel,
        key,
        &CallbackEvent::Start {
            client_ip: *client_ip,
        },
    );

    // Make the request

    let client = reqwest::Client::new();

    let request_builder = client.post(callback_url).header("rtmp-event", token);

    let response = request_builder.send().await;

    // Check the response

    match response {
        Ok(r) => {
            if r.status() != StatusCode::OK {
                log_debug!(
                    logger,
                    format!("Callback resulted in status code: {}", r.status().as_u16())
                );

                return None;
            }

            match r.headers().get("stream-id") {
                Some(s) => match s.to_str() {
                    Ok(stream_id) => Some(stream_id.to_string()),
                    Err(_) => Some("".to_string()),
                },
                None => Some("".to_string()),
            }
        }
        Err(e) => {
            log_debug!(logger, format!("Callback resulted in error: {}", e));

            None
        }
    }
}

/// Makes stop event callback
/// logger - The logger
/// config - Callback config
/// channel - The channel
/// key - The streaming key
/// stream_id - The stream ID given when called the start callback
/// Returns true on success, false on error
pub async fn make_stop_callback(
    logger: &Logger,
    config: &CallbackConfiguration,
    channel: &str,
    key: &str,
    stream_id: &str,
) -> bool {
    let callback_url = &config.callback_url;

    if callback_url.is_empty() {
        return true;
    }

    log_debug!(
        logger,
        format!(
            "POST {} | | Event: STOP | Channel: {} | Stream ID: {}",
            callback_url, channel, stream_id
        )
    );

    // Generate token

    let token = make_callback_jwt(
        logger,
        config,
        channel,
        key,
        &CallbackEvent::Stop {
            stream_id: stream_id.to_string(),
        },
    );

    // Make the request

    let client = reqwest::Client::new();

    let request_builder = client.post(callback_url).header("rtmp-event", token);

    let response = request_builder.send().await;

    // Check the response

    match response {
        Ok(r) => {
            if r.status() != StatusCode::OK {
                log_debug!(
                    logger,
                    format!("Callback resulted in status code: {}", r.status().as_u16())
                );

                return false;
            }

            true
        }
        Err(e) => {
            log_debug!(logger, format!("Callback resulted in error: {}", e));

            false
        }
    }
}
