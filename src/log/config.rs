// Log config

/// Logger configuration
pub struct LogConfig {
    // Prefix for all the logs
    pub prefix: String,

    // Error messages enabled?
    pub error_enabled: bool,

    // Warning messages enabled?
    pub warning_enabled: bool,

    // Info messages enabled?
    pub info_enabled: bool,

    // Debug messages enabled?
    pub debug_enabled: bool,

    // Trace messages enabled?
    pub trace_enabled: bool,
}

impl LogConfig {
    /// Creates a child configuration for a child logger
    ///
    /// The prefix parameter will be added to the parent's prefix,
    /// concatenated with a space
    ///
    /// Returns a new configuration for the child logger
    pub fn child_config(&self, prefix: &str) -> LogConfig {
        LogConfig {
            prefix: format!("{}{}", self.prefix, prefix),
            error_enabled: self.error_enabled,
            warning_enabled: self.warning_enabled,
            info_enabled: self.info_enabled,
            debug_enabled: self.debug_enabled,
            trace_enabled: self.trace_enabled,
        }
    }
}
