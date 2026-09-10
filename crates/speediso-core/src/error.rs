use thiserror::Error;

#[derive(Error, Debug)]
pub enum SpeedIsoCoreError {
    #[error("I/O error during flashing operation: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Target drive '{id}' is protected or unsafe to overwrite: {reason}")]
    UnsafeTarget { id: String, reason: String },

    #[error("Failed to acquire volume lock or dismount target volume: {0}")]
    VolumeLockFailed(String),

    #[error("Image file size ({0} bytes) exceeds target drive capacity ({1} bytes)")]
    ImageTooLarge(u64, u64),

    #[error("Verification failure: Checksum mismatch (source SHA-256: {source_hash}, target SHA-256: {target_hash})")]
    VerificationFailed {
        source_hash: String,
        target_hash: String,
    },

    #[error("Memory buffer alignment error: {0}")]
    BufferError(String),

    #[error("Target device query error: {0}")]
    DetectError(#[from] speediso_detect::SpeedIsoDetectError),
}
