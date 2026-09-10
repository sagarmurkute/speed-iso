use crate::error::SpeedIsoDetectError;
use crate::model::{BusType, DiskInfo};
use std::process::Command;

pub fn detect_disks() -> Result<Vec<DiskInfo>, SpeedIsoDetectError> {
    let mut disks = Vec::new();

    // Run diskutil list to find disk identifiers
    let output = match Command::new("diskutil").arg("list").output() {
        Ok(out) => out,
        Err(_) => return Ok(disks),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.starts_with("/dev/disk") {
            let id = line.split_whitespace().next().unwrap_or("").to_string();
            if id.is_empty() || id.contains("s") {
                // Skip slice partitions (e.g. disk2s1)
                continue;
            }

            if let Ok(info) = inspect_macos_disk(&id) {
                disks.push(info);
            }
        }
    }

    Ok(disks)
}

fn inspect_macos_disk(disk_id: &str) -> Result<DiskInfo, SpeedIsoDetectError> {
    let output = Command::new("diskutil")
        .args(["info", disk_id])
        .output()
        .map_err(|e| SpeedIsoDetectError::PlatformError(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut name = format!("Disk {}", disk_id);
    let mut size_bytes = 0u64;
    let mut is_removable = false;
    let mut bus_type = BusType::Unknown;
    let mut mount_points = Vec::new();
    let mut is_system = false;
    let mut is_read_only = false;

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Device / Media Name:") {
            name = trimmed.split(':').nth(1).unwrap_or("").trim().to_string();
        } else if trimmed.starts_with("Disk Size:") {
            if let Some(bytes_str) = trimmed.split('(').nth(1) {
                let digits: String = bytes_str.chars().filter(|c| c.is_ascii_digit()).collect();
                size_bytes = digits.parse::<u64>().unwrap_or(0);
            }
        } else if trimmed.starts_with("Removable Media:") || trimmed.starts_with("Ejectable:") {
            if trimmed.contains("Yes") || trimmed.contains("Removable") {
                is_removable = true;
            }
        } else if trimmed.starts_with("Protocol:") {
            let proto = trimmed.split(':').nth(1).unwrap_or("").trim();
            if proto.eq_ignore_ascii_case("USB") {
                bus_type = BusType::Usb;
                is_removable = true;
            } else if proto.eq_ignore_ascii_case("PCI-Express") || proto.contains("NVMe") {
                bus_type = BusType::Nvme;
            } else if proto.eq_ignore_ascii_case("SATA") {
                bus_type = BusType::Sata;
            }
        } else if trimmed.starts_with("Mount Point:") {
            let mp = trimmed.split(':').nth(1).unwrap_or("").trim().to_string();
            if !mp.is_empty() && mp != "Not mounted" {
                if mp == "/" || mp == "/System/Volumes/Data" {
                    is_system = true;
                }
                mount_points.push(mp);
            }
        } else if trimmed.starts_with("Read-Only Media:") && trimmed.contains("Yes") {
            is_read_only = true;
        }
    }

    Ok(DiskInfo {
        id: disk_id.to_string(),
        name,
        size_bytes,
        is_removable,
        bus_type,
        mount_points,
        is_system,
        is_read_only,
    })
}
