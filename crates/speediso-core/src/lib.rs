//! `speediso-core`
//! Bare-metal direct I/O, sector alignment, streaming engine, and checksum verification for SpeedISO.

pub mod buffer;
pub mod error;
pub mod flasher;
pub mod hasher;
pub mod progress;
pub mod sys;

pub use buffer::AlignedBuffer;
pub use error::SpeedIsoCoreError;
pub use flasher::{flash_image, FlasherOptions, FlasherResult};
pub use hasher::StreamingHasher;
pub use progress::{FlashingPhase, WriteProgress};
