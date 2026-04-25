# crabsync

A terminal UI for rsync file synchronization. Browse, select, and sync files between local and remote directories with an intuitive split-pane interface.

## Features

- **Split-pane file tree** — browse source (local) and target (remote) directories side by side
- **Checkbox selection** — pick individual files or entire directories to sync
- **Upload & download** — push files to remote (`s`) or pull files from remote (`d`)
- **Dry-run preview** — review exactly what rsync will do before confirming
- **SSH password support** — interactive password prompt via PTY when SSH keys aren't configured
- **Wrong password retry** — automatically re-prompts on incorrect passwords instead of hanging
- **CLI management** — add, remove, and list sync pairs from the command line
- **Two-tier config** — local (`./crabsync.toml`) and global (`~/.config/crabsync/crabsync.toml`) config files; local pairs take priority over global pairs with the same name

## Requirements

- **rsync** must be on `$PATH`
- **SSH** for remote targets (key-based or password auth)
- Linux or macOS (PTY support required for interactive password flow)

## Installation

### From Source

```bash
git clone https://github.com/anomalyco/crabsync.git
cd crabsync
cargo build --release
```

The binary is at `target/release/crabsync`.

### From Release

Download a pre-built archive from [GitHub Releases](https://github.com/anomalyco/crabsync/releases):

| Platform | File |
|----------|------|
| Linux x86_64 | `crabsync-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 | `crabsync-aarch64-unknown-linux-gnu.tar.gz` |
| macOS x86_64 | `crabsync-x86_64-apple-darwin.tar.gz` |
| macOS ARM64 | `crabsync-aarch64-apple-darwin.tar.gz` |

```bash
tar xzf crabsync-*.tar.gz
chmod +x crabsync
sudo mv crabsync /usr/local/bin/
```

## Usage

### Managing Pairs

A "pair" links a local directory to a remote target. Pairs are stored in two config files:

- **Local**: `./crabsync.toml` (current directory, project-specific)
- **Global**: `~/.config/crabsync/crabsync.toml` (shared across projects)

Local pairs take priority — if the same name exists in both files, the global one is shadowed.

```bash
# Add a pair (local config by default)
crabsync add myproject ./data user@server:/backup/data

# Add a pair to global config
crabsync add myproject ./data user@server:/backup/data --global

# List configured pairs (shows scope: local/global)
crabsync list

# Remove a pair (tries local first, then global)
crabsync remove myproject

# Remove a pair from global config only
crabsync remove myproject --global
```

### Interactive TUI

```bash
# Launch the TUI at the pair list
crabsync

# Launch directly into a specific pair's file tree
crabsync sync myproject
```

### Key Bindings

#### Pair List

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Enter` | Enter file tree |
| `a` | Add pair |
| `d` | Delete pair |

#### File Tree

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down (active panel) |
| `k` / `↑` | Move up (active panel) |
| `Space` | Toggle file/directory selection |
| `Enter` | Expand/collapse directory |
| `a` | Select/deselect all |
| `s` | Dry-run upload (source panel) |
| `d` | Dry-run download (target panel) |
| `Tab` | Switch between source/target panel |
| `r` | Reload remote directory tree |
| `p` | Enter SSH password (when auth required) |
| `?` | Show help |
| `Esc` / `q` | Go back |

#### Sync Preview

| Key | Action |
|-----|--------|
| `y` | Confirm and execute sync |
| `n` / `Esc` | Go back |

#### Password Input

| Key | Action |
|-----|--------|
| `Enter` | Submit password |
| `Esc` | Cancel |

#### Add Pair

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous field |
| `Space` | Toggle scope (Local / Global) |
| `Enter` | Save pair |
| `Esc` | Cancel |

## How It Works

1. **Select a pair** from the list and press Enter
2. **Browse and select** files in the source (left) or target (right) panel
3. **Press `s`** (upload) or **`d`** (download) to preview what will be synced
4. **Press `y`** to confirm and execute the rsync operation
5. If SSH authentication is required, a password prompt appears automatically

Upload syncs selected source files to the target. Download syncs selected target files to the source. The underlying rsync command swaps source and destination arguments accordingly.

## Configuration

Pairs are stored in two TOML files with the same format:

**Local** (`./crabsync.toml`):
```toml
[[pair]]
name = "myproject"
local = "./data"
remote = "user@server:/backup/data"
```

**Global** (`~/.config/crabsync/crabsync.toml`):
```toml
[[pair]]
name = "photos"
local = "/home/user/photos"
remote = "nas:/backup/photos"
```

When both files contain a pair with the same name, the local one takes priority and the global one is shown as "shadowed" in the TUI. The global config directory and file are only created when you add your first global pair.

## Building

```bash
cargo build          # debug build
cargo build --release # release build
cargo test            # run tests
```

## License

MIT
