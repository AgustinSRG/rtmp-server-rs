/// RTMP server configuration
use crate::{
    log::Logger,
    utils::{get_env_bool, get_env_string, get_env_u32, IpRangeConfig},
};

/// RTMP server configuration
#[derive(Clone)]
pub struct TlsServerConfiguration {
    /// Port
    pub port: u32,

    /// Bind address
    pub bind_address: String,

    /// Certificate path
    pub certificate: String,

    /// Key path
    pub key: String,

    /// Seconds to check for auto-renewal
    pub check_reload_seconds: u32,
}

impl TlsServerConfiguration {
    pub fn load_from_env(logger: &Logger) -> Result<TlsServerConfiguration, ()> {
        let port = get_env_u32("SSL_PORT", 443);

        if port == 0 || port > 65535 {
            logger.log_error(&format!("SSL_PORT has an invalid value: {}", port));
            return Err(());
        }

        let bind_address = get_env_string("SSL_BIND_ADDRESS", &get_env_string("BIND_ADDRESS", ""));

        let certificate = get_env_string("SSL_CERT", "");
        let key = get_env_string("SSL_KEY", "");

        let check_reload_seconds = get_env_u32("SSL_CHECK_RELOAD_SECONDS", 60);

        Ok(TlsServerConfiguration {
            port,
            bind_address,
            certificate,
            key,
            check_reload_seconds,
        })
    }

    /// Checks if the TLS config is enabled (cert and key must be present)
    pub fn is_enabled(&self) -> bool {
        return !self.certificate.is_empty() && !self.key.is_empty();
    }

    /// Gets TLS address for listening
    pub fn get_tcp_listen_addr(&self) -> String {
        format!("{}:{}", self.bind_address, self.port)
    }
}

/// Callback configuration
#[derive(Clone)]
pub struct CallbackConfiguration {
    /// Callback URL
    pub callback_url: String,

    /// JWT secret
    pub jwt_secret: String,

    /// Custom JWT subject
    pub jwt_custom_subject: String,
}

impl CallbackConfiguration {
    pub fn load_from_env(logger: &Logger) -> Result<CallbackConfiguration, ()> {
        let callback_url = get_env_string("CALLBACK_URL", "");

        let jwt_secret = get_env_string("JWT_SECRET", "");

        if jwt_secret.is_empty() {
            logger.log_warning("JWT_SECRET is empty. Make sure to set a secure JWT secret to prevent security issues.");
        }

        let jwt_custom_subject = get_env_string("CUSTOM_JWT_SUBJECT", "");

        Ok(CallbackConfiguration{
            callback_url,
            jwt_secret,
            jwt_custom_subject,
        })
    }
}

/// RTMP server configuration
#[derive(Clone)]
pub struct RtmpServerConfiguration {
    /// Port
    pub port: u32,

    /// Host
    pub host: String,

    /// Bind address
    pub bind_address: String,

    /// TLS config
    pub tls: TlsServerConfiguration,

    /// Max length for Ids and Keys
    pub id_max_length: usize,

    /// Whitelist of IPs to play
    pub play_whitelist: IpRangeConfig,

    /// RTMP chunk size
    pub chunk_size: u32,

    /// Size limit in megabytes of packet cache (MB).
    pub gop_cache_size_mb: u32,

    /// Max number of concurrent connections per IP address
    pub max_concurrent_connections_per_ip: u32,

    /// List of IP ranges not affected by the max number of concurrent connections limit.
    pub max_concurrent_connections_whitelist: IpRangeConfig,

    /// Callback configuration
    pub callback: CallbackConfiguration,

    /// True to log requests
    pub log_requests: bool,
}

impl RtmpServerConfiguration {
    pub fn load_from_env(logger: &Logger) -> Result<RtmpServerConfiguration, ()> {
        let port = get_env_u32("RTMP_PORT", 1935);

        if port == 0 || port > 65535 {
            logger.log_error(&format!("RTMP_PORT has an invalid value: {}", port));
            return Err(());
        }

        let host = get_env_string("RTMP_HOST", "");
        let bind_address = get_env_string("BIND_ADDRESS", "");

        let id_max_length = get_env_u32("ID_MAX_LENGTH", 128);

        let play_whitelist_res =
            IpRangeConfig::new_from_string(&get_env_string("RTMP_PLAY_WHITELIST", ""));
        let play_whitelist: IpRangeConfig;

        match play_whitelist_res {
            Ok(pw) => {
                play_whitelist = pw;
            }
            Err(s) => {
                logger.log_error(&format!("RTMP_PLAY_WHITELIST has an invalid value: {}", s));
                return Err(());
            }
        }

        let chunk_size = get_env_u32("RTMP_CHUNK_SIZE", 128);
        let gop_cache_size_mb = get_env_u32("GOP_CACHE_SIZE_MB", 256);
        let max_concurrent_connections_per_ip = get_env_u32("MAX_IP_CONCURRENT_CONNECTIONS", 4);

        let max_concurrent_connections_whitelist_res =
            IpRangeConfig::new_from_string(&get_env_string("CONCURRENT_LIMIT_WHITELIST", ""));
        let max_concurrent_connections_whitelist: IpRangeConfig;

        match max_concurrent_connections_whitelist_res {
            Ok(cw) => {
                max_concurrent_connections_whitelist = cw;
            }
            Err(s) => {
                logger.log_error(&format!(
                    "CONCURRENT_LIMIT_WHITELIST has an invalid value: {}",
                    s
                ));
                return Err(());
            }
        }

        let tls_res = TlsServerConfiguration::load_from_env(logger);
        let tls: TlsServerConfiguration;

        match tls_res {
            Ok(c) => {
                tls = c;
            }
            Err(()) => {
                return Err(());
            }
        }

        let callback_res = CallbackConfiguration::load_from_env(logger);
        let callback: CallbackConfiguration;

        match callback_res {
            Ok(c) => {
                callback = c;
            }
            Err(()) => {
                return Err(());
            }
        }

        let log_requests = get_env_bool("LOG_REQUESTS", true);

        Ok(RtmpServerConfiguration {
            port,
            host,
            bind_address,
            tls,
            id_max_length: id_max_length as usize,
            play_whitelist,
            chunk_size,
            gop_cache_size_mb,
            max_concurrent_connections_per_ip,
            max_concurrent_connections_whitelist,
            callback,
            log_requests,
        })
    }

    /// Gets TLS address for listening
    pub fn get_tcp_listen_addr(&self) -> String {
        format!("{}:{}", self.bind_address, self.port)
    }
}
