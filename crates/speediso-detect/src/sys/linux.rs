use crate::error::SpeedIsoDetectError;
use crate::model::{BusType, DiskInfo};
use std::fs;
use std::path::Path;

pub fn detect_disks() -> Result<Vec<DiskInfo>, SpeedIsoDetectError> {
    let mut disks = Vec::new();
    let sys_block = Path::new("/sys/block");

    if !sys_block.exists() {
        return Ok(disks);
    }

    let mounts = parse_proc_mounts();

    if let Ok(entries) = fs::read_dir(sys_block) {
        for entry in entries.flatten() {
            let dev_name = entry.file_name().to_string_lossy().to_string();

            // Ignore virtual devices
            if dev_name.starts_with("loop")
                || dev_name.starts_with("ram")
                || dev_name.starts_with("zram")
                || dev_name.starts_with("sr")
            {
                continue;
            }

            let dev_path = format!("/dev/{}", dev_name);
            let sys_path = sys_block.join(&dev_name);

            // Removable check
            let is_removable = fs::read_to_string(sys_path.join("removable"))
                .map(|s| s.trim() == "1")
                .unwrap_or(false);

            // Read-only check
            let is_read_only = fs::read_to_string(sys_path.join("ro"))
                .map(|s| s.trim() == "1")
                .unwrap_or(false);

            // Size check (in 512 byte sectors)
            let size_bytes = fs::read_to_string(sys_path.join("size"))
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(|sectors| sectors * 512)
                .unwrap_or(0);

            // Model / Vendor
            let vendor = fs::read_to_string(sys_path.join("device/vendor"))
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            let model = fs::read_to_string(sys_path.join("device/model"))
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            let name = match (vendor.is_empty(), model.is_empty()) {
                (false, false) => format!("{} {}", vendor, model),
                (true, false) => model,
                (false, true) => vendor,
                (true, true) => dev_name.clone(),
            };

            // Bus type heuristic
            let symlink = fs::canonicalize(&sys_path).unwrap_or_default();
            let symlink_str = symlink.to_string_lossy();

            let bus_type = if symlink_str.contains("/usb") || symlink_str.contains("/target") && symlink_str.contains("usb") {
                BusType::Usb
            } else if dev_name.starts_with("nvme") {
                BusType::Nvme
            } else if symlink_str.contains("/ata") || dev_name.starts_with("sd") {
                BusType::Sata
            } else if dev_name.starts_with("mmcblk") {
                BusType::SdCard
            } else {
                BusType::Unknown
            };

            // Mount points and System check
            let mut mount_points = Vec::new();
            let mut is_system = false;

            for (part_dev, mount) in &mounts {
                if part_dev.starts_with(&dev_path) {
                    mount_points.push(mount.clone());
                    if mount == "/" || mount == "/boot" || mount.starts_with("/boot/") {
                        is_system = true;
                    }
                }
            }

            let is_removable_final = is_removable || bus_type == BusType::Usb;

            disks.push(DiskInfo {
                id: dev_path,
                name,
                size_bytes,
                is_removable: is_removable_final,
                bus_type,
                mount_points,
                is_system,
                is_read_only,
            });
        }
    }

    Ok(disks)
}

fn parse_proc_mounts() -> Vec<(String, String)> {
    let mut mounts = Vec::new();
    if let Ok(content) = fs::read_to_string("/proc/mounts") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                mounts.push((parts[0].to_string(), parts[1].to_string()));
            }
        }
    }
    mounts
}
