use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Local, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{Config, FilesystemType};
use crate::utils::{detect_filesystem_type, log_action, run_command};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: usize,
    pub timestamp: DateTime<Utc>,
    pub description: String,
    pub path: String,
}

pub struct SnapshotManager {
    config: Config,
    fs_type: FilesystemType,
    metadata_file: PathBuf,
}

impl SnapshotManager {
    pub fn new(config: Config) -> Result<Self> {
        // Detect filesystem type if auto
        let fs_type = match &config.fs_type {
            FilesystemType::Auto => detect_filesystem_type(&config.path)?,
            other => other.clone(),
        };

        // Ensure snapshot directory exists
        let snapshot_dir = Path::new(&config.snapshot_dir);
        if !snapshot_dir.exists() {
            fs::create_dir_all(snapshot_dir)
                .context("Failed to create snapshot directory")?;
        }

        let metadata_file = snapshot_dir.join("metadata.yaml");

        Ok(Self {
            config,
            fs_type,
            metadata_file,
        })
    }

    pub fn list_snapshots(&mut self) -> Result<Vec<Snapshot>> {
        let mut snapshots = self.load_metadata()?;
        snapshots.sort_by_key(|s| s.id);
        Ok(snapshots)
    }

    pub fn create_snapshot(&mut self, description: &str) -> Result<Snapshot> {
        let mut snapshots = self.load_metadata()?;
        let next_id = snapshots.iter().map(|s| s.id).max().unwrap_or(0) + 1;

        let timestamp = Utc::now();
        let snapshot_name = format!("snapshot-{}", next_id);
        let snapshot_path = Path::new(&self.config.snapshot_dir).join(&snapshot_name);

        match self.fs_type {
            FilesystemType::Btrfs => {
                self.create_btrfs_snapshot(&snapshot_path)?;
            }
            FilesystemType::Lvm => {
                self.create_lvm_snapshot(&snapshot_name)?;
            }
            FilesystemType::Auto => {
                anyhow::bail!("Filesystem type detection failed");
            }
        }

        let snapshot = Snapshot {
            id: next_id,
            timestamp,
            description: description.to_string(),
            path: snapshot_path.to_string_lossy().to_string(),
        };

        snapshots.push(snapshot.clone());
        self.save_metadata(&snapshots)?;

        log_action(&format!(
            "Created snapshot #{} for {}",
            next_id, self.config.name
        ))?;

        Ok(snapshot)
    }

    pub fn delete_snapshot(&mut self, id: usize) -> Result<()> {
        let mut snapshots = self.load_metadata()?;

        let snapshot = snapshots
            .iter()
            .find(|s| s.id == id)
            .ok_or_else(|| anyhow::anyhow!("Snapshot #{} not found", id))?
            .clone();

        match self.fs_type {
            FilesystemType::Btrfs => {
                self.delete_btrfs_snapshot(&snapshot.path)?;
            }
            FilesystemType::Lvm => {
                self.delete_lvm_snapshot(&snapshot.path)?;
            }
            FilesystemType::Auto => {
                anyhow::bail!("Filesystem type detection failed");
            }
        }

        snapshots.retain(|s| s.id != id);
        self.save_metadata(&snapshots)?;

        log_action(&format!(
            "Deleted snapshot #{} for {}",
            id, self.config.name
        ))?;

        Ok(())
    }

    pub fn rollback_snapshot(&mut self, id: usize) -> Result<()> {
        let snapshots = self.load_metadata()?;

        let snapshot = snapshots
            .iter()
            .find(|s| s.id == id)
            .ok_or_else(|| anyhow::anyhow!("Snapshot #{} not found", id))?;

        match self.fs_type {
            FilesystemType::Btrfs => {
                self.rollback_btrfs_snapshot(&snapshot.path)?;
            }
            FilesystemType::Lvm => {
                self.rollback_lvm_snapshot(&snapshot.path)?;
            }
            FilesystemType::Auto => {
                anyhow::bail!("Filesystem type detection failed");
            }
        }

        log_action(&format!(
            "Rolled back to snapshot #{} for {}",
            id, self.config.name
        ))?;

        Ok(())
    }

    pub fn diff_snapshots(&self, from_id: usize, to_id: usize) -> Result<Vec<String>> {
        let snapshots = self.load_metadata()?;

        let from_snap = snapshots
            .iter()
            .find(|s| s.id == from_id)
            .ok_or_else(|| anyhow::anyhow!("Snapshot #{} not found", from_id))?;

        let to_snap = snapshots
            .iter()
            .find(|s| s.id == to_id)
            .ok_or_else(|| anyhow::anyhow!("Snapshot #{} not found", to_id))?;

        match self.fs_type {
            FilesystemType::Btrfs => self.diff_btrfs_snapshots(&from_snap.path, &to_snap.path),
            FilesystemType::Lvm => {
                // LVM doesn't have native diff, so we'd need to mount and compare
                Ok(vec!["LVM diff not yet implemented".to_string()])
            }
            FilesystemType::Auto => {
                anyhow::bail!("Filesystem type detection failed");
            }
        }
    }

