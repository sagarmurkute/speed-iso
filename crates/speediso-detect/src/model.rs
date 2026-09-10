use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BusType {
    Usb,
    Nvme,
    Sata,
    Scsi,
    SdCard,
    Unknown,
}

impl std::fmt::Display for BusType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BusType::Usb => write!(f, "USB"),
            BusType::Nvme => write!(f, "NVMe"),
            BusType::Sata => write!(f, "SATA"),
            BusType::Scsi => write!(f, "SCSI"),
            BusType::SdCard => write!(f, "SD Card"),
            BusType::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub id: String,           // e.g., "/dev/sdb", "\\.\PhysicalDrive2", "/dev/disk2"
    pub name: String,         // Model / Vendor string
    pub size_bytes: u64,
    pub is_removable: bool,
    pub bus_type: BusType,    // Usb, Nvme, Sata, Unknown
    pub mount_points: Vec<String>,
    pub is_system: bool,      // True if it holds root/boot/C: drive
    pub is_read_only: bool,
}

impl DiskInfo {
    /// Safety Invariant:
    /// Returns `true` ONLY if the device is removable, is NOT a system/boot drive, and is NOT read-only.
    pub fn is_safe_target(&self) -> bool {
        self.is_removable && !self.is_system && !self.is_read_only
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_safe_target_logic() {
        let safe_usb = DiskInfo {
            id: "\\\\.\\PhysicalDrive1".into(),
            name: "Kingston DataTraveler 3.0".into(),
            size_bytes: 32_000_000_000,
            is_removable: true,
            bus_type: BusType::Usb,
            mount_points: vec!["E:".into()],
            is_system: false,
            is_read_only: false,
        };
        assert!(safe_usb.is_safe_target());

        let system_nvme = DiskInfo {
            id: "\\\\.\\PhysicalDrive0".into(),
            name: "Samsung SSD 980 PRO 1TB".into(),
            size_bytes: 1_000_000_000_000,
            is_removable: false,
            bus_type: BusType::Nvme,
            mount_points: vec!["C:".into()],
            is_system: true,
            is_read_only: false,
        };
        assert!(!system_nvme.is_safe_target());

        let readonly_usb = DiskInfo {
            id: "\\\\.\\PhysicalDrive2".into(),
            name: "Locked USB".into(),
            size_bytes: 16_000_000_000,
            is_removable: true,
            bus_type: BusType::Usb,
            mount_points: vec!["F:".into()],
            is_system: false,
            is_read_only: true,
        };
        assert!(!readonly_usb.is_safe_target());
    }
}
