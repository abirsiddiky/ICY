# ❄️ ICY - Modern Linux Snapshot Manager

A beautiful, modern terminal-based snapshot manager for Linux, written in Rust. ICY supports both Btrfs subvolumes and LVM thin-provisioned snapshots with an intuitive TUI interface.

```
    ██╗ ██████╗██╗   ██╗
    ██║██╔════╝╚██╗ ██╔╝
    ██║██║      ╚████╔╝ 
    ██║██║       ╚██╔╝  
    ██║╚██████╗   ██║   
    ╚═╝ ╚═════╝   ╚═╝   
```

## ✨ Features

- 🖥️ **Beautiful TUI**: Modern, colorful terminal interface built with Ratatui
- 📸 **Snapshot Management**: Create, list, delete, and rollback snapshots
- 🔄 **Multiple Filesystems**: Support for Btrfs and LVM
- 📋 **Multiple Configurations**: Manage different snapshot sets (root, home, etc.)
- 🧹 **Auto-Cleanup**: Retention policies (hourly, daily, weekly, monthly)
- 🔍 **Diff View**: Compare snapshots and see changed files
- 📝 **Logging**: All actions logged to `/var/log/icy.log`
- ⚡ **Fast & Safe**: Written in Rust for performance and safety

## 📦 Installation

### Prerequisites

- Rust toolchain (1.70+)
- Root/sudo access
- For Btrfs: `btrfs-progs` package
- For LVM: `lvm2` package

### Step 1: Install Rust (if not already installed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Step 2: Install System Dependencies

**Debian/Ubuntu:**
```bash
sudo apt update
sudo apt install btrfs-progs lvm2 build-essential
```

**Fedora:**
```bash
sudo dnf install btrfs-progs lvm2 gcc
```

**Arch Linux:**
```bash
sudo pacman -S btrfs-progs lvm2 base-devel
```

### Step 3: Build and Install ICY

#### Option A: Quick Install Script

```bash
git clone https://github.com/abirsiddiky/icy.git
cd icy/

# Run the installer
chmod +x install.sh
sudo ./install.sh
```

#### Option B: Manual Installation

```bash
# Build release binary
cargo build --release

# Install (requires sudo)
sudo cp target/release/icy /usr/local/bin/
sudo chmod +x /usr/local/bin/icy

# Create config directory
sudo mkdir -p /etc/icy/configs
```

## 🚀 Quick Start

### First Time Setup

#### 1. Initialize Your First Configuration

For a Btrfs root filesystem:
```bash
sudo icy init root / --snapshot-dir /.icy-snapshots/root
```

For your home directory:
```bash
sudo icy init home /home --snapshot-dir /home/.icy-snapshots
```

#### 2. Launch the TUI

```bash
sudo icy
```

You should see the beautiful ICY interface with your configuration(s) listed!

**Keyboard Shortcuts:**
- `↑/↓` - Navigate lists
- `Tab` - Switch between panels
- `c` - Create new snapshot
- `r` - Rollback to selected snapshot
- `d` - Delete selected snapshot
- `q` - Quit

#### 3. Create Your First Snapshot

**In TUI mode:**
- Press `c` to create a snapshot
- Enter a description (e.g., "Initial snapshot")
- Press Enter

**In CLI mode:**
```bash
sudo icy create --config root --description "Initial snapshot"
```

## 💻 Usage

### TUI Mode (Recommended)

Simply run `icy` to launch the interactive terminal interface:

```bash
sudo icy
```

### CLI Mode

ICY also provides a full command-line interface:

#### List Snapshots

```bash
sudo icy list --config root
```

#### Create Snapshot

```bash
sudo icy create --config root --description "Before system update"
```

#### Delete Snapshot

```bash
sudo icy delete --config root --snapshot 5
```

#### Rollback to Snapshot

```bash
sudo icy rollback --config root --snapshot 3
```

**⚠️ Warning**: Always backup important data before rollback!

#### Compare Snapshots

```bash
sudo icy diff --config root --from 3 --to 5
```

