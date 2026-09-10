use clap::{Parser, Subcommand};
use speediso_core::{flash_image, FlasherOptions, FlashingPhase, WriteProgress};
use speediso_detect::{detect_disks, DiskInfo};
use std::io::{stdin, stdout, Write};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "speediso-cli")]
#[command(about = "SpeedISO CLI & Bare-Metal USB Flashing Engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List connected storage drives and display safety target evaluation
    List {
        /// Output disk enumeration results as pretty JSON
        #[arg(long)]
        json: bool,
    },
    /// Flash an ISO/IMG file directly to a target USB drive
    Flash {
        /// Path to the source ISO or IMG image file
        #[arg(long, short)]
        image: PathBuf,

        /// Target disk device ID (e.g. "\\.\PhysicalDrive1", "/dev/sdb")
        #[arg(long, short)]
        target: String,

        /// Enable post-flash SHA-256 verification read-pass
        #[arg(long)]
        verify: bool,

        /// Skip confirmation prompt and proceed immediately
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::List { json } => handle_list(json),
        Commands::Flash {
            image,
            target,
            verify,
            yes,
        } => handle_flash(image, target, verify, yes),
    }
}

fn handle_list(json: bool) {
    match detect_disks() {
        Ok(disks) => {
            if json {
                match serde_json::to_string_pretty(&disks) {
                    Ok(json_output) => println!("{}", json_output),
                    Err(e) => eprintln!("Error serializing disk list to JSON: {}", e),
                }
            } else {
                print_disk_table(&disks);
            }
        }
        Err(err) => {
            eprintln!("Error detecting disks: {}", err);
            std::process::exit(1);
        }
    }
}

fn handle_flash(image: PathBuf, target_id: String, verify: bool, auto_confirm: bool) {
    if !image.exists() {
        eprintln!("Error: Source image file not found at '{:?}'", image);
        std::process::exit(1);
    }

    // 1. Safety Verification before asking for confirmation
    let disks = match detect_disks() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error querying system storage devices: {}", e);
            std::process::exit(1);
        }
    };

    let target_disk = match disks.into_iter().find(|d| d.id.eq_ignore_ascii_case(&target_id)) {
        Some(d) => d,
        None => {
            eprintln!("Error: Target storage device '{}' was not found on the system.", target_id);
            std::process::exit(1);
        }
    };

    if !target_disk.is_safe_target() {
        eprintln!("\n🚨 CRITICAL SAFETY PROTECTION ABORT 🚨");
        eprintln!("Target device '{}' ({}) is PROTECTED.", target_disk.id, target_disk.name);
        if target_disk.is_system {
            eprintln!("REASON: Drive hosts the active operating system / boot partition ({})!", target_disk.mount_points.join(", "));
        } else if !target_disk.is_removable {
            eprintln!("REASON: Drive is an internal fixed disk (SATA/NVMe).");
        } else if target_disk.is_read_only {
            eprintln!("REASON: Drive is read-only.");
        }
        eprintln!("Flashing to protected or internal system drives is strictly prohibited.\n");
        std::process::exit(1);
    }

    // 2. Interactive Terminal Confirmation Prompt
    if !auto_confirm {
        println!("\n⚠️  WARNING: ALL DATA ON THE TARGET DRIVE WILL BE PERMANENTLY OVERWRITTEN ⚠️");
        println!("Target Disk:  {} [{}]", target_disk.name, target_disk.id);
        println!("Capacity:     {} ({})", format_size(target_disk.size_bytes), target_disk.bus_type);
        println!("Mount Points: {}", if target_disk.mount_points.is_empty() { "-".to_string() } else { target_disk.mount_points.join(", ") });
        println!("Source Image: {:?}", image);
        println!("Verify Pass:  {}", if verify { "ENABLED" } else { "DISABLED" });
        println!();
        print!("Type 'YES' to confirm overwrite and start flashing: ");
        stdout().flush().ok();

        let mut input = String::new();
        if stdin().read_line(&mut input).is_err() || input.trim() != "YES" {
            println!("\nOperation cancelled. No changes were made to the target device.");
            return;
        }
    }

    println!("\nInitiating bare-metal sector-aligned unbuffered flash operation...\n");

    let progress_callback = Arc::new(|p: &WriteProgress| {
        render_progress_bar(p);
    });

    let options = FlasherOptions {
        image_path: image,
        target_disk_id: target_id,
        verify,
        progress_callback: Some(progress_callback),
    };

    match flash_image(options) {
        Ok(res) => {
            println!("\n\n🎉 FLASHING COMPLETE!");
            println!("Bytes Written:    {} bytes ({})", res.bytes_written, format_size(res.bytes_written));
            println!("Elapsed Time:     {:.2} seconds", res.elapsed_secs);
            let avg_speed = (res.bytes_written as f64 / (1024.0 * 1024.0)) / res.elapsed_secs.max(0.001);
            println!("Average Speed:    {:.2} MB/s", avg_speed);
            println!("SHA-256 Checksum: {}", res.sha256_checksum);
            if res.verified {
                println!("Integrity Status: VERIFIED (Post-write SHA-256 match)");
            }
            println!();
        }
        Err(err) => {
            eprintln!("\n\n❌ FLASHING FAILED: {}", err);
            std::process::exit(1);
        }
    }
}

