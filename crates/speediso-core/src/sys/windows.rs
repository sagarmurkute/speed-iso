use crate::error::SpeedIsoCoreError;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{
    CloseHandle, HANDLE, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FlushFileBuffers, ReadFile, WriteFile, FILE_FLAG_NO_BUFFERING,
    FILE_FLAG_WRITE_THROUGH, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Ioctl::{
    FSCTL_DISMOUNT_VOLUME, FSCTL_LOCK_VOLUME, FSCTL_UNLOCK_VOLUME,
};

fn to_u16_null(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

pub struct RawDiskWriter {
    handle: HANDLE,
    path: String,
}

impl RawDiskWriter {
    pub fn open(device_id: &str) -> Result<Self, SpeedIsoCoreError> {
        let path_u16 = to_u16_null(device_id);

        let handle = unsafe {
            CreateFileW(
                path_u16.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null(),
                OPEN_EXISTING,
                FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH,
                null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(SpeedIsoCoreError::IoError(std::io::Error::last_os_error()));
        }

        let mut writer = Self {
            handle,
            path: device_id.to_string(),
        };

        // Lock & Dismount target device volume
        writer.lock_and_dismount()?;

        Ok(writer)
    }

    fn lock_and_dismount(&mut self) -> Result<(), SpeedIsoCoreError> {
        let mut bytes_returned = 0u32;

        // Try locking volume
        let res_lock = unsafe {
            DeviceIoControl(
                self.handle,
                FSCTL_LOCK_VOLUME,
                null(),
                0,
                null_mut(),
                0,
                &mut bytes_returned,
                null_mut(),
            )
        };
        if res_lock == 0 {
            tracing::warn!("FSCTL_LOCK_VOLUME failed on {}, continuing dismount...", self.path);
        }

        // Dismount volume
        let res_dismount = unsafe {
            DeviceIoControl(
                self.handle,
                FSCTL_DISMOUNT_VOLUME,
                null(),
                0,
                null_mut(),
                0,
                &mut bytes_returned,
                null_mut(),
            )
        };
        if res_dismount == 0 {
            tracing::warn!("FSCTL_DISMOUNT_VOLUME failed on {}", self.path);
        }

        Ok(())
    }

    pub fn write_all_aligned(&mut self, buffer: &[u8]) -> Result<(), SpeedIsoCoreError> {
        let mut total_written = 0usize;
        while total_written < buffer.len() {
            let to_write = (buffer.len() - total_written) as u32;
            let mut written = 0u32;
            let res = unsafe {
                WriteFile(
                    self.handle,
                    buffer[total_written..].as_ptr(),
                    to_write,
                    &mut written,
                    null_mut(),
                )
            };
            if res == 0 || written == 0 {
                return Err(SpeedIsoCoreError::IoError(std::io::Error::last_os_error()));
            }
            total_written += written as usize;
        }
        Ok(())
    }

    pub fn read_exact_aligned(&mut self, buffer: &mut [u8]) -> Result<(), SpeedIsoCoreError> {
        let mut total_read = 0usize;
        while total_read < buffer.len() {
            let to_read = (buffer.len() - total_read) as u32;
            let mut read_bytes = 0u32;
            let res = unsafe {
                ReadFile(
                    self.handle,
                    buffer[total_read..].as_mut_ptr(),
                    to_read,
                    &mut read_bytes,
                    null_mut(),
                )
            };
            if res == 0 || read_bytes == 0 {
                return Err(SpeedIsoCoreError::IoError(std::io::Error::last_os_error()));
            }
            total_read += read_bytes as usize;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), SpeedIsoCoreError> {
        let res = unsafe { FlushFileBuffers(self.handle) };
        if res == 0 {
            Err(SpeedIsoCoreError::IoError(std::io::Error::last_os_error()))
        } else {
            Ok(())
        }
    }
}

impl Drop for RawDiskWriter {
    fn drop(&mut self) {
        if self.handle != INVALID_HANDLE_VALUE {
            let mut bytes_returned = 0u32;
            unsafe {
                DeviceIoControl(
                    self.handle,
                    FSCTL_UNLOCK_VOLUME,
                    null(),
                    0,
                    null_mut(),
                    0,
                    &mut bytes_returned,
                    null_mut(),
                );
                CloseHandle(self.handle);
            }
        }
    }
}
