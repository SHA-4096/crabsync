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

A "pair" links a local directory to a remote target. Pairs are stored in `config/rusync/pairs.toml` relative to the project root.

```bash
# Add a sync pair
crabsync add myproject ./data user@server:/backup/data

# List configured pairs
crabsync list

# Remove a pair
crabsync remove myproject
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

## How It Works

1. **Select a pair** from the list and press Enter
2. **Browse and select** files in the source (left) or target (right) panel
3. **Press `s`** (upload) or **`d`** (download) to preview what will be synced
4. **Press `y`** to confirm and execute the rsync operation
5. If SSH authentication is required, a password prompt appears automatically

Upload syncs selected source files to the target. Download syncs selected target files to the source. The underlying rsync command swaps source and destination arguments accordingly.

## Configuration

Pairs are stored in `config/rusync/pairs.toml`:

```toml
[[pair]]
name = "myproject"
local = "./data"
remote = "user@server:/backup/data"
```

Remote targets use the `rsync` remote path syntax (`user@host:/path`). Local-to-local pairs are also supported — just use a local path as the remote target.

## Building

```bash
cargo build          # debug build
cargo build --release # release build
cargo test            # run tests
```

## License

MIT