fn render_progress_bar(p: &WriteProgress) {
    let percentage = if p.total_bytes > 0 {
        (p.bytes_written as f64 / p.total_bytes as f64) * 100.0
    } else {
        0.0
    };

    let bar_width: usize = 30;
    let filled = ((percentage / 100.0) * bar_width as f64) as usize;
    let bar: String = std::iter::repeat('=')
        .take(filled.saturating_sub(1))
        .chain(if filled > 0 { ">" } else { "" }.chars())
        .chain(std::iter::repeat(' ').take(bar_width.saturating_sub(filled)))
        .collect();

    let phase_str = match p.phase {
        FlashingPhase::Writing => "Writing",
        FlashingPhase::Verifying => "Verifying",
        FlashingPhase::Complete => "Complete",
    };

    print!(
        "\r[{}] {:>5.1}% | {:>6.2} MB/s | ETA: {:>3}s | {}",
        bar, percentage, p.speed_mb_per_sec, p.eta_seconds, phase_str
    );
    stdout().flush().ok();
}

fn print_disk_table(disks: &[DiskInfo]) {
    println!();
    println!("┌───────────────────────┬──────────────────────────────────┬──────────────┬────────┬──────────────┬───────────────────────────────┐");
    println!("│ ID                    │ Drive Name / Model               │ Size         │ Bus    │ Mounts       │ Safety Status                 │");
    println!("├───────────────────────┼──────────────────────────────────┼──────────────┼────────┼──────────────┼───────────────────────────────┤");

    if disks.is_empty() {
        println!("│ No storage devices detected.                                                                                                │");
    } else {
        for disk in disks {
            let id_formatted = format!("{:<21}", truncate(&disk.id, 21));
            let name_formatted = format!("{:<32}", truncate(&disk.name, 32));
            let size_formatted = format!("{:<12}", format_size(disk.size_bytes));
            let bus_formatted = format!("{:<6}", disk.bus_type.to_string());
            
            let mounts_str = if disk.mount_points.is_empty() {
                "-".to_string()
            } else {
                disk.mount_points.join(",")
            };
            let mounts_formatted = format!("{:<12}", truncate(&mounts_str, 12));

            let status_str = if disk.is_safe_target() {
                "[SAFE TO WRITE]"
            } else if disk.is_system {
                "[PROTECTED - SYSTEM]"
            } else if disk.is_read_only {
                "[PROTECTED - READ-ONLY]"
            } else {
                "[PROTECTED - INTERNAL]"
            };
            let status_formatted = format!("{:<29}", status_str);

            println!(
                "│ {} │ {} │ {} │ {} │ {} │ {} │",
                id_formatted, name_formatted, size_formatted, bus_formatted, mounts_formatted, status_formatted
            );
        }
    }

    println!("└───────────────────────┴──────────────────────────────────┴──────────────┴────────┴──────────────┴───────────────────────────────┘");
    println!();
}

fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    let gib = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if gib >= 1.0 {
        format!("{:.1} GiB", gib)
    } else {
        let mib = bytes as f64 / (1024.0 * 1024.0);
        format!("{:.0} MiB", mib)
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}…", &s[..max_len - 1])
    } else {
        s.to_string()
    }
}
