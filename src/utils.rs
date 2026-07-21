use anyhow::{Context, Result};
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use crate::config::FilesystemType;

const LOG_FILE: &str = "/var/log/icy.log";

/// Check if the current user has root privileges
pub fn check_root_privileges() -> Result<()> {
    if !nix::unistd::Uid::effective().is_root() {
        anyhow::bail!("Root privileges required");
    }
    Ok(())
}

/// Detect the filesystem type for a given path
pub fn detect_filesystem_type(path: &str) -> Result<FilesystemType> {
    // Try to detect Btrfs
    let output = Command::new("findmnt")
        .args(&["-n", "-o", "FSTYPE", path])
        .output()
        .context("Failed to run findmnt")?;

    if output.status.success() {
        let fstype = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
        
        if fstype.contains("btrfs") {
            return Ok(FilesystemType::Btrfs);
        }
    }

    // Try to detect LVM
    let output = Command::new("lvdisplay")
        .arg(path)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            return Ok(FilesystemType::Lvm);
        }
    }

    anyhow::bail!("Unable to detect filesystem type for {}", path)
}

/// Run a system command and return error if it fails
pub fn run_command(program: &str, args: &[&str]) -> Result<()> {
    // Check if command exists
    which::which(program)
        .context(format!("Command '{}' not found. Please install it.", program))?;

    let output = Command::new(program)
        .args(args)
        .output()
        .context(format!("Failed to execute {} command", program))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Command '{}' failed with status {}: {}",
            program,
            output.status,
            stderr
        );
    }

    Ok(())
}

/// Log an action to the log file
pub fn log_action(message: &str) -> Result<()> {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_message = format!("[{}] {}\n", timestamp, message);

    // Create log file if it doesn't exist
    let log_path = Path::new(LOG_FILE);
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE)
        .context("Failed to open log file")?;

    file.write_all(log_message.as_bytes())
        .context("Failed to write to log file")?;

    log::info!("{}", message);
    Ok(())
}

/// Format bytes to human-readable size
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    
    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_idx])
}

/// Check if a path exists and is accessible
pub fn check_path_accessible(path: &str) -> Result<()> {
    let p = Path::new(path);
    
    if !p.exists() {
        anyhow::bail!("Path '{}' does not exist", path);
    }

    // Try to read metadata to check accessibility
    p.metadata()
        .context(format!("Cannot access path '{}'", path))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }
}
