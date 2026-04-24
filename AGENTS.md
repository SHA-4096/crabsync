# AGENTS.md

## Build & Test

```bash
cargo build
cargo test
cargo build --release
```

No separate lint/typecheck commands configured. `cargo build` is the primary verification step.

## Architecture

Single-binary Rust TUI app for rsync file synchronization.

- `src/main.rs` — entrypoint + TUI event loop (crossterm/ratatui)
- `src/app.rs` — `App` struct: state machine driving 4 views (`PairList → FileTree → SyncPreview → SyncProgress`), plus a `Help` popup
- `src/ui/` — one file per view, each with `draw()` and `handle_key()` called from `ui/mod.rs` dispatch
- `src/config.rs` — reads/writes `config/rusync/pairs.toml`; `CONFIG_DIR`/`PAIRS_FILE` are global constants
- `src/sync.rs` — wraps `rsync` CLI via `std::process::Command`; `dry_run()` uses `-n` flag, `do_sync()` uses `.output()` (blocking) to keep `NamedTempFile` alive
- `src/tree.rs` — builds `FileNode` tree from local dir via `walkdir`; `flatten_tree_for_display()` produces flat list for TUI rendering
- `src/cli.rs` — clap derive commands: `add`, `remove`, `list`, `sync <name>`

## Key Gotchas

- **`do_sync` must use `.output()` not `.spawn()`**: the `NamedTempFile` holding `--files-from` data must outlive the rsync process. `.spawn()` returns immediately and the temp file gets dropped/deleted before rsync reads it.
- **Config path is relative to CWD**: `CONFIG_DIR` is `"config/rusync"` — run CLI commands from the repo root.
- **TUI cannot be tested in non-interactive terminals**: unit tests cover `sync.rs` only; TUI views have no automated tests.
- **`insert_node` in tree.rs**: uses index-based `position()` to avoid borrow checker issues with `iter_mut().find()` + push pattern.

## CLI Usage

```bash
rusync add <name> <local> <remote>   # positional args, not flags
rusync remove <name>
rusync list
rusync sync <name>                   # launches TUI directly into file tree
rusync                               # launches TUI at pair list
```

## Runtime Dependency

`rsync` must be on `$PATH`. Tests will fail if rsync is not installed.
