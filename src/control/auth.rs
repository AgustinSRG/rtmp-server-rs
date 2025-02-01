// Logic to generate auth tokens for the control server

use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

use crate::log::Logger;

use super::ControlServerConnectionConfig;

#[derive(Debug, Serialize, Deserialize)]
struct ControlAuthJwtClaims {
    /// Issued at (UTC timestamp)
    iat: i64,

    /// Expiration (UTC timestamp)
    exp: i64,

    /// Subject
    sub: String,
}

const JWT_EXPIRATION_TIME_SECONDS: i64 = 60 * 60;

/// Creates an authentication token valid for 1 hour
/// to connect to the control server
pub fn make_control_auth_token(logger: &Logger, config: &ControlServerConnectionConfig) -> String {
    let now = Utc::now().timestamp();

    let claims = ControlAuthJwtClaims {
        iat: now,
        exp: now + JWT_EXPIRATION_TIME_SECONDS,
        sub: "rtmp-control".to_string(),
    };

    let header = Header::new(Algorithm::HS256);
    match encode(
        &header,
        &claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    ) {
        Ok(token) => token,
        Err(e) => {
            logger.log_error(&format!("Error encoding JWT: {}", e.to_string()));
            "".to_string()
        }
    }
}
