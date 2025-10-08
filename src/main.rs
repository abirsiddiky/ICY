use anyhow::Result;
use clap::{Parser, Subcommand};
use std::process;

mod config;
mod core;
mod ui;
mod utils;

use config::ConfigManager;
use core::SnapshotManager;
use ui::TuiApp;
use utils::check_root_privileges;

#[derive(Parser)]
#[command(name = "icy")]
#[command(about = "A modern snapshot manager for Linux", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List snapshots for a configuration
    List {
        /// Configuration name
        #[arg(short, long, default_value = "root")]
        config: String,
    },
    /// Create a new snapshot
    Create {
        /// Configuration name
        #[arg(short, long, default_value = "root")]
        config: String,
        /// Snapshot description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Delete a snapshot
    Delete {
        /// Configuration name
        #[arg(short, long, default_value = "root")]
        config: String,
        /// Snapshot ID
        #[arg(short, long)]
        snapshot: usize,
    },
    /// Rollback to a snapshot
    Rollback {
        /// Configuration name
        #[arg(short, long, default_value = "root")]
        config: String,
        /// Snapshot ID
        #[arg(short, long)]
        snapshot: usize,
    },
    /// Show diff between snapshots
    Diff {
        /// Configuration name
        #[arg(short, long, default_value = "root")]
        config: String,
        /// From snapshot ID
        #[arg(short, long)]
        from: usize,
        /// To snapshot ID
        #[arg(short, long)]
        to: usize,
    },
    /// Clean up old snapshots based on retention policy
    Cleanup {
        /// Configuration name (optional, cleans all if not specified)
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Initialize a new configuration
    Init {
        /// Configuration name
        name: String,
        /// Path to monitor
        path: String,
        /// Snapshot directory
        #[arg(short, long)]
        snapshot_dir: Option<String>,
    },
}

fn print_logo() {
    println!(
        r#"
    ██╗ ██████╗██╗   ██╗
    ██║██╔════╝╚██╗ ██╔╝
    ██║██║      ╚████╔╝ 
    ██║██║       ╚██╔╝  
    ██║╚██████╗   ██║   
    ╚═╝ ╚═════╝   ╚═╝   
    
    Modern Snapshot Manager
    "#
    );
}

fn main() {
    env_logger::init();

    let cli = Cli::parse();

    // Check root privileges
    if let Err(e) = check_root_privileges() {
        eprintln!("Error: {}", e);
        eprintln!("Please run icy with root privileges (sudo)");
        process::exit(1);
    }

    let result = match cli.command {
        None => {
            // Launch TUI mode
            print_logo();
            run_tui()
        }
        Some(Commands::List { config }) => cmd_list(&config),
        Some(Commands::Create { config, description }) => cmd_create(&config, description),
        Some(Commands::Delete { config, snapshot }) => cmd_delete(&config, snapshot),
        Some(Commands::Rollback { config, snapshot }) => cmd_rollback(&config, snapshot),
        Some(Commands::Diff { config, from, to }) => cmd_diff(&config, from, to),
        Some(Commands::Cleanup { config }) => cmd_cleanup(config.as_deref()),
        Some(Commands::Init {
            name,
            path,
            snapshot_dir,
        }) => cmd_init(&name, &path, snapshot_dir.as_deref()),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run_tui() -> Result<()> {
    let mut app = TuiApp::new()?;
    app.run()?;
    Ok(())
}

fn cmd_list(config_name: &str) -> Result<()> {
    let config_mgr = ConfigManager::new()?;
    let config = config_mgr.load_config(config_name)?;
    let mut mgr = SnapshotManager::new(config)?;

    let snapshots = mgr.list_snapshots()?;

    println!("\nSnapshots for config '{}':\n", config_name);
    println!("{:<5} {:<20} {:<30} {}", "ID", "Date", "Time", "Description");
    println!("{}", "-".repeat(80));

    for snap in snapshots {
        println!(
            "{:<5} {:<20} {:<30} {}",
            snap.id,
            snap.timestamp.format("%Y-%m-%d"),
            snap.timestamp.format("%H:%M:%S"),
            snap.description
        );
    }

    Ok(())
}

fn cmd_create(config_name: &str, description: Option<String>) -> Result<()> {
    let config_mgr = ConfigManager::new()?;
    let config = config_mgr.load_config(config_name)?;
    let mut mgr = SnapshotManager::new(config)?;

    let desc = description.unwrap_or_else(|| "Manual snapshot".to_string());
    let snapshot = mgr.create_snapshot(&desc)?;

    println!("✓ Snapshot created: #{} - {}", snapshot.id, desc);
    Ok(())
}

fn cmd_delete(config_name: &str, snapshot_id: usize) -> Result<()> {
    let config_mgr = ConfigManager::new()?;
    let config = config_mgr.load_config(config_name)?;
    let mut mgr = SnapshotManager::new(config)?;

    mgr.delete_snapshot(snapshot_id)?;
    println!("✓ Snapshot #{} deleted", snapshot_id);
    Ok(())
}

fn cmd_rollback(config_name: &str, snapshot_id: usize) -> Result<()> {
    let config_mgr = ConfigManager::new()?;
    let config = config_mgr.load_config(config_name)?;
    let mut mgr = SnapshotManager::new(config)?;

    println!("⚠️  Rolling back to snapshot #{}...", snapshot_id);
    println!("This will revert your system to a previous state.");
    println!("Make sure you have backed up any important data!");
    println!("\nPress Enter to continue or Ctrl+C to cancel...");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    mgr.rollback_snapshot(snapshot_id)?;
    println!("✓ Rollback complete. Please reboot your system.");
    Ok(())
}

fn cmd_diff(config_name: &str, from: usize, to: usize) -> Result<()> {
    let config_mgr = ConfigManager::new()?;
    let config = config_mgr.load_config(config_name)?;
    let mgr = SnapshotManager::new(config)?;

    let diff = mgr.diff_snapshots(from, to)?;

    println!("\nDiff between snapshot #{} and #{}:\n", from, to);
    println!("Changed files:");
    for file in diff {
        println!("  {}", file);
    }

    Ok(())
}

fn cmd_cleanup(config_name: Option<&str>) -> Result<()> {
    let config_mgr = ConfigManager::new()?;

    match config_name {
        Some(name) => {
            let config = config_mgr.load_config(name)?;
            let mut mgr = SnapshotManager::new(config)?;
            let removed = mgr.cleanup_snapshots()?;
            println!("✓ Cleaned up {} snapshots for '{}'", removed, name);
        }
        None => {
            let configs = config_mgr.list_configs()?;
            let mut total = 0;
            for config in configs {
                let mut mgr = SnapshotManager::new(config.clone())?;
                let removed = mgr.cleanup_snapshots()?;
                total += removed;
                if removed > 0 {
                    println!("✓ Cleaned up {} snapshots for '{}'", removed, config.name);
                }
            }
            println!("\n✓ Total snapshots removed: {}", total);
        }
    }

    Ok(())
}

fn cmd_init(name: &str, path: &str, snapshot_dir: Option<&str>) -> Result<()> {
    let config_mgr = ConfigManager::new()?;

    let snapshot_dir = snapshot_dir
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}/.icy-snapshots/{}", path, name));

    config_mgr.create_config(name, path, &snapshot_dir)?;
    println!("✓ Configuration '{}' initialized", name);
    println!("  Path: {}", path);
    println!("  Snapshot directory: {}", snapshot_dir);

    Ok(())
}
