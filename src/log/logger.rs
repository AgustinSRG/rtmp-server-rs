// Logger

use super::config::LogConfig;
use chrono::{DateTime, Local};

/// Logger
pub struct Logger {
    /// Configuration
    pub config: LogConfig,
}

impl Logger {
    // Creates new logger
    pub fn new(config: LogConfig) -> Logger {
        Logger{
            config,
        }
    }

    /// Makes child logger
    pub fn make_child_logger(&self, prefix: &str) -> Logger {
        Logger{
            config: self.config.child_config(prefix),
        }
    }

    /// Logs a message
    pub fn log(&self, line: &str) {
        let time_local: DateTime<Local> = Local::now();
        let time_format = time_local.format("[%Y-%m-%d %H:%M:%S] ");

        if self.config.trace_enabled {
            eprintln!("{}{}{}", time_format, self.config.prefix, line);
        } else {
            println!("{}{}{}", time_format, self.config.prefix, line);
        }
    }

    /// Logs error message
    pub fn log_error(&self, line: &str) {
        if !self.config.error_enabled {
            return;
        }

        self.log(&format!("[ERROR] {}", line));
    }

    /// Logs warning message
    pub fn log_warning(&self, line: &str) {
        if !self.config.warning_enabled {
            return;
        }

        self.log(&format!("[WARNING] {}", line));
    }

    /// Logs info message
    pub fn log_info(&self, line: &str) {
        if !self.config.info_enabled {
            return;
        }

        self.log(&format!("[INFO] {}", line));
    }

    /// Logs debug message
    pub fn log_debug(&self, line: &str) {
        if !self.config.debug_enabled {
            return;
        }

        self.log(&format!("[DEBUG] {}", line));
    }

    /// Logs trace message
    pub fn log_trace(&self, line: &str) {
        if !self.config.trace_enabled {
            return;
        }

        self.log(&format!("[TRACE] {}", line));
    }
}
