use thiserror::Error;

#[derive(Error, Debug)]
pub enum NipeError {
    #[error("Tor process failed to start: {0}")]
    TorStartFailed(String),

    #[error("Tor process failed to stop: {0}")]
    TorStopFailed(String),

    #[error("Tor bootstrap timeout")]
    BootstrapTimeout,

    #[error("Not connected to Tor network")]
    NotConnected,

    #[error("Firewall configuration failed: {0}")]
    FirewallError(String),

    #[error("Network interface not found")]
    InterfaceNotFound,

    #[allow(dead_code)]
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, NipeError>;
