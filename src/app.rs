use anyhow::{Context, Result};
use std::path::Path;

use crate::config::Pair;
use crate::sync::{self, SyncError, SyncPhase};
use crate::tree::{self, FileNode};

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    PairList,
    FileTree,
    SyncPreview,
    SyncProgress,
    PasswordInput,
    Help,
}

pub struct App {
    pub mode: Mode,
    pub previous_mode: Option<Mode>,
    pub pairs: Vec<Pair>,
    pub pair_index: usize,
    pub current_pair: Option<Pair>,
    pub tree: Option<FileNode>,
    pub tree_items: Vec<(usize, FileNode)>,
    pub tree_cursor: usize,
    pub tree_scroll: usize,
    pub dry_run_output: String,
    pub sync_command: String,
    pub sync_output: String,
    pub sync_error: bool,
    pub status_msg: String,
    pub should_quit: bool,
    pub select_all: bool,
    pub pty_session: Option<(expectrl::session::OsSession, Vec<u8>)>,
    pub password_buffer: String,
}

impl App {
    pub fn new(initial_pair_name: Option<&str>) -> Result<Self> {
        let pairs = crate::config::load_pairs().unwrap_or_default();

        if let Some(name) = initial_pair_name {
            if let Some(pair) = pairs.iter().find(|p| p.name == name).cloned() {
                let tree = tree::build_tree(Path::new(&pair.local))
                    .with_context(|| format!("failed to build tree for '{}'", pair.local))?;
                let mut items = Vec::new();
                tree::flatten_tree_for_display(&tree, 0, &mut items);

                return Ok(Self {
                    mode: Mode::FileTree,
                    previous_mode: None,
                    pairs,
                    pair_index: 0,
                    current_pair: Some(pair),
                    tree: Some(tree),
                    tree_items: items,
                    tree_cursor: 0,
                    tree_scroll: 0,
                    dry_run_output: String::new(),
                    sync_command: String::new(),
                    sync_output: String::new(),
                    sync_error: false,
                    status_msg: String::new(),
                    should_quit: false,
                    select_all: false,
                    pty_session: None,
                    password_buffer: String::new(),
                });
            } else {
                anyhow::bail!("pair '{}' not found", name);
            }
        }

        Ok(Self {
            mode: Mode::PairList,
            previous_mode: None,
            pairs,
            pair_index: 0,
            current_pair: None,
            tree: None,
            tree_items: Vec::new(),
            tree_cursor: 0,
            tree_scroll: 0,
            dry_run_output: String::new(),
            sync_command: String::new(),
            sync_output: String::new(),
            sync_error: false,
            status_msg: String::new(),
            should_quit: false,
            select_all: false,
            pty_session: None,
            password_buffer: String::new(),
        })
    }

    pub fn enter_file_tree(&mut self) -> Result<()> {
        let pair = self.pairs.get(self.pair_index).cloned();
        if let Some(pair) = pair {
            let tree = tree::build_tree(Path::new(&pair.local))
                .with_context(|| format!("failed to build tree for '{}'", pair.local))?;
            let mut items = Vec::new();
            tree::flatten_tree_for_display(&tree, 0, &mut items);

            self.current_pair = Some(pair);
            self.tree = Some(tree);
            self.tree_items = items;
            self.tree_cursor = 0;
            self.tree_scroll = 0;
            self.mode = Mode::FileTree;
            self.status_msg.clear();
        }
        Ok(())
    }

    pub fn toggle_tree_item(&mut self) {
        if self.tree_items.is_empty() {
            return;
        }
        let cursor = self.tree_cursor;
        if let Some(item) = self.tree_items.get(cursor) {
            let rel_path = item.1.relative_path.clone();
            let is_dir = item.1.is_dir;
            if let Some(tree) = &mut self.tree {
                toggle_node_by_path(tree, &rel_path, is_dir, false);
                self.tree_items.clear();
                tree::flatten_tree_for_display(tree, 0, &mut self.tree_items);
            }
        }
    }

    pub fn toggle_expand(&mut self) {
        if self.tree_items.is_empty() {
            return;
        }
        let cursor = self.tree_cursor;
        if let Some(item) = self.tree_items.get(cursor) {
            let rel_path = item.1.relative_path.clone();
            if let Some(tree) = &mut self.tree {
                toggle_node_by_path(tree, &rel_path, true, true);
                self.tree_items.clear();
                tree::flatten_tree_for_display(tree, 0, &mut self.tree_items);
            }
        }
    }

    pub fn toggle_select_all(&mut self) {
        self.select_all = !self.select_all;
        if let Some(tree) = &mut self.tree {
            tree.select_all(self.select_all);
            self.tree_items.clear();
            tree::flatten_tree_for_display(tree, 0, &mut self.tree_items);
        }
    }