#### Cleanup Old Snapshots

```bash
# Clean specific config
sudo icy cleanup --config root

# Clean all configs
sudo icy cleanup
```

## ⚙️ Configuration

Configurations are stored in `/etc/icy/configs/` as YAML files.

### Example Configuration

```yaml
# /etc/icy/configs/root.yaml
name: root
path: /
snapshot_dir: /.icy-snapshots/root
retention:
  hourly: 0
  daily: 7
  weekly: 4
  monthly: 3
fs_type: auto  # or 'btrfs' or 'lvm'
```

### Retention Policy Examples

#### Aggressive Retention (Keep More Snapshots)

```yaml
retention:
  hourly: 24    # Keep 24 hourly
  daily: 14     # Keep 14 daily
  weekly: 8     # Keep 8 weekly
  monthly: 12   # Keep 12 monthly
```

#### Minimal Retention (Keep Fewer Snapshots)

```yaml
retention:
  hourly: 0     # No hourly
  daily: 3      # Keep 3 daily
  weekly: 2     # Keep 2 weekly
  monthly: 1    # Keep 1 monthly
```

Set any value to `0` to disable that retention level.

## 📋 Common Tasks

### View All Snapshots

**TUI:** Just run `sudo icy` and navigate with arrow keys

**CLI:**
```bash
sudo icy list --config root
```

### Create Snapshot Before System Update

```bash
sudo icy create --config root --description "Before system update $(date +%Y-%m-%d)"
sudo apt upgrade  # or your package manager
```

### Rollback After Bad Update

**TUI:**
1. Run `sudo icy`
2. Navigate to the snapshot you want
3. Press `r` for rollback
4. Confirm and reboot

**CLI:**
```bash
sudo icy list --config root  # Find the snapshot ID
sudo icy rollback --config root --snapshot 3
sudo reboot
```

### Multiple Configurations

Create separate configs for different parts of your system:

```bash
sudo icy init root /
sudo icy init home /home
sudo icy init data /mnt/data
sudo icy init docker /var/lib/docker
```

Then manage them all in the TUI or individually via CLI.

## 🤖 Advanced Usage

### Automated Snapshots

Create a cron job for automatic snapshots:

```bash
sudo crontab -e
```

Add:
```cron
# Daily snapshot at 2 AM
0 2 * * * /usr/local/bin/icy create --config root --description "Auto-daily $(date +\%Y-\%m-\%d)"

# Cleanup weekly on Sunday at 3 AM
0 3 * * 0 /usr/local/bin/icy cleanup
```

### Pre/Post Package Manager Hooks

**For APT (Debian/Ubuntu):**

Create `/etc/apt/apt.conf.d/80-icy-snapshots`:
```
DPkg::Pre-Invoke {"/usr/local/bin/icy create --config root --description 'Pre-APT'";};
```

**For DNF (Fedora):**

Add to `/etc/dnf/plugins/pre-transaction-actions.d/icy.action`:
```
*:any:/usr/local/bin/icy create --config root --description 'Pre-DNF'
```

## 🔧 Supported Filesystems

### Btrfs

ICY uses Btrfs subvolume snapshots:
- Read-only snapshots by default
- Fast, space-efficient CoW (Copy-on-Write)
- Native snapshot comparison

### LVM

ICY uses LVM thin-provisioned snapshots:
- Requires thin-provisioned logical volumes
- Configurable snapshot size
- Merge-based rollback

## 🏗️ Project Architecture

```
icy/
├── src/
│   ├── main.rs       # CLI entry point and command handling
│   ├── core.rs       # Snapshot management (Btrfs/LVM operations)
│   ├── ui.rs         # TUI interface with Ratatui
│   ├── config.rs     # Configuration management
│   └── utils.rs      # Utilities and system detection
├── Cargo.toml
├── README.md
└── install.sh
```

## 🐛 Troubleshooting

### "Command not found: btrfs"

