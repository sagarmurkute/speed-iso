use thiserror::Error;

#[derive(Error, Debug)]
pub enum SpeedIsoDetectError {
    #[error("I/O error during disk enumeration: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Platform enumeration error: {0}")]
    PlatformError(String),

    #[error("Permission denied when accessing disk details (requires administrative privileges)")]
    PermissionDenied,

    #[error("Unsupported target platform")]
    UnsupportedPlatform,
}