    pub fn do_dry_run(&mut self) {
        if let (Some(tree), Some(pair)) = (&self.tree, &self.current_pair) {
            let files = tree.collect_selected();
            if files.is_empty() {
                self.status_msg = "no files selected".to_string();
                return;
            }
            match sync::dry_run(&pair.local, &pair.remote, &files) {
                Ok(output) => {
                    self.dry_run_output = output;
                    self.sync_command =
                        sync::build_command_display(&pair.local, &pair.remote, &files);
                    self.mode = Mode::SyncPreview;
                    self.status_msg.clear();
                }
                Err(SyncError::AuthRequired) => {
                    self.dry_run_output =
                        "Authentication required: SSH key not configured for this remote.\n\n\
                         You can still proceed — a password prompt will appear when you confirm sync."
                            .to_string();
                    self.sync_command =
                        sync::build_command_display(&pair.local, &pair.remote, &files);
                    self.mode = Mode::SyncPreview;
                    self.status_msg.clear();
                }
                Err(SyncError::Other(msg)) => {
                    self.status_msg = format!("dry-run error: {}", msg);
                }
            }
        }
    }

    pub fn do_sync(&mut self) {
        if let (Some(tree), Some(pair)) = (&self.tree, &self.current_pair) {
            let files = tree.collect_selected();
            if files.is_empty() {
                self.status_msg = "no files selected".to_string();
                return;
            }
            match sync::do_sync(&pair.local, &pair.remote, &files) {
                Ok(result) => {
                    self.sync_output = result.output;
                    self.sync_error = !result.success;
                    self.mode = Mode::SyncProgress;
                    self.status_msg.clear();
                }
                Err(SyncError::AuthRequired) => {
                    let files_clone = files.clone();
                    let local = pair.local.clone();
                    let remote = pair.remote.clone();
                    match sync::do_sync_interactive(&local, &remote, &files_clone) {
                        Ok(SyncPhase::NeedPassword((session, pre_output))) => {
                            self.pty_session = Some((session, pre_output));
                            self.password_buffer.clear();
                            self.mode = Mode::PasswordInput;
                        }
                        Ok(SyncPhase::Done(result)) => {
                            self.sync_output = result.output;
                            self.sync_error = !result.success;
                            self.mode = Mode::SyncProgress;
                            self.status_msg.clear();
                        }
                        Err(SyncError::AuthRequired) => {
                            self.show_auth_fallback();
                        }
                        Err(SyncError::Other(msg)) => {
                            self.sync_output = msg;
                            self.sync_error = true;
                            self.mode = Mode::SyncProgress;
                        }
                    }
                }
                Err(SyncError::Other(msg)) => {
                    self.sync_output = msg;
                    self.sync_error = true;
                    self.mode = Mode::SyncProgress;
                }
            }
        }
    }

    pub fn submit_password(&mut self) {
        if let Some((session, pre_output)) = self.pty_session.take() {
            let password = std::mem::take(&mut self.password_buffer);
            match sync::feed_password(session, pre_output, &password) {
                Ok(result) => {
                    self.sync_output = result.output;
                    self.sync_error = !result.success;
                    self.mode = Mode::SyncProgress;
                }
                Err(e) => {
                    self.sync_output = e.to_string();
                    self.sync_error = true;
                    self.mode = Mode::SyncProgress;
                }
            }
            let len = password.len();
            drop(password);
            self.password_buffer = "\0".repeat(len);
            self.password_buffer.clear();
        }
    }

    pub fn cancel_password(&mut self) {
        self.pty_session = None;
        self.password_buffer.clear();
        self.mode = Mode::SyncPreview;
        self.status_msg = "sync cancelled".to_string();
    }

    fn show_auth_fallback(&mut self) {
        self.sync_output = format!(
            "Authentication failed: SSH key not configured and interactive\n\
             terminal (PTY) is not available in this environment.\n\n\
             Please configure SSH key-based authentication:\n\n\
               ssh-keygen -t ed25519\n\
               ssh-copy-id <remote>"
        );
        self.sync_error = true;
        self.mode = Mode::SyncProgress;
    }

    pub fn tree_cursor_down(&mut self) {
        if !self.tree_items.is_empty() && self.tree_cursor < self.tree_items.len() - 1 {
            self.tree_cursor += 1;
        }
    }

    pub fn tree_cursor_up(&mut self) {
        if self.tree_cursor > 0 {
            self.tree_cursor -= 1;
        }
    }
}

fn toggle_node_by_path(
    node: &mut FileNode,
    target: &std::path::Path,
    is_dir_toggle: bool,
    toggle_expand: bool,
) {
    if node.relative_path == target {
        if toggle_expand && node.is_dir {
            node.toggle_expanded();
        } else {
            node.toggle_selected();
        }
        return;
    }
    if node.is_dir {
        for child in &mut node.children {
            toggle_node_by_path(child, target, is_dir_toggle, toggle_expand);
        }
    }
}
