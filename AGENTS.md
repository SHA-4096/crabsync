# AGENTS.md

## Build & Test

```bash
cargo build
cargo test
cargo build --release
```

No separate lint/typecheck commands. `cargo build` is the primary verification step.

## Architecture

Single-binary Rust TUI app for rsync file synchronization.

- `src/main.rs` — entrypoint + TUI event loop (crossterm/ratatui)
- `src/app.rs` — `App` struct: state machine (`PairList → FileTree → SyncPreview → SyncProgress`, plus `PasswordInput`, `AddPair`, and `Help`). Holds two trees (`tree` for source, `remote_tree` for target), dual cursor tracking, `ActivePanel`/`RemoteStatus`/`PasswordContext`/`SyncDirection` enums for split-pane, password flow, and upload/download direction
- `src/ui/` — one file per view, each with `draw()` and `handle_key()` dispatched from `ui/mod.rs`
- `src/config.rs` — two-tier config: global (`~/.config/crabsync/crabsync.toml`) and local (`./crabsync.toml`). `TaggedPair` tracks source scope and shadow status. `load_all_pairs()` merges both with local-first priority; global pairs with same name as local are marked `shadowed`. Global config file/directory is only created when a global pair is first added.
- `src/sync.rs` — wraps `rsync` CLI:
  - `dry_run()`/`do_sync()`/`list_remote()` use `.output()` (blocking, BatchMode)
  - `run_rsync_pty()` is the shared PTY loop used by `do_sync_interactive()` and `list_remote_interactive()`
  - `feed_password()` also uses `Any::boxed` matching — returns `FeedPasswordPhase::NeedPassword` on wrong password so TUI can re-prompt
  - `list_remote()` uses `rsync -r --list-only` for remote directory listing
- `src/tree.rs` — `build_tree()` for local dirs via walkdir; `build_tree_from_listing()` parses `rsync --list-only` output into `FileNode` tree; `is_local_path()` detects local vs remote targets
- `src/cli.rs` — clap derive commands: `add`, `remove`, `list`, `sync <name>`

## Key Gotchas

- **`do_sync` must use `.output()` not `.spawn()`**: the `NamedTempFile` holding `--files-from` data must outlive the rsync process. `.spawn()` returns immediately and the temp file gets dropped/deleted before rsync reads it.
- **PTY output must come from `Captures::as_bytes()`, not `read_to_end()`**: `expect()` consumes data into its internal buffer. After `expect(Eof)` the stream is empty. Always use `captures.as_bytes()` from the `expect` return value.
- **`expectrl::Expect` trait must be in scope**: `expect()`, `send_line()` are trait methods, not inherent on `Session`. Without `use expectrl::Expect` you get "method not found".
- **`expectrl::spawn()` returns `OsSession`**: concrete type is `Session<OsProc, OsProcStream>`, aliased as `expectrl::session::OsSession`. Use this alias in struct fields and function signatures.
- **All three PTY paths use `Any::boxed` matching**: `run_rsync_pty()`, `feed_password()`, and the drain helpers all match `password`/`are you sure`/`fingerprint` simultaneously. Host key prompts are auto-answered "yes"; password prompts return `NeedPassword`.
- **Wrong password does NOT hang**: `feed_password()` returns `FeedPasswordPhase::NeedPassword` when SSH re-prompts, so the TUI can re-show the password dialog. Never use `expect(Eof)` after sending a password — a wrong password means the process is still alive waiting for input.
- **All rsync calls use `--rsh=ssh -o BatchMode=yes`** (via `build_args(batch_mode=true)`): prevents SSH password prompts from hanging the process. `do_sync_interactive` and `list_remote_interactive` are the only paths that omit it (use PTY instead).
- **Config path is two-tier**: global `~/.config/crabsync/crabsync.toml`, local `./crabsync.toml`. `load_all_pairs()` merges with local-first priority. Global config dir/file is only created when a global pair is first added. Removing the last pair from a config file deletes the file (and dir for global).
- **Pair names are unique per scope**: same name can exist in both global and local; local shadows global. `TaggedPair.shadowed` marks the global duplicate as dimmed in the TUI.
- **`PairSource` is `Copy`**: `enum PairSource { Global, Local }` derives `Copy` to avoid move issues when passing to `remove_pair()` and matching on it.
- **`insert_node` in tree.rs**: uses index-based `position()` to avoid borrow checker issues with `iter_mut().find()` + push pattern.
- **Local vs remote target detection**: `is_local_path()` returns true if path has no `:` or starts with `/`. Local targets use `build_tree()` (walkdir, instant); remote targets use `rsync --list-only` (subprocess, may need auth).
- **TUI cannot be tested in non-interactive terminals**: unit tests cover `sync.rs` and `tree.rs` only; TUI views have no automated tests.
- **`Regex` is a tuple struct in expectrl**: `expectrl::Regex("(?i)password")` not `Regex::new()`.

## CLI Usage

```bash
crabsync add <name> <local> <remote>   # adds to local config
crabsync add <name> <local> <remote> --global  # adds to global config
crabsync remove <name>                  # removes from local, then global as fallback
crabsync remove <name> --global         # removes from global only
crabsync list                           # shows scope indicators (local/global)
crabsync sync <name>                    # launches TUI directly into file tree
crabsync                                # launches TUI at pair list
```

## TUI Key Bindings

### Pair List
- `j/k` or arrows — move cursor
- `Enter` — enter file tree
- `a` — add pair (opens AddPair form)
- `d` — delete pair (from its source config)

### Add Pair
- `Tab` / `Shift+Tab` — next/previous field
- `Space` — toggle scope (Local/Global)
- `Enter` — save
- `Esc` — cancel

### File Tree

- `j/k` or arrows — move cursor (in active panel)
- `Space` — toggle file selection (both panels)
- `Enter` — expand/collapse dir (both panels)
- `a` — select/deselect all (both panels)
- `s` — dry-run sync / upload (source panel)
- `d` — dry-run download (target panel, only when remote tree loaded)
- `Tab` — switch source/target panel
- `r` — reload remote tree
- `p` — enter password for auth-required remote

## Release

Push a `v*` tag to trigger the GitHub Actions release workflow (`.github/workflows/release.yml`):

```bash
git tag v0.1.0
git push origin v0.1.0
```

Builds 4 targets in parallel and uploads to GitHub Release:
- `crabsync-x86_64-unknown-linux-gnu.tar.gz`
- `crabsync-aarch64-unknown-linux-gnu.tar.gz`
- `crabsync-x86_64-apple-darwin.tar.gz`
- `crabsync-aarch64-apple-darwin.tar.gz`

**Naming**: GitHub repo, Cargo package, binary, and release artifacts are all named **crabsync**.

## Runtime Dependencies

- `rsync` must be on `$PATH`. Tests will fail if rsync is not installed.
- `expectrl` requires PTY support (Linux/macOS). When PTY is unavailable, interactive paths fall back to `AuthRequired` error with SSH key setup instructions.
