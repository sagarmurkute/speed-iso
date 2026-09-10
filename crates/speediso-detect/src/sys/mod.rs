#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

use crate::error::SpeedIsoDetectError;
use crate::model::DiskInfo;

pub fn detect_disks() -> Result<Vec<DiskInfo>, SpeedIsoDetectError> {
    #[cfg(target_os = "windows")]
    {
        windows::detect_disks()
    }

    #[cfg(target_os = "linux")]
    {
        linux::detect_disks()
    }

    #[cfg(target_os = "macos")]
    {
        macos::detect_disks()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(SpeedIsoDetectError::UnsupportedPlatform)
    }
}
