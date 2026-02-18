use crate::error::MemoryError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Input error: {0}")]
    Input(String),

    #[error("Output error: {0}")]
    Output(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("{0}")]
    Memory(#[from] MemoryError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for CliError {
    fn from(err: anyhow::Error) -> Self {
        CliError::Other(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CliError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    Success = 0,
    ConfigError = 1,
    DatabaseError = 2,
    InputError = 3,
    OutputError = 4,
    ValidationError = 5,
    MemoryError = 6,
    IoError = 7,
    UnknownError = 255,
}

impl From<&CliError> for ExitCode {
    fn from(error: &CliError) -> Self {
        match error {
            CliError::Config(_) => ExitCode::ConfigError,
            CliError::Database(_) => ExitCode::DatabaseError,
            CliError::Input(_) => ExitCode::InputError,
            CliError::Output(_) => ExitCode::OutputError,
            CliError::Validation(_) => ExitCode::ValidationError,
            CliError::Memory(_) => ExitCode::MemoryError,
            CliError::Io(_) => ExitCode::IoError,
            CliError::Other(_) => ExitCode::UnknownError,
        }
    }
}

impl From<CliError> for ExitCode {
    fn from(error: CliError) -> Self {
        ExitCode::from(&error)
    }
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        ExitCode::from(self) as i32
    }
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code_mapping() {
        assert_eq!(CliError::Config("x".into()).exit_code(), 1);
        assert_eq!(CliError::Database("x".into()).exit_code(), 2);
        assert_eq!(CliError::Input("x".into()).exit_code(), 3);
        assert_eq!(CliError::Output("x".into()).exit_code(), 4);
        assert_eq!(CliError::Validation("x".into()).exit_code(), 5);
    }

    #[test]
    fn test_memory_error_conversion() {
        let cli_err: CliError = MemoryError::InvalidInput {
            field: "test".into(),
            reason: "invalid".into(),
        }
        .into();
        assert!(matches!(cli_err, CliError::Memory(_)));
        assert_eq!(cli_err.exit_code(), 6);
    }
}
