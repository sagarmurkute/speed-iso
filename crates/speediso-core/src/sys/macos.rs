use crate::error::SpeedIsoCoreError;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

pub struct RawDiskWriter {
    file: File,
    path: String,
}

impl RawDiskWriter {
    pub fn open(device_id: &str) -> Result<Self, SpeedIsoCoreError> {
        // macOS raw disk optimization: convert /dev/diskX to /dev/rdiskX if applicable
        let raw_path = if device_id.starts_with("/dev/disk") && !device_id.starts_with("/dev/rdisk") {
            device_id.replace("/dev/disk", "/dev/rdisk")
        } else {
            device_id.to_string()
        };

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&raw_path)?;

        Ok(Self {
            file,
            path: raw_path,
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
