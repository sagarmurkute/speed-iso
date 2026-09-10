use crate::error::SpeedIsoDetectError;
use crate::model::{BusType, DiskInfo};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_WRITE_PROTECT, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetDriveTypeW, GetLogicalDriveStringsW, FILE_READ_ATTRIBUTES, FILE_SHARE_READ,
    FILE_SHARE_WRITE, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Ioctl::{
    DISK_GEOMETRY_EX, GET_LENGTH_INFORMATION, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
    IOCTL_DISK_GET_LENGTH_INFO, IOCTL_DISK_IS_WRITABLE, IOCTL_STORAGE_QUERY_PROPERTY,
    PropertyStandardQuery, STORAGE_DEVICE_DESCRIPTOR, STORAGE_PROPERTY_QUERY,
    StorageDeviceProperty, VOLUME_DISK_EXTENTS,
};
use windows_sys::Win32::System::WindowsProgramming::DRIVE_REMOVABLE;

fn to_u16_null(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

pub fn detect_disks() -> Result<Vec<DiskInfo>, SpeedIsoDetectError> {
    let mut disks = Vec::new();
    let system_drive = std::env::var("SystemDrive")
        .unwrap_or_else(|_| "C:".to_string())
        .to_uppercase();

    let letter_to_disk = get_drive_letter_to_disk_map();

    let mut consecutive_failures = 0;
    for index in 0..64 {
        let drive_path = format!("\\\\.\\PhysicalDrive{}", index);
        let path_u16 = to_u16_null(&drive_path);

        let handle = unsafe {
            CreateFileW(
                path_u16.as_ptr(),
                FILE_READ_ATTRIBUTES,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null(),
                OPEN_EXISTING,
                0,
                null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            consecutive_failures += 1;
            if consecutive_failures >= 4 {
                break;
            }
            continue;
        }
        consecutive_failures = 0;

        // Query Size
        let size_bytes = query_disk_size(handle).unwrap_or(0);

        // Query Storage Descriptor (Bus Type, Removable, Model)
        let (bus_type, is_removable_bus, name) = query_storage_descriptor(handle, index);

        // Query Read-only status
        let is_read_only = query_is_read_only(handle);

        unsafe { CloseHandle(handle) };

        let mut mount_points = Vec::new();
        let mut is_system = false;

        for (letter, disk_num) in &letter_to_disk {
            if *disk_num == index {
                mount_points.push(letter.clone());
                if letter.to_uppercase() == system_drive {
                    is_system = true;
                }
            }
        }

        let is_removable = is_removable_bus || mount_points.iter().any(|m| is_removable_drive(m));

        disks.push(DiskInfo {
            id: drive_path,
            name,
            size_bytes,
            is_removable,
            bus_type,
            mount_points,
            is_system,
            is_read_only,
        });
    }

    Ok(disks)
}

fn query_disk_size(handle: HANDLE) -> Option<u64> {
    let mut length_info: GET_LENGTH_INFORMATION = unsafe { std::mem::zeroed() };
    let mut bytes_returned = 0u32;
    let res = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_DISK_GET_LENGTH_INFO,
            null(),
            0,
            &mut length_info as *mut _ as *mut _,
            std::mem::size_of::<GET_LENGTH_INFORMATION>() as u32,
            &mut bytes_returned,
            null_mut(),
        )
    };
    if res != 0 && length_info.Length > 0 {
        return Some(length_info.Length as u64);
    }

    // Fallback: IOCTL_DISK_GET_DRIVE_GEOMETRY_EX
    let mut geometry_ex: DISK_GEOMETRY_EX = unsafe { std::mem::zeroed() };
    let res = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
            null(),
            0,
            &mut geometry_ex as *mut _ as *mut _,
            std::mem::size_of::<DISK_GEOMETRY_EX>() as u32,
            &mut bytes_returned,
            null_mut(),
        )
    };
    if res != 0 && geometry_ex.DiskSize > 0 {
        Some(geometry_ex.DiskSize as u64)
    } else {
        None
    }
}

