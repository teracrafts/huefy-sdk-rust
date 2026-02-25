/// Log level used by SDK loggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// Trait for SDK logging implementations.
pub trait Logger: Send + Sync {
    /// Logs a message at the given level.
    fn log(&self, level: LogLevel, message: &str);

    /// Convenience method for debug-level messages.
    fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }

    /// Convenience method for info-level messages.
    fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    /// Convenience method for warn-level messages.
    fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    /// Convenience method for error-level messages.
    fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }
}

/// A logger that writes to stderr with timestamps.
#[derive(Debug, Default)]
pub struct ConsoleLogger {
    /// Minimum level to emit. Messages below this level are discarded.
    pub min_level: Option<LogLevel>,
}

impl ConsoleLogger {
    /// Creates a new `ConsoleLogger` that emits all levels.
    pub fn new() -> Self {
        Self { min_level: None }
    }

    /// Creates a new `ConsoleLogger` with a minimum log level filter.
    pub fn with_level(level: LogLevel) -> Self {
        Self {
            min_level: Some(level),
        }
    }
}

impl Logger for ConsoleLogger {
    fn log(&self, level: LogLevel, message: &str) {
        if let Some(min) = self.min_level {
            if level < min {
                return;
            }
        }
        eprintln!("[huefy] [{}] {}", level, message);
    }
}

/// A logger that silently discards all messages.
#[derive(Debug, Default)]
pub struct NoopLogger;

impl NoopLogger {
    /// Creates a new `NoopLogger`.
    pub fn new() -> Self {
        Self
    }
}

impl Logger for NoopLogger {
    fn log(&self, _level: LogLevel, _message: &str) {
        // intentionally empty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_logger_does_not_panic() {
        let logger = NoopLogger::new();
        logger.debug("test");
        logger.info("test");
        logger.warn("test");
        logger.error("test");
    }

    #[test]
    fn test_console_logger_does_not_panic() {
        let logger = ConsoleLogger::new();
        logger.debug("debug message");
        logger.info("info message");
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Debug.to_string(), "DEBUG");
        assert_eq!(LogLevel::Error.to_string(), "ERROR");
    }
}
