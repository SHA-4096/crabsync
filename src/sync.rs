use anyhow::{Context, Result};
use expectrl::Expect;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

pub struct SyncResult {
    pub output: String,
    pub success: bool,
}

#[derive(Debug)]
pub enum SyncError {
    AuthRequired,
    Other(String),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::AuthRequired => write!(f, "authentication required: SSH key not configured"),
            SyncError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

pub enum SyncPhase {
    NeedPassword((expectrl::session::OsSession, Vec<u8>)),
    Done(SyncResult),
}

pub enum ListPhase {
    NeedPassword((expectrl::session::OsSession, Vec<u8>)),
    Done(String),
}

enum InteractivePhase {
    NeedPassword((expectrl::session::OsSession, Vec<u8>)),
    Done(String),
}

fn run_rsync_pty(args: &[String]) -> std::result::Result<InteractivePhase, SyncError> {
    let cmd_str = format!("rsync {}", args.join(" "));
    let mut session = expectrl::spawn(&cmd_str).map_err(|e| {
        SyncError::Other(format!(
            "PTY not available: {}. Please configure SSH key-based authentication.",
            e
        ))
    })?;

    session.set_expect_timeout(Some(std::time::Duration::from_secs(30)));

    let mut pre_output = Vec::new();

    loop {
        match session.expect(expectrl::Any::boxed(vec![
            Box::new(expectrl::Regex("(?i)password")),
            Box::new(expectrl::Regex("(?i)are you sure")),
            Box::new(expectrl::Regex("(?i)fingerprint")),
        ])) {
            Ok(captures) => {
                let matched = String::from_utf8_lossy(captures.as_bytes());
                pre_output.extend_from_slice(captures.as_bytes());

                if matched.to_lowercase().contains("password") {
                    return Ok(InteractivePhase::NeedPassword((session, pre_output)));
                } else {
                    session.send_line("yes").map_err(|e| {
                        SyncError::Other(format!("failed to send host key confirmation: {}", e))
                    })?;
                    continue;
                }
            }
            Err(expectrl::Error::Eof) => {
                let output = drain_session_with_prefix(session, &pre_output);
                return Ok(InteractivePhase::Done(output));
            }
            Err(expectrl::Error::ExpectTimeout) => {
                let output = drain_session_with_prefix(session, &pre_output);
                return Ok(InteractivePhase::Done(output));
            }
            Err(e) => return Err(SyncError::Other(format!("unexpected rsync output: {}", e))),
        }
    }
}

fn build_args(
    local: &str,
    remote: &str,
    files_from_path: &str,
    dry_run: bool,
    batch_mode: bool,
) -> Vec<String> {
    let flags = if dry_run { "-avzn" } else { "-avz" };
    let mut args = vec![
        flags.to_string(),
        "--relative".to_string(),
        format!("--files-from={}", files_from_path),
    ];
    if batch_mode {
        args.push(r#"--rsh=ssh -o BatchMode=yes"#.to_string());
    }
    args.push(format!("{}/", local));
    args.push(format!("{}/", remote));
    args
}

fn is_auth_error(stderr: &str) -> bool {
    let lower = stderr.to_lowercase();
    lower.contains("permission denied")
        || lower.contains("authentication")
        || lower.contains("auth fail")
        || lower.contains("host key verification failed")
}

pub fn build_command_display(local: &str, remote: &str, files: &[PathBuf]) -> String {
    let args = build_args(local, remote, "<file-list>", false, true);
    let mut cmd = format!("rsync {}", args.join(" "));
    if !files.is_empty() {
        cmd.push_str("\n\nFile list:");
        for f in files {
            cmd.push_str(&format!("\n  {}", f.display()));
        }
    }
    cmd
}

pub fn dry_run(
    local: &str,
    remote: &str,
    files: &[PathBuf],
) -> std::result::Result<String, SyncError> {
    let files_from = write_files_from(files).map_err(|e| SyncError::Other(e.to_string()))?;
    let args = build_args(
        local,
        remote,
        &files_from.path().display().to_string(),
        true,
        true,
    );
    let output = Command::new("rsync")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| SyncError::Other(format!("failed to run rsync dry-run: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        if is_auth_error(&stderr) {
            return Err(SyncError::AuthRequired);
        }
        return Err(SyncError::Other(format!(
            "rsync dry-run failed:\n{}",
            stderr
        )));
    }

    Ok(if stdout.is_empty() { stderr } else { stdout })
}

pub fn do_sync(
    local: &str,
    remote: &str,
    files: &[PathBuf],
) -> std::result::Result<SyncResult, SyncError> {
    let files_from = write_files_from(files).map_err(|e| SyncError::Other(e.to_string()))?;
    let args = build_args(
        local,
        remote,
        &files_from.path().display().to_string(),
        false,
        true,
    );
    let output = Command::new("rsync")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| SyncError::Other(format!("failed to run rsync: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    if !success && is_auth_error(&stderr) {
        return Err(SyncError::AuthRequired);
    }

    let combined = if stdout.is_empty() {
        stderr
    } else if stderr.is_empty() {
        stdout
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    Ok(SyncResult {
        output: combined,
        success,
    })
}

pub fn do_sync_interactive(
    local: &str,
    remote: &str,
    files: &[PathBuf],
) -> std::result::Result<SyncPhase, SyncError> {
    let files_from = write_files_from(files).map_err(|e| SyncError::Other(e.to_string()))?;
    let args = build_args(
        local,
        remote,
        &files_from.path().display().to_string(),
        false,
        false,
    );

    match run_rsync_pty(&args)? {
        InteractivePhase::NeedPassword((session, pre_output)) => {
            Ok(SyncPhase::NeedPassword((session, pre_output)))
        }
        InteractivePhase::Done(output) => Ok(SyncPhase::Done(SyncResult {
            output,
            success: true,
        })),
    }
}

pub fn list_remote(remote: &str) -> std::result::Result<String, SyncError> {
    let mut args = vec![
        "-r".to_string(),
        "--list-only".to_string(),
        r#"--rsh=ssh -o BatchMode=yes"#.to_string(),
    ];
    args.push(format!("{}/", remote));

    let output = Command::new("rsync")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| SyncError::Other(format!("failed to run rsync --list-only: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        if is_auth_error(&stderr) {
            return Err(SyncError::AuthRequired);
        }
        return Err(SyncError::Other(format!(
            "rsync --list-only failed:\n{}",
            stderr
        )));
    }

    Ok(if stdout.is_empty() { stderr } else { stdout })
}

pub fn list_remote_interactive(remote: &str) -> std::result::Result<ListPhase, SyncError> {
    let mut args = vec!["-r".to_string(), "--list-only".to_string()];
    args.push(format!("{}/", remote));

    match run_rsync_pty(&args)? {
        InteractivePhase::NeedPassword((session, pre_output)) => {
            Ok(ListPhase::NeedPassword((session, pre_output)))
        }
        InteractivePhase::Done(output) => Ok(ListPhase::Done(output)),
    }
}

pub enum FeedPasswordPhase {
    NeedPassword((expectrl::session::OsSession, Vec<u8>)),
    Done(SyncResult),
}

pub fn feed_password(
    mut session: expectrl::session::OsSession,
    mut pre_output: Vec<u8>,
    password: &str,
) -> std::result::Result<FeedPasswordPhase, SyncError> {
    session
        .send_line(password)
        .map_err(|e| SyncError::Other(format!("failed to send password: {}", e)))?;

    loop {
        match session.expect(expectrl::Any::boxed(vec![
            Box::new(expectrl::Regex("(?i)password")),
            Box::new(expectrl::Regex("(?i)are you sure")),
            Box::new(expectrl::Regex("(?i)fingerprint")),
        ])) {
            Ok(captures) => {
                let matched = String::from_utf8_lossy(captures.as_bytes());
                pre_output.extend_from_slice(captures.as_bytes());

                if matched.to_lowercase().contains("password") {
                    return Ok(FeedPasswordPhase::NeedPassword((session, pre_output)));
                } else {
                    session.send_line("yes").map_err(|e| {
                        SyncError::Other(format!("failed to send host key confirmation: {}", e))
                    })?;
                    continue;
                }
            }
            Err(expectrl::Error::Eof) => {
                let output = drain_session_with_prefix(session, &pre_output);
                let success = !output.to_lowercase().contains("permission denied")
                    && !output.to_lowercase().contains("authentication");
                return Ok(FeedPasswordPhase::Done(SyncResult { output, success }));
            }
            Err(expectrl::Error::ExpectTimeout) => {
                let mut tmp = [0u8; 4096];
                let n = session.try_read(&mut tmp).unwrap_or(0);
                if n > 0 {
                    pre_output.extend_from_slice(&tmp[..n]);
                }
                continue;
            }
            Err(e) => {
                return Err(SyncError::Other(format!(
                    "error waiting for rsync to finish: {}",
                    e
                )));
            }
        }
    }
}

fn drain_session_with_prefix(mut session: expectrl::session::OsSession, prefix: &[u8]) -> String {
    let mut all = prefix.to_vec();
    loop {
        match session.expect(expectrl::Eof) {
            Ok(captures) => {
                all.extend_from_slice(captures.as_bytes());
                break;
            }
            Err(expectrl::Error::ExpectTimeout) => continue,
            _ => break,
        }
    }
    String::from_utf8_lossy(&all).to_string()
}

fn write_files_from(files: &[PathBuf]) -> Result<NamedTempFile> {
    let mut tmp = NamedTempFile::new().context("failed to create temp file")?;
    for f in files {
        writeln!(tmp, "{}", f.display()).context("failed to write to temp file")?;
    }
    tmp.flush().context("failed to flush temp file")?;
    Ok(tmp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_dirs() -> (tempfile::TempDir, tempfile::TempDir) {
        let local = tempfile::TempDir::new().unwrap();
        let remote = tempfile::TempDir::new().unwrap();

        fs::write(local.path().join("file1.txt"), "hello").unwrap();
        fs::create_dir(local.path().join("subdir")).unwrap();
        fs::write(local.path().join("subdir/file2.txt"), "world").unwrap();

        (local, remote)
    }

    #[test]
    fn test_dry_run_reports_files_to_sync() {
        let (local, remote) = setup_test_dirs();

        let files = vec![
            PathBuf::from("file1.txt"),
            PathBuf::from("subdir/file2.txt"),
        ];

        let output = dry_run(
            local.path().to_str().unwrap(),
            remote.path().to_str().unwrap(),
            &files,
        )
        .unwrap();

        assert!(
            output.contains("file1.txt"),
            "dry-run should mention file1.txt"
        );
        assert!(
            output.contains("file2.txt"),
            "dry-run should mention file2.txt"
        );

        assert!(
            !remote.path().join("file1.txt").exists(),
            "dry-run should not actually copy files"
        );
    }

    #[test]
    fn test_do_sync_copies_files() {
        let (local, remote) = setup_test_dirs();

        let files = vec![
            PathBuf::from("file1.txt"),
            PathBuf::from("subdir/file2.txt"),
        ];

        let result = do_sync(
            local.path().to_str().unwrap(),
            remote.path().to_str().unwrap(),
            &files,
        )
        .unwrap();

        assert!(result.success, "sync should succeed");
        assert!(
            remote.path().join("file1.txt").exists(),
            "file1.txt should exist in remote"
        );
        assert!(
            remote.path().join("subdir/file2.txt").exists(),
            "subdir/file2.txt should exist in remote"
        );

        let content = fs::read_to_string(remote.path().join("file1.txt")).unwrap();
        assert_eq!(content, "hello");
        let content = fs::read_to_string(remote.path().join("subdir/file2.txt")).unwrap();
        assert_eq!(content, "world");
    }

    #[test]
    fn test_do_sync_preserves_directory_structure() {
        let (local, remote) = setup_test_dirs();

        let files = vec![PathBuf::from("subdir/file2.txt")];

        let result = do_sync(
            local.path().to_str().unwrap(),
            remote.path().to_str().unwrap(),
            &files,
        )
        .unwrap();

        assert!(result.success);
        assert!(remote.path().join("subdir").is_dir());
        assert!(remote.path().join("subdir/file2.txt").exists());
        assert!(
            !remote.path().join("file1.txt").exists(),
            "unselected file should not be synced"
        );
    }

    #[test]
    fn test_dry_run_with_empty_files_list() {
        let (local, remote) = setup_test_dirs();

        let result = dry_run(
            local.path().to_str().unwrap(),
            remote.path().to_str().unwrap(),
            &[],
        );

        assert!(
            result.is_ok(),
            "dry-run with empty file list should not error"
        );
    }

    #[test]
    fn test_do_sync_idempotent() {
        let (local, remote) = setup_test_dirs();

        let files = vec![PathBuf::from("file1.txt")];

        let result1 = do_sync(
            local.path().to_str().unwrap(),
            remote.path().to_str().unwrap(),
            &files,
        )
        .unwrap();
        assert!(result1.success);

        let result2 = do_sync(
            local.path().to_str().unwrap(),
            remote.path().to_str().unwrap(),
            &files,
        )
        .unwrap();
        assert!(result2.success, "second sync should also succeed");

        let content = fs::read_to_string(remote.path().join("file1.txt")).unwrap();
        assert_eq!(
            content, "hello",
            "content should remain the same after re-sync"
        );
    }

    #[test]
    fn test_do_sync_nonexistent_local_dir() {
        let remote = tempfile::TempDir::new().unwrap();

        let files = vec![PathBuf::from("file1.txt")];

        let result = do_sync("/nonexistent/path", remote.path().to_str().unwrap(), &files);

        match result {
            Ok(r) => {
                assert!(
                    !r.success,
                    "sync should report failure for nonexistent local dir"
                );
            }
            Err(_) => {}
        }
    }

    #[test]
    fn test_write_files_from_creates_valid_file() {
        let files = vec![PathBuf::from("a.txt"), PathBuf::from("b/c.txt")];

        let tmp = write_files_from(&files).unwrap();
        let content = fs::read_to_string(tmp.path()).unwrap();

        assert_eq!(content, "a.txt\nb/c.txt\n");
    }

    #[test]
    fn test_is_auth_error() {
        assert!(is_auth_error("Permission denied (publickey,password)."));
        assert!(is_auth_error("Authentication failed."));
        assert!(is_auth_error("Host key verification failed."));
        assert!(!is_auth_error("No such file or directory"));
        assert!(!is_auth_error("rsync: write error"));
    }

    #[test]
    fn test_build_args_includes_batch_mode() {
        let args = build_args("/local", "/remote", "/tmp/files", false, true);
        assert!(args.contains(&r#"--rsh=ssh -o BatchMode=yes"#.to_string()));
    }

    #[test]
    fn test_build_args_without_batch_mode() {
        let args = build_args("/local", "/remote", "/tmp/files", false, false);
        assert!(!args.iter().any(|a| a.contains("BatchMode")));
    }

    #[test]
    fn test_list_remote_local_dir() {
        let remote = tempfile::TempDir::new().unwrap();
        fs::write(remote.path().join("file1.txt"), "hello").unwrap();
        fs::create_dir(remote.path().join("subdir")).unwrap();
        fs::write(remote.path().join("subdir/file2.txt"), "world").unwrap();

        let output = list_remote(remote.path().to_str().unwrap()).unwrap();

        assert!(
            output.contains("file1.txt"),
            "list_remote should mention file1.txt"
        );
        assert!(
            output.contains("subdir"),
            "list_remote should mention subdir"
        );
    }

    #[test]
    fn test_list_remote_nonexistent_dir() {
        let result = list_remote("/nonexistent/path");
        assert!(
            result.is_err(),
            "list_remote should fail for nonexistent dir"
        );
    }

    #[test]
    fn test_do_sync_reverse_direction() {
        let local = tempfile::TempDir::new().unwrap();
        let remote = tempfile::TempDir::new().unwrap();

        fs::write(remote.path().join("file1.txt"), "from_remote").unwrap();
        fs::create_dir(remote.path().join("subdir")).unwrap();
        fs::write(remote.path().join("subdir/file2.txt"), "world").unwrap();

        let files = vec![
            PathBuf::from("file1.txt"),
            PathBuf::from("subdir/file2.txt"),
        ];

        let result = do_sync(
            remote.path().to_str().unwrap(),
            local.path().to_str().unwrap(),
            &files,
        )
        .unwrap();

        assert!(result.success, "reverse sync should succeed");
        assert!(
            local.path().join("file1.txt").exists(),
            "file1.txt should exist in local after download"
        );
        assert!(
            local.path().join("subdir/file2.txt").exists(),
            "subdir/file2.txt should exist in local after download"
        );

        let content = fs::read_to_string(local.path().join("file1.txt")).unwrap();
        assert_eq!(content, "from_remote");
    }
}
