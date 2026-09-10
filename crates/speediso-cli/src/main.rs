use clap::{Parser, Subcommand};
use speediso_detect::{detect_disks, DiskInfo};

#[derive(Parser)]
#[command(name = "speediso-cli")]
#[command(about = "SpeedISO CLI & Disk Enumeration Tool", long_about = None)]
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
}

fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::List { json } => {
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
    }
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
