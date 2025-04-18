/// RTMP server configuration
use crate::{
    callback::CallbackConfiguration,
    log::Logger,
    rtmp::{RTMP_CHUNK_SIZE_DEFAULT, RTMP_MAX_CHUNK_SIZE, RTMP_MIN_CHUNK_SIZE},
    utils::{get_env_bool, get_env_string, get_env_u32, IpRangeConfig, DEFAULT_MAX_ID_LENGTH},
};

const RTMP_PORT_DEFAULT: u32 = 1935;
const TLS_PORT_DEFAULT: u32 = 443;

const MAX_PORT: u32 = 65535;

const GOP_CACHE_SIZE_MB_DEFAULT: u32 = 256;
const MSG_BUFFER_SIZE_DEFAULT: u32 = 8;

const SSL_CHECK_RELOAD_SECONDS_DEFAULT: u32 = 60;

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
    /// Loads configuration for environment variables
    ///
    /// # Arguments
    ///
    /// * `logger` - The logger
    pub fn load_from_env(logger: &Logger) -> Result<TlsServerConfiguration, ()> {
        let port = get_env_u32("SSL_PORT", TLS_PORT_DEFAULT);

        if port == 0 || port > MAX_PORT {
            logger.log_error(&format!("SSL_PORT has an invalid value: {}", port));
            return Err(());
        }

        let bind_address = get_env_string("SSL_BIND_ADDRESS", &get_env_string("BIND_ADDRESS", "0.0.0.0"));

        let certificate = get_env_string("SSL_CERT", "");
        let key = get_env_string("SSL_KEY", "");

        let check_reload_seconds =
            get_env_u32("SSL_CHECK_RELOAD_SECONDS", SSL_CHECK_RELOAD_SECONDS_DEFAULT);

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
        !self.certificate.is_empty() && !self.key.is_empty()
    }

    /// Gets TLS address for listening
    pub fn get_tcp_listen_addr(&self) -> String {
        format!("{}:{}", self.bind_address, self.port)
    }
}

/// RTMP server configuration
#[derive(Clone)]
pub struct RtmpServerConfiguration {
    /// Port
    pub port: u32,

    /// Bind address
    pub bind_address: String,

    /// TLS config
    pub tls: TlsServerConfiguration,

    /// Max length for Ids and Keys
    pub id_max_length: usize,

    /// Whitelist of IPs to play
    pub play_whitelist: IpRangeConfig,

    /// RTMP chunk size
    pub chunk_size: usize,

    /// Size limit in megabytes of packet cache (bytes).
    pub gop_cache_size: usize,

    /// Size of the message buffer for sessions
    pub msg_buffer_size: usize,

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
    /// Loads configuration for environment variables
    ///
    /// # Arguments
    ///
    /// * `logger` - The logger
    pub fn load_from_env(logger: &Logger) -> Result<RtmpServerConfiguration, ()> {
        let port = get_env_u32("RTMP_PORT", RTMP_PORT_DEFAULT);

        if port == 0 || port > MAX_PORT {
            logger.log_error(&format!("RTMP_PORT has an invalid value: {}", port));
            return Err(());
        }

        let bind_address = get_env_string("BIND_ADDRESS", "0.0.0.0");

        let id_max_length = get_env_u32("ID_MAX_LENGTH", DEFAULT_MAX_ID_LENGTH as u32);

        let play_whitelist =
            match IpRangeConfig::new_from_string(&get_env_string("RTMP_PLAY_WHITELIST", "")) {
                Ok(pw) => pw,
                Err(s) => {
                    logger.log_error(&format!("RTMP_PLAY_WHITELIST has an invalid value: {}", s));
                    return Err(());
                }
            };

        let chunk_size = get_env_u32("RTMP_CHUNK_SIZE", RTMP_CHUNK_SIZE_DEFAULT as u32) as usize;

        if !(RTMP_MIN_CHUNK_SIZE..=RTMP_MAX_CHUNK_SIZE).contains(&chunk_size) {
            logger.log_error(&format!(
                "RTMP_CHUNK_SIZE has an invalid value: {}. Min: {}. Max: {}",
                chunk_size, RTMP_MIN_CHUNK_SIZE, RTMP_MAX_CHUNK_SIZE
            ));
            return Err(());
        }

        let gop_cache_size =
            (get_env_u32("GOP_CACHE_SIZE_MB", GOP_CACHE_SIZE_MB_DEFAULT) as usize) * 1024 * 1024;
        let max_concurrent_connections_per_ip = get_env_u32("MAX_IP_CONCURRENT_CONNECTIONS", 4);
        let msg_buffer_size = get_env_u32("MSG_BUFFER_SIZE", MSG_BUFFER_SIZE_DEFAULT) as usize;

        let max_concurrent_connections_whitelist =
            match IpRangeConfig::new_from_string(&get_env_string("CONCURRENT_LIMIT_WHITELIST", ""))
            {
                Ok(cw) => cw,
                Err(s) => {
                    logger.log_error(&format!(
                        "CONCURRENT_LIMIT_WHITELIST has an invalid value: {}",
                        s
                    ));
                    return Err(());
                }
            };

        let tls = match TlsServerConfiguration::load_from_env(logger) {
            Ok(c) => c,
            Err(()) => {
                return Err(());
            }
        };

        let callback = match CallbackConfiguration::load_from_env(logger) {
            Ok(c) => c,
            Err(()) => {
                return Err(());
            }
        };

        let log_requests = get_env_bool("LOG_REQUESTS", true);

        Ok(RtmpServerConfiguration {
            port,
            bind_address,
            tls,
            id_max_length: id_max_length as usize,
            play_whitelist,
            chunk_size,
            gop_cache_size,
            msg_buffer_size,
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
