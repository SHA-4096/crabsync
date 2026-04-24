use anyhow::{Context, Result};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

pub struct SyncResult {
    pub output: String,
    pub success: bool,
}

fn build_args(local: &str, remote: &str, files_from_path: &str, dry_run: bool) -> Vec<String> {
    let flags = if dry_run { "-avzn" } else { "-avz" };
    vec![
        flags.to_string(),
        "--relative".to_string(),
        format!("--files-from={}", files_from_path),
        format!("{}/", local),
        format!("{}/", remote),
    ]
}

pub fn build_command_display(local: &str, remote: &str, files: &[PathBuf]) -> String {
    let args = build_args(local, remote, "<file-list>", false);
    let mut cmd = format!("rsync {}", args.join(" "));
    if !files.is_empty() {
        cmd.push_str("\n\nFile list:");
        for f in files {
            cmd.push_str(&format!("\n  {}", f.display()));
        }
    }
    cmd
}

pub fn dry_run(local: &str, remote: &str, files: &[PathBuf]) -> Result<String> {
    let files_from = write_files_from(files)?;
    let args = build_args(
        local,
        remote,
        &files_from.path().display().to_string(),
        true,
    );
    let output = Command::new("rsync")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("failed to run rsync dry-run")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        anyhow::bail!("rsync dry-run failed:\n{}", stderr);
    }

    Ok(if stdout.is_empty() { stderr } else { stdout })
}

pub fn do_sync(local: &str, remote: &str, files: &[PathBuf]) -> Result<SyncResult> {
    let files_from = write_files_from(files)?;
    let args = build_args(
        local,
        remote,
        &files_from.path().display().to_string(),
        false,
    );
    let output = Command::new("rsync")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("failed to run rsync")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

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
}
