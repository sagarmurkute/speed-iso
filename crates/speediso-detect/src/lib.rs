//! `speediso-detect`
//! Cross-platform storage drive enumeration and system drive protection module.

pub mod error;
pub mod model;
pub mod sys;

pub use error::SpeedIsoDetectError;
pub use model::{BusType, DiskInfo};
pub use sys::detect_disks;
