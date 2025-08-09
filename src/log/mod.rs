// Log module

mod config;
mod logger;

pub use config::*;
pub use logger::*;

#[macro_export]
macro_rules! log_error {
    // This marco logs an ERROR message, only if the ERROR level is enabled
    // The first argument must be the logger
    // The second argument must be the message to log, as a string
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
    // The second argument must be the message to log, as a string
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
    // The second argument must be the message to log, as a string
    ($logger:ident, $msg:expr) => {
        if $logger.config.info_enabled {
            $logger.log(&format!("[INFO] {}", $msg));
        }
    };
}

#[macro_export]
macro_rules! log_debug {
    // This marco logs an DEBUG message, only if the DEBUG level is enabled
    // The first argument must be the logger
    // The second argument must be the message to log, as a string
    ($logger:ident, $msg:expr) => {
        if $logger.config.debug_enabled {
            $logger.log(&format!("[DEBUG] {}", $msg));
        }
    };
}

#[macro_export]
macro_rules! log_trace {
    // This marco logs an TRACE message, only if the TRACE level is enabled
    // The first argument must be the logger
    // The second argument must be the message to log, as a string
    ($logger:ident, $msg:expr) => {
        if $logger.config.trace_enabled {
            $logger.log(&format!("[TRACE] {}", $msg));
        }
    };
}