fn query_storage_descriptor(handle: HANDLE, index: u32) -> (BusType, bool, String) {
    let mut query: STORAGE_PROPERTY_QUERY = unsafe { std::mem::zeroed() };
    query.PropertyId = StorageDeviceProperty;
    query.QueryType = PropertyStandardQuery;

    let mut buffer = vec![0u8; 1024];
    let mut bytes_returned = 0u32;

    let res = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            &query as *const _ as *const _,
            std::mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
            buffer.as_mut_ptr() as *mut _,
            buffer.len() as u32,
            &mut bytes_returned,
            null_mut(),
        )
    };

    if res == 0 || bytes_returned < std::mem::size_of::<STORAGE_DEVICE_DESCRIPTOR>() as u32 {
        return (BusType::Unknown, false, format!("Physical Drive {}", index));
    }

    let descriptor = unsafe { &*(buffer.as_ptr() as *const STORAGE_DEVICE_DESCRIPTOR) };

    let bus_type = match descriptor.BusType {
        7 => BusType::Usb,
        17 => BusType::Nvme,
        3 | 11 => BusType::Sata,
        1 | 10 => BusType::Scsi,
        13 => BusType::SdCard,
        _ => BusType::Unknown,
    };

    let is_removable = descriptor.RemovableMedia != 0 || bus_type == BusType::Usb;

    let vendor = extract_string(&buffer, descriptor.VendorIdOffset as usize);
    let product = extract_string(&buffer, descriptor.ProductIdOffset as usize);

    let name = match (vendor.is_empty(), product.is_empty()) {
        (false, false) => format!("{} {}", vendor, product),
        (true, false) => product,
        (false, true) => vendor,
        (true, true) => format!("Generic Storage Device {}", index),
    };

    (bus_type, is_removable, name.trim().to_string())
}

fn extract_string(buffer: &[u8], offset: usize) -> String {
    if offset == 0 || offset >= buffer.len() {
        return String::new();
    }
    let bytes = &buffer[offset..];
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

fn query_is_read_only(handle: HANDLE) -> bool {
    let mut bytes_returned = 0u32;
    let res = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_DISK_IS_WRITABLE,
            null(),
            0,
            null_mut(),
            0,
            &mut bytes_returned,
            null_mut(),
        )
    };
    if res == 0 {
        let err = unsafe { GetLastError() };
        err == ERROR_WRITE_PROTECT
    } else {
        false
    }
}

fn get_drive_letter_to_disk_map() -> Vec<(String, u32)> {
    let mut map = Vec::new();
    let mut buffer = vec![0u16; 512];
    let len = unsafe { GetLogicalDriveStringsW(buffer.len() as u32, buffer.as_mut_ptr()) };
    if len == 0 || len > buffer.len() as u32 {
        return map;
    }

    let drives: Vec<String> = buffer[..len as usize]
        .split(|&c| c == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf16_lossy(s).trim_matches('\\').to_string())
        .collect();

    for drive in drives {
        let vol_path = format!("\\\\.\\{}", drive);
        let vol_u16 = to_u16_null(&vol_path);
        let handle = unsafe {
            CreateFileW(
                vol_u16.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null(),
                OPEN_EXISTING,
                0,
                null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            continue;
        }

        let mut extents_buf = vec![0u8; 512];
        let mut bytes_returned = 0u32;
        let res = unsafe {
            DeviceIoControl(
                handle,
                IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS,
                null(),
                0,
                extents_buf.as_mut_ptr() as *mut _,
                extents_buf.len() as u32,
                &mut bytes_returned,
                null_mut(),
            )
        };

        unsafe { CloseHandle(handle) };

        if res != 0 && bytes_returned >= std::mem::size_of::<VOLUME_DISK_EXTENTS>() as u32 {
            let extents = unsafe { &*(extents_buf.as_ptr() as *const VOLUME_DISK_EXTENTS) };
            if extents.NumberOfDiskExtents > 0 {
                let disk_num = extents.Extents[0].DiskNumber;
                map.push((drive, disk_num));
            }
        }
    }

    map
}

fn is_removable_drive(drive_letter: &str) -> bool {
    let path = format!("{}\\", drive_letter);
    let path_u16 = to_u16_null(&path);
    let drive_type = unsafe { GetDriveTypeW(path_u16.as_ptr()) };
    drive_type == DRIVE_REMOVABLE
}
