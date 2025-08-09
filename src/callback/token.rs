// JWT generation logic

use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

use crate::{log::Logger, log_error};

use super::{CallbackConfiguration, CallbackEvent};

const JWT_EXPIRATION_TIME_SECONDS: i64 = 120;

#[derive(Debug, Serialize, Deserialize)]
struct CallbackJwtClaims {
    /// Issued at (UTC timestamp)
    iat: i64,

    /// Expiration (UTC timestamp)
    exp: i64,

    /// Subject
    sub: String,

    /// Event
    event: String,

    /// Channel
    channel: String,

    /// Key
    key: String,

    /// Client IP
    client_ip: Option<String>,

    /// Stream ID
    stream_id: Option<String>,

    /// RTMP port
    rtmp_port: u32,

    /// RTMP host
    rtmp_host: String,
}

/// Generates JWT for a callback request
/// logger - Logger
/// config - Callback configuration
/// channel - The channel
/// key - Streaming key
/// event - Callback event
pub fn make_callback_jwt(
    logger: &Logger,
    config: &CallbackConfiguration,
    channel: &str,
    key: &str,
    event: &CallbackEvent,
) -> String {
    let now = Utc::now().timestamp();

    let claims = CallbackJwtClaims {
        iat: now,
        exp: now + JWT_EXPIRATION_TIME_SECONDS,
        sub: config.get_jwt_subject().to_string(),
        event: event.get_event(),
        channel: channel.to_string(),
        key: key.to_string(),
        client_ip: event.get_client_ip(),
        stream_id: event.get_stream_id(),
        rtmp_port: config.port,
        rtmp_host: config.host.clone(),
    };

    let header = Header::new(Algorithm::HS256);
    match encode(
        &header,
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    ) {
        Ok(token) => token,
        Err(e) => {
            log_error!(logger, format!("Error encoding JWT: {}", e));
            "".to_string()
        }
    }
}