You need to install btrfs-progs:
```bash
sudo apt install btrfs-progs  # Debian/Ubuntu
sudo dnf install btrfs-progs  # Fedora
sudo pacman -S btrfs-progs    # Arch
```

### "Command not found: lvm"

Install LVM tools:
```bash
sudo apt install lvm2         # Debian/Ubuntu
sudo dnf install lvm2         # Fedora
sudo pacman -S lvm2           # Arch
```

### "Root privileges required"

Always run ICY with sudo:
```bash
sudo icy  # Always run with sudo
```

### Snapshot Directory Full

Check available space:
```bash
df -h /.icy-snapshots/
```

Clean up old snapshots:
```bash
sudo icy cleanup --config root
```

Or adjust retention policy to keep fewer snapshots.

### TUI Not Displaying Correctly

Ensure your terminal:
- Supports colors
- Has adequate size (at least 80x24)
- Is not tmux/screen without proper TERM variable

### Snapshot directory issues

Ensure snapshot directories exist and are accessible:

```bash
sudo mkdir -p /.icy-snapshots/root
sudo chmod 755 /.icy-snapshots
```

## 📝 Logging & Debugging

All operations are logged to `/var/log/icy.log`:

```bash
# View logs in real-time
sudo tail -f /var/log/icy.log

# Check recent actions
sudo tail -n 50 /var/log/icy.log

# Search for errors
sudo grep -i error /var/log/icy.log
```

### Getting Help

Built-in help:
```bash
icy --help
icy create --help
icy rollback --help
```

Check configuration:
```bash
cat /etc/icy/configs/root.yaml
```

## ✅ Best Practices

1. **Create snapshots before major changes:**
   - System updates
   - Installing new software
   - Configuration changes

2. **Use descriptive names:**
   ```bash
   sudo icy create --config root --description "Before installing nvidia drivers"
   ```

3. **Regular cleanup:**
   - Run `sudo icy cleanup` weekly or set up a cron job

4. **Test rollback in non-critical environment first:**
   - Try rollback on a test system before production

5. **Monitor disk space:**
   - Snapshots consume disk space
   - Keep an eye on `df -h`

6. **Backup important data separately:**
   - Snapshots are not backups
   - Keep off-system backups for critical data

## 📊 Example Workflows

### Daily Backup Workflow

```bash
# Create config
sudo icy init system /

# Create snapshot before updates
sudo icy create --config system --description "Pre-update backup"

# After update, if issues occur
sudo icy rollback --config system --snapshot 1

# Cleanup old snapshots
sudo icy cleanup --config system
```

### Multi-Directory Management

```bash
# Setup multiple configurations
sudo icy init root /
sudo icy init home /home
sudo icy init data /data

# Use TUI to manage all at once
sudo icy
```

## 🤝 Contributing

Contributions welcome! Please feel free to submit issues and pull requests.

### Development

```bash
# Clone the repository
git clone https://github.com/yourusername/icy.git
cd icy

# Run in debug mode
cargo run

# Run with CLI args
cargo run -- list --config root

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy

# Build release
cargo build --release
```

## 📄 License

MIT License 

## 🙏 Acknowledgments

- Built with [Ratatui](https://github.com/ratatui-org/ratatui) for the TUI
- Inspired by [Snapper](https://github.com/openSUSE/snapper)
- Command-line parsing with [Clap](https://github.com/clap-rs/clap)

## 🔮 Roadmap

- [ ] Disk usage visualization in TUI
- [ ] File restore mode (restore specific files from snapshots)
- [ ] Snapshot scheduling (cron integration helper)
- [ ] REST API for remote management
- [ ] Compression support
- [ ] Snapshot encryption
- [ ] Email notifications
- [ ] Web UI
- [ ] ZFS support
- [ ] Differential backup to remote storage

## 📞 Support

- **Issues**: Report bugs on GitHub Issues
- **Documentation**: This README and inline help (`icy --help`)
- **Logs**: Check `/var/log/icy.log` for debugging

---

**Made with ❄️ by Abir Siddiky**

*Keep your system safe, one snapshot at a time!*
