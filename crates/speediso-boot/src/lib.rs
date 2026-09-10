//! `speediso-boot`
//! Partitioning schemes and bootloader integration for SpeedISO.

pub enum PartitionScheme {
    Mbr,
    Gpt,
}
