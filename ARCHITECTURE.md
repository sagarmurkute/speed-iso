# Architecture Specification — SpeedISO

## 1. Workspace Layout

SpeedISO is built as a modular Cargo workspace in Rust:

```text
speediso/
├── Cargo.toml                    # Root workspace definition
├── crates/
│   ├── speediso-core/            # Low-level direct I/O, sector alignment, streaming
│   ├── speediso-detect/          # Cross-platform drive & partition enumeration
│   ├── speediso-boot/            # Bootloader logic (MBR/GPT/UEFI), unattend injection
│   └── speediso-cli/             # Command-line interface and daemon runner
├── ui/                           # Frontend client application (IPC-driven)
└── .antigravity/                 # Agent rules, workflows, and task configs
```
