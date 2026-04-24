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
- `src/app.rs` — `App` struct: state machine driving views (`PairList → FileTree → SyncPreview → SyncProgress`, plus `PasswordInput` and `Help` popup)
- `src/ui/` — one file per view, each with `draw()` and `handle_key()` called from `ui/mod.rs` dispatch
- `src/config.rs` — reads/writes `config/rusync/pairs.toml`; `CONFIG_DIR`/`PAIRS_FILE` are global constants
- `src/sync.rs` — wraps `rsync` CLI; `dry_run()`/`do_sync()` use `.output()` (blocking); `do_sync_interactive()` uses `expectrl` PTY for SSH password prompts; `feed_password()` sends password and collects output via `Captures::as_bytes()`
- `src/tree.rs` — builds `FileNode` tree from local dir via `walkdir`; `flatten_tree_for_display()` produces flat list for TUI rendering
- `src/cli.rs` — clap derive commands: `add`, `remove`, `list`, `sync <name>`

## Key Gotchas

- **`do_sync` must use `.output()` not `.spawn()`**: the `NamedTempFile` holding `--files-from` data must outlive the rsync process. `.spawn()` returns immediately and the temp file gets dropped/deleted before rsync reads it.
- **PTY output must come from `Captures::as_bytes()`, not `read_to_end()`**: `expect()` consumes data into its internal buffer. After `expect(Eof)` the stream is empty — calling `read_to_end` returns nothing. Always use `captures.as_bytes()` from the `expect` return value to collect output.
- **`expectrl::Expect` trait must be in scope**: `expect()`, `send_line()` are trait methods, not inherent on `Session`. Without `use expectrl::Expect` you get "method not found".
- **`expectrl::spawn()` returns `OsSession`**: the concrete type is `Session<OsProc, OsProcStream>`, aliased as `expectrl::session::OsSession`. Use this type alias in struct fields and function signatures.
- **`do_sync_interactive` uses a loop with `Any::boxed`**: matches `password`, `are you sure`, `fingerprint` patterns simultaneously. Host key prompts are auto-answered "yes" and the loop continues; password prompts return `NeedPassword`.
- **All rsync calls use `--rsh=ssh -o BatchMode=yes`** (via `build_args(batch_mode=true)`): prevents SSH password prompts from hanging the process. `do_sync_interactive` is the only path that omits it (uses PTY instead).
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

## Runtime Dependencies

- `rsync` must be on `$PATH`. Tests will fail if rsync is not installed.
- `expectrl` requires PTY support (Linux/macOS). When PTY is unavailable, `do_sync_interactive` falls back to an `AuthRequired` error with SSH key setup instructions.
