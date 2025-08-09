// Configuration

use url::Url;

use crate::{
    log::Logger,
    log_error,
    utils::{get_env_bool, get_env_string},
};

/// Configuration of the connection to the control server
pub struct ControlServerConnectionConfig {
    /// Connection URL
    pub connection_url: String,

    /// Secret to sign auth JWTs
    pub secret: String,

    /// External IP for other components
    pub external_ip: String,

    /// External port for other components
    pub external_port: String,

    /// True if external components must use TLS to connect
    pub external_ssl: bool,
}

impl ControlServerConnectionConfig {
    /// Loads control server feature configuration
    /// from environment variables
    pub fn load_from_env(logger: &Logger) -> Result<ControlServerConnectionConfig, ()> {
        let secret = get_env_string("CONTROL_SECRET", "");
        let base_url = get_env_string("CONTROL_BASE_URL", "");

        let connection_url = if !base_url.is_empty() {
            match Url::parse(&base_url) {
                Ok(u) => match u.join("./ws/control/rtmp") {
                    Ok(cu) => cu.to_string(),
                    Err(_) => {
                        log_error!(
                            logger,
                            &format!("CONTROL_BASE_URL has an invalid value: {}", base_url)
                        );
                        return Err(());
                    }
                },
                Err(_) => {
                    log_error!(
                        logger,
                        &format!("CONTROL_BASE_URL has an invalid value: {}", base_url)
                    );
                    return Err(());
                }
            }
        } else {
            "".to_string()
        };

        let external_ip = get_env_string("EXTERNAL_IP", "");
        let external_port = get_env_string("EXTERNAL_PORT", "");
        let external_ssl = get_env_bool("EXTERNAL_SSL", false);

        Ok(ControlServerConnectionConfig {
            connection_url,
            secret,
            external_ip,
            external_port,
            external_ssl,
        })
    }
}
