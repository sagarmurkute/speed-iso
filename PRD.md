# Product Requirements Document (PRD) — SpeedISO

## 1. Executive Summary

**SpeedISO** is a free, open-source, ultra-fast cross-platform bootable USB creation tool. It is engineered to outperform balenaEtcher in resource consumption and speed, while matching the advanced bootloader capabilities of Rufus across Windows, Linux, and macOS.

## 2. Core Value Propositions

- **Bare-Metal Speed:** Direct unbuffered I/O with dynamic multi-megabyte sector-aligned buffering.
- **Minimalist Footprint:** Native binary under 20MB (Rust core + native GUI). Zero Electron bloat.
- **100% Private:** Zero telemetry, zero external network calls, zero bundled promotions.
- **Safe by Default:** Multi-stage heuristics to guarantee system and internal fixed drives cannot be overwritten accidentally.

## 3. Key Functional Requirements

### 3.1 Drive Detection & Safety Whitelist

- Filter out internal drives, NVMe system disks, and root partition mount points.
- Display drive vendor, model, capacity, and current filesystem label.
- Require explicit user confirmation for write targets.

### 3.2 Flashing Engine

- **Raw Block Mode:** Direct streaming (`dd`-style) for raw ISO, IMG, and compressed archives (`.gz`, `.xz`, `.zst`).
- **Hybrid Boot Mode:** Windows ISO extraction with automated dual-partition scheme (FAT32 boot + NTFS payload) to bypass the 4GB `install.wim` limitation under UEFI.
- **Integrity Verification:** Multi-threaded streaming checksum (SHA-256 / BLAKE3) calculated during write and verified against a post-write read-pass.

### 3.3 System Tweaks (The Rufus Feature Parity)

- Windows 11 installation bypasses (injecting custom `autounattend.xml` for TPM 2.0, Secure Boot, 8GB+ RAM, and mandatory Microsoft Account).

## 4. Non-Functional Requirements

- **Startup Time:** Under 300ms on standard hardware.
- **Memory Usage:** Peak RAM consumption under 60MB during active write/verify cycles.
- **Privilege Handling:** Separation between unprivileged UI and elevated disk-writer daemon.

## 5. Release Roadmap

- **v0.1.0 (Alpha):** Headless CLI with safe USB detection and raw block flashing.
- **v0.2.0 (Beta):** Native lightweight UI (Tauri v2 / Slint) with progress, write speeds, and ETA.
- **v1.0.0 (GA):** Windows installer extraction, TPM bypass customization, cross-platform standalone binaries.
