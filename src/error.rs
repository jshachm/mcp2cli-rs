use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    CliError = 1,
    NetworkError = 2,
    ProtocolError = 3,
    ExecutionError = 4,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

#[derive(Debug, Clone)]
pub enum AppError {
    Cli(String),
    Network(String),
    Protocol(String),
    Execution(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Cli(msg) => write!(f, "CLI Error: {}", msg),
            AppError::Network(msg) => write!(f, "Network Error: {}", msg),
            AppError::Protocol(msg) => write!(f, "Protocol Error: {}", msg),
            AppError::Execution(msg) => write!(f, "Execution Error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl AppError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            AppError::Cli(_) => ExitCode::CliError,
            AppError::Network(_) => ExitCode::NetworkError,
            AppError::Protocol(_) => ExitCode::ProtocolError,
            AppError::Execution(_) => ExitCode::ExecutionError,
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Execution(err.to_string())
    }
}