    pub fn cleanup_snapshots(&mut self) -> Result<usize> {
        let mut snapshots = self.load_metadata()?;
        snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        let mut to_keep = Vec::new();
        let mut categorized: HashMap<String, Vec<&Snapshot>> = HashMap::new();

        // Categorize snapshots
        for snap in &snapshots {
            let category = if snap.timestamp > Utc::now() - chrono::Duration::hours(1) {
                "hourly"
            } else if snap.timestamp > Utc::now() - chrono::Duration::days(1) {
                "daily"
            } else if snap.timestamp > Utc::now() - chrono::Duration::weeks(1) {
                "weekly"
            } else {
                "monthly"
            };

            categorized
                .entry(category.to_string())
                .or_insert_with(Vec::new)
                .push(snap);
        }

        // Apply retention policy
        if let Some(hourly) = categorized.get("hourly") {
            to_keep.extend(hourly.iter().take(self.config.retention.hourly).copied());
        }

        if let Some(daily) = categorized.get("daily") {
            to_keep.extend(daily.iter().take(self.config.retention.daily).copied());
        }

        if let Some(weekly) = categorized.get("weekly") {
            to_keep.extend(weekly.iter().take(self.config.retention.weekly).copied());
        }

        if let Some(monthly) = categorized.get("monthly") {
            to_keep.extend(monthly.iter().take(self.config.retention.monthly).copied());
        }

        let keep_ids: Vec<usize> = to_keep.iter().map(|s| s.id).collect();
        let mut removed_count = 0;

        for snap in &snapshots {
            if !keep_ids.contains(&snap.id) {
                match self.delete_snapshot(snap.id) {
                    Ok(_) => removed_count += 1,
                    Err(e) => log::warn!("Failed to delete snapshot #{}: {}", snap.id, e),
                }
            }
        }

        Ok(removed_count)
    }

    // Btrfs operations
    fn create_btrfs_snapshot(&self, dest: &Path) -> Result<()> {
        run_command(
            "btrfs",
            &[
                "subvolume",
                "snapshot",
                "-r",
                &self.config.path,
                &dest.to_string_lossy(),
            ],
        )
    }

    fn delete_btrfs_snapshot(&self, path: &str) -> Result<()> {
        run_command("btrfs", &["subvolume", "delete", path])
    }

    fn rollback_btrfs_snapshot(&self, snapshot_path: &str) -> Result<()> {
        // Create a backup of current state
        let backup_path = format!("{}.backup", self.config.path);
        run_command(
            "btrfs",
            &[
                "subvolume",
                "snapshot",
                &self.config.path,
                &backup_path,
            ],
        )?;

        // Delete current subvolume
        run_command("btrfs", &["subvolume", "delete", &self.config.path])?;

        // Create new subvolume from snapshot
        run_command(
            "btrfs",
            &["subvolume", "snapshot", snapshot_path, &self.config.path],
        )?;

        Ok(())
    }

    fn diff_btrfs_snapshots(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let output = Command::new("btrfs")
            .args(&["subvolume", "find-new", from, "9999999"])
            .output()
            .context("Failed to run btrfs find-new")?;

        if !output.status.success() {
            anyhow::bail!("btrfs find-new failed");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let files: Vec<String> = stdout
            .lines()
            .filter_map(|line| {
                // Parse btrfs output
                line.split_whitespace().last().map(|s| s.to_string())
            })
            .collect();

        Ok(files)
    }

    // LVM operations
    fn create_lvm_snapshot(&self, name: &str) -> Result<()> {
        run_command(
            "lvcreate",
            &[
                "--snapshot",
                "--name",
                name,
                "--size",
                "5G",
                &self.config.path,
            ],
        )
    }

    fn delete_lvm_snapshot(&self, path: &str) -> Result<()> {
        run_command("lvremove", &["-f", path])
    }

    fn rollback_lvm_snapshot(&self, snapshot_path: &str) -> Result<()> {
        run_command("lvconvert", &["--merge", snapshot_path])
    }

    // Metadata operations
    fn load_metadata(&self) -> Result<Vec<Snapshot>> {
        if !self.metadata_file.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.metadata_file)
            .context("Failed to read metadata file")?;

        let snapshots: Vec<Snapshot> = serde_yaml::from_str(&content)
            .context("Failed to parse metadata file")?;

        Ok(snapshots)
    }

    fn save_metadata(&self, snapshots: &[Snapshot]) -> Result<()> {
        let yaml = serde_yaml::to_string(snapshots)
            .context("Failed to serialize metadata")?;

        fs::write(&self.metadata_file, yaml)
            .context("Failed to write metadata file")?;

        Ok(())
    }
}
