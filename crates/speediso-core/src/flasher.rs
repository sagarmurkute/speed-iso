use crate::buffer::{AlignedBuffer, DEFAULT_ALIGNMENT};
use crate::error::SpeedIsoCoreError;
use crate::hasher::StreamingHasher;
use crate::progress::{FlashingPhase, ProgressCallback, WriteProgress};
use crate::sys::RawDiskWriter;
use speediso_detect::detect_disks;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::time::Instant;

pub struct FlasherOptions {
    pub image_path: PathBuf,
    pub target_disk_id: String,
    pub verify: bool,
    pub progress_callback: Option<ProgressCallback>,
}

pub struct FlasherResult {
    pub bytes_written: u64,
    pub elapsed_secs: f64,
    pub sha256_checksum: String,
    pub verified: bool,
}

pub fn flash_image(options: FlasherOptions) -> Result<FlasherResult, SpeedIsoCoreError> {
    // 1. Safety Check via speediso-detect
    let disks = detect_disks()?;
    let target_disk = disks
        .into_iter()
        .find(|d| d.id.eq_ignore_ascii_case(&options.target_disk_id))
        .ok_or_else(|| SpeedIsoCoreError::UnsafeTarget {
            id: options.target_disk_id.clone(),
            reason: "Target drive not found on system".to_string(),
        })?;

    if !target_disk.is_safe_target() {
        let reason = if target_disk.is_system {
            "Drive is a protected OS system/boot volume"
        } else if !target_disk.is_removable {
            "Drive is a fixed internal disk"
        } else {
            "Drive is read-only"
        };
        return Err(SpeedIsoCoreError::UnsafeTarget {
            id: options.target_disk_id.clone(),
            reason: reason.to_string(),
        });
    }

    // 2. Open Source Image File
    let mut image_file = File::open(&options.image_path)?;
    let total_bytes = image_file.metadata()?.len();

    if target_disk.size_bytes > 0 && total_bytes > target_disk.size_bytes {
        return Err(SpeedIsoCoreError::ImageTooLarge(
            total_bytes,
            target_disk.size_bytes,
        ));
    }

    // 3. Setup Aligned Buffer (2MB chunks, 4096-byte aligned)
    let mut buffer = AlignedBuffer::with_default_capacity()
        .map_err(|e| SpeedIsoCoreError::BufferError(e))?;
    let mut writer = RawDiskWriter::open(&options.target_disk_id)?;
    let mut hasher = StreamingHasher::new();

    let start_time = Instant::now();
    let mut bytes_written: u64 = 0;

    // 4. Streaming Direct Write Loop
    loop {
        let bytes_read = image_file.read(&mut buffer[..])?;
        if bytes_read == 0 {
            break; // EOF reached
        }

        // Calculate source checksum incrementally
        hasher.update(&buffer[..bytes_read]);

        // Sector alignment padding if trailing chunk < 4096
        let write_len = if bytes_read % DEFAULT_ALIGNMENT == 0 {
            bytes_read
        } else {
            let padded_len = bytes_read + (DEFAULT_ALIGNMENT - (bytes_read % DEFAULT_ALIGNMENT));
            buffer[bytes_read..padded_len].fill(0);
            padded_len
        };

        // Unbuffered raw block write
        writer.write_all_aligned(&buffer[..write_len])?;
        bytes_written += bytes_read as u64;

        // Progress Calculation
        let elapsed = start_time.elapsed().as_secs_f64();
        let speed_mb_per_sec = if elapsed > 0.0 {
            (bytes_written as f64 / (1024.0 * 1024.0)) / elapsed
        } else {
            0.0
        };

        let remaining_bytes = total_bytes.saturating_sub(bytes_written);
        let eta_seconds = if speed_mb_per_sec > 0.0 {
            (remaining_bytes as f64 / (1024.0 * 1024.0) / speed_mb_per_sec) as u64
        } else {
            0
        };

        if let Some(ref cb) = options.progress_callback {
            cb(&WriteProgress {
                bytes_written,
                total_bytes,
                speed_mb_per_sec,
                eta_seconds,
                phase: FlashingPhase::Writing,
            });
        }
    }

    // Flush cache buffers to disk
    writer.flush()?;

    let sha256_checksum = hasher.finalize_hex();
    let mut verified = false;

    // 5. Verification Phase
    if options.verify {
        if let Some(ref cb) = options.progress_callback {
            cb(&WriteProgress {
                bytes_written: total_bytes,
                total_bytes,
                speed_mb_per_sec: 0.0,
                eta_seconds: 0,
                phase: FlashingPhase::Verifying,
            });
        }

        let mut verify_writer = RawDiskWriter::open(&options.target_disk_id)?;
        let mut verify_hasher = StreamingHasher::new();
        let mut verify_read = 0u64;

        while verify_read < total_bytes {
            let to_read = std::cmp::min(buffer.capacity() as u64, total_bytes - verify_read) as usize;
            let aligned_read = if to_read % DEFAULT_ALIGNMENT == 0 {
                to_read
            } else {
                to_read + (DEFAULT_ALIGNMENT - (to_read % DEFAULT_ALIGNMENT))
            };

            verify_writer.read_exact_aligned(&mut buffer[..aligned_read])?;
            verify_hasher.update(&buffer[..to_read]);
            verify_read += to_read as u64;

            if let Some(ref cb) = options.progress_callback {
                cb(&WriteProgress {
                    bytes_written: verify_read,
                    total_bytes,
                    speed_mb_per_sec: 0.0,
                    eta_seconds: 0,
                    phase: FlashingPhase::Verifying,
                });
            }
        }

        let target_checksum = verify_hasher.finalize_hex();
        if target_checksum != sha256_checksum {
            return Err(SpeedIsoCoreError::VerificationFailed {
                source_hash: sha256_checksum,
                target_hash: target_checksum,
            });
        }
        verified = true;
    }

    let total_elapsed = start_time.elapsed().as_secs_f64();

    if let Some(ref cb) = options.progress_callback {
        cb(&WriteProgress {
            bytes_written: total_bytes,
            total_bytes,
            speed_mb_per_sec: if total_elapsed > 0.0 {
                (total_bytes as f64 / (1024.0 * 1024.0)) / total_elapsed
            } else {
                0.0
            },
            eta_seconds: 0,
            phase: FlashingPhase::Complete,
        });
    }

    Ok(FlasherResult {
        bytes_written: total_bytes,
        elapsed_secs: total_elapsed,
        sha256_checksum,
        verified,
    })
}
