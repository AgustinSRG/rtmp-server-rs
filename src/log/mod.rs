// Log module

mod config;
mod logger;

pub use config::*;
pub use logger::*;

#[macro_export]
macro_rules! log_error {
    // This marco logs an ERROR message, only if the ERROR level is enabled
    // The first argument must be the logger
    // The second argument must be the message to log, as an string
    ($logger:ident, $msg:expr) => {
        if $logger.config.error_enabled {
            $logger.log(&format!("[ERROR] {}", $msg));
        }
    };
}

#[macro_export]
macro_rules! log_warning {
    // This marco logs an WARNING message, only if the WARNING level is enabled
    // The first argument must be the logger
    // The second argument must be the message to log, as an string
    ($logger:ident, $msg:expr) => {
        if $logger.config.warning_enabled {
            $logger.log(&format!("[WARNING] {}", $msg));
        }
    };
}

#[macro_export]
macro_rules! log_info {
    // This marco logs an INFO message, only if the INFO level is enabled
    // The first argument must be the logger
    // The second argument must be the message to log, as an string
    ($logger:ident, $msg:expr) => {
        if $logger.config.info_enabled {
            $logger.log(&format!("[INFO] {}", $msg));
        }
    };
}
