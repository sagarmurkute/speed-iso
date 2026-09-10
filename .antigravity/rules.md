You are an expert systems programmer building SpeedISO, a high-performance cross-platform USB flasher in Rust.

## Core Directives & Standards

### 1. Zero Catastrophic Data Loss (Safety First)

- **Never write code that blind-writes to a disk index.** Every target block device must be verified through `speediso-detect` to confirm:
  - It is flagged as removable / USB bus.
  - It does NOT host the active OS root (`/` or `C:`).
  - It is NOT an internal NVMe, SATA system drive, or active swap volume.

### 2. Direct I/O Compliance

- Always use sector-aligned memory buffers for low-level writing. Raw devices with `O_DIRECT` or `FILE_FLAG_NO_BUFFERING` will crash or return `EINVAL` if buffers are not aligned to the disk physical sector boundary (usually 4096 bytes).
- Use custom aligned memory allocators or aligned slice crates (such as `aligned-box` or raw `alloc_zeroed` with alignment).

### 3. Cross-Platform Separation

- Strictly isolate OS-specific low-level APIs behind platform modules:
  - `sys::windows` using `windows-sys` crate.
  - `sys::linux` using `nix` and `libc`.
  - `sys::macos` using raw disk endpoints and CoreFoundation bindings.
- Common interfaces must return unified Rust types (`DiskInfo`, `WriteProgress`, `Result<T, SpeedIsoError>`).

### 4. Code Quality & Performance

- No unhandled `.unwrap()` or `.expect()` calls in production library code. Use a unified `thiserror`-based error enum.
- Keep external dependencies minimal. Avoid heavy async runtimes inside `speediso-core` unless strictly required for IPC; prefer standard threads with high-throughput bounded channels (`crossbeam-channel`).
