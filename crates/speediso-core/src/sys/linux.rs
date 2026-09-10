use crate::error::SpeedIsoCoreError;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;

pub struct RawDiskWriter {
    file: File,
    path: String,
}

impl RawDiskWriter {
    pub fn open(device_id: &str) -> Result<Self, SpeedIsoCoreError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(0x4000 | 0x101000) // O_DIRECT | O_SYNC
            .open(device_id)?;

        Ok(Self {
            file,
            path: device_id.to_string(),
        })
    }

    pub fn write_all_aligned(&mut self, buffer: &[u8]) -> Result<(), SpeedIsoCoreError> {
        self.file.write_all(buffer)?;
        Ok(())
    }

    pub fn read_exact_aligned(&mut self, buffer: &mut [u8]) -> Result<(), SpeedIsoCoreError> {
        self.file.read_exact(buffer)?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), SpeedIsoCoreError> {
        self.file.sync_data()?;
        Ok(())
    }
}
