// Callback feature configuration

use crate::{
    log::Logger,
    log_warning,
    utils::{get_env_string, get_env_u32},
};

/// Callback configuration
#[derive(Clone)]
pub struct CallbackConfiguration {
    /// Callback URL
    pub callback_url: String,

    /// JWT secret
    pub jwt_secret: String,

    /// Custom JWT subject
    pub jwt_custom_subject: String,

    /// Host to add in the token clams
    pub host: String,

    /// Port to add in the token clams
    pub port: u32,
}

impl CallbackConfiguration {
    /// Loads callback feature configuration
    /// from environment variables
    pub fn load_from_env(logger: &Logger) -> Result<CallbackConfiguration, ()> {
        let callback_url = get_env_string("CALLBACK_URL", "");

        let jwt_secret = get_env_string("JWT_SECRET", "");

        if jwt_secret.is_empty() {
            log_warning!(logger, "JWT_SECRET is empty. Make sure to set a secure JWT secret to prevent security issues.");
        }

        let jwt_custom_subject = get_env_string("CUSTOM_JWT_SUBJECT", "");

        let port = get_env_u32("RTMP_PORT", 1935);
        let host = get_env_string("RTMP_HOST", "");

        Ok(CallbackConfiguration {
            callback_url,
            jwt_secret,
            jwt_custom_subject,
            port,
            host,
        })
    }

    /// Get JWT subject
    pub fn get_jwt_subject(&self) -> &str {
        if self.jwt_custom_subject.is_empty() {
            "rtmp_event"
        } else {
            &self.jwt_custom_subject
        }
    }
}
