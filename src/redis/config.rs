// Redis feature configuration

use crate::{
    log::Logger,
    log_error,
    utils::{get_env_bool, get_env_string, get_env_u32},
};

/// Redis configuration
pub struct RedisConfiguration {
    /// Redis host
    pub host: String,

    /// Redis port
    pub port: u32,

    // Password
    pub password: String,

    /// Channel to subscribe to
    pub channel: String,

    /// Use TLS?
    pub tls: bool,
}

impl RedisConfiguration {
    /// Loads redis feature configuration
    /// from environment variables
    pub fn load_from_env(logger: &Logger) -> Result<RedisConfiguration, ()> {
        let host = get_env_string("REDIS_HOST", "127.0.0.1");

        let port = get_env_u32("REDIS_PORT", 6379);

        if port == 0 || port > 65535 {
            log_error!(logger, format!("REDIS_PORT has an invalid value: {}", port));
            return Err(());
        }

        let password = get_env_string("REDIS_PASSWORD", "");
        let channel = get_env_string("REDIS_CHANNEL", "rtmp_commands");

        let tls = get_env_bool("REDIS_TLS", false);

        Ok(RedisConfiguration {
            host,
            port,
            password,
            channel,
            tls,
        })
    }

    /// Gets redis URL based on the parameters
    pub fn get_redis_url(&self) -> String {
        // {redis|rediss}://[<username>][:<password>@]<hostname>[:port][/<db>]

        let proto = if self.tls { "rediss" } else { "redis" };

        let password_part = if self.password.is_empty() {
            ""
        } else {
            &format!(":{}@", self.password)
        };

        format!("{}://{}{}:{}", proto, password_part, self.host, self.port)
    }
}
