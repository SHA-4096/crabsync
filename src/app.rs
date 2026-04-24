use anyhow::{Context, Result};
use std::path::Path;

use crate::config::Pair;
use crate::sync::{self, FeedPasswordPhase, ListPhase, SyncError, SyncPhase};
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

#[derive(Debug, Clone, PartialEq)]
pub enum ActivePanel {
    Source,
    Target,
}

#[derive(Debug, Clone)]
pub enum RemoteStatus {
    NotLoaded,
    Loading,
    Loaded,
    AuthRequired,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PasswordContext {
    Sync,
    RemoteList,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyncDirection {
    Upload,
    Download,
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
    pub active_panel: ActivePanel,
    pub remote_tree: Option<FileNode>,
    pub remote_tree_items: Vec<(usize, FileNode)>,
    pub remote_tree_cursor: usize,
    pub remote_status: RemoteStatus,
    pub password_context: PasswordContext,
    pub sync_direction: SyncDirection,
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

                let mut app = Self {
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
                    active_panel: ActivePanel::Source,
                    remote_tree: None,
                    remote_tree_items: Vec::new(),
                    remote_tree_cursor: 0,
                    remote_status: RemoteStatus::NotLoaded,
                    password_context: PasswordContext::Sync,
                    sync_direction: SyncDirection::Upload,
                };
                app.load_remote_tree();
                return Ok(app);
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
            active_panel: ActivePanel::Source,
            remote_tree: None,
            remote_tree_items: Vec::new(),
            remote_tree_cursor: 0,
            remote_status: RemoteStatus::NotLoaded,
            password_context: PasswordContext::Sync,
            sync_direction: SyncDirection::Upload,
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
            self.active_panel = ActivePanel::Source;
            self.remote_tree = None;
            self.remote_tree_items = Vec::new();
            self.remote_tree_cursor = 0;
            self.remote_status = RemoteStatus::NotLoaded;
            self.mode = Mode::FileTree;
            self.status_msg.clear();
            self.load_remote_tree();
        }
        Ok(())
    }

    pub fn load_remote_tree(&mut self) {
        if let Some(pair) = &self.current_pair {
            let remote = pair.remote.clone();
            if tree::is_local_path(&remote) {
                match tree::build_tree(Path::new(&remote)) {
                    Ok(tree) => {
                        let mut items = Vec::new();
                        tree::flatten_tree_for_display(&tree, 0, &mut items);
                        self.remote_tree = Some(tree);
                        self.remote_tree_items = items;
                        self.remote_tree_cursor = 0;
                        self.remote_status = RemoteStatus::Loaded;
                    }
                    Err(e) => {
                        self.remote_status = RemoteStatus::Error(e.to_string());
                    }
                }
            } else {
                self.remote_status = RemoteStatus::Loading;
                match sync::list_remote(&remote) {
                    Ok(output) => {
                        let tree = tree::build_tree_from_listing(&output);
                        let mut items = Vec::new();
                        tree::flatten_tree_for_display(&tree, 0, &mut items);
                        self.remote_tree = Some(tree);
                        self.remote_tree_items = items;
                        self.remote_tree_cursor = 0;
                        self.remote_status = RemoteStatus::Loaded;
                    }
                    Err(SyncError::AuthRequired) => {
                        self.remote_status = RemoteStatus::AuthRequired;
                    }
                    Err(SyncError::Other(msg)) => {
                        self.remote_status = RemoteStatus::Error(msg);
                    }
                }
            }
        }
    }

    pub fn load_remote_interactive(&mut self) {
        if let Some(pair) = &self.current_pair {
            let remote = pair.remote.clone();
            match sync::list_remote_interactive(&remote) {
                Ok(ListPhase::NeedPassword((session, pre_output))) => {
                    self.pty_session = Some((session, pre_output));
                    self.password_buffer.clear();
                    self.password_context = PasswordContext::RemoteList;
                    self.mode = Mode::PasswordInput;
                }
                Ok(ListPhase::Done(output)) => {
                    let tree = tree::build_tree_from_listing(&output);
                    let mut items = Vec::new();
                    tree::flatten_tree_for_display(&tree, 0, &mut items);
                    self.remote_tree = Some(tree);
                    self.remote_tree_items = items;
                    self.remote_tree_cursor = 0;
                    self.remote_status = RemoteStatus::Loaded;
                }
                Err(SyncError::AuthRequired) => {
                    self.remote_status = RemoteStatus::AuthRequired;
                }
                Err(SyncError::Other(msg)) => {
                    self.remote_status = RemoteStatus::Error(format!(
                        "PTY unavailable: {}. Please configure SSH key-based authentication.",
                        msg
                    ));
                }
            }
        }
    }

    pub fn toggle_panel(&mut self) {
        self.active_panel = match self.active_panel {
            ActivePanel::Source => ActivePanel::Target,
            ActivePanel::Target => ActivePanel::Source,
        };
    }

    pub fn remote_tree_cursor_down(&mut self) {
        if !self.remote_tree_items.is_empty()
            && self.remote_tree_cursor < self.remote_tree_items.len() - 1
        {
            self.remote_tree_cursor += 1;
        }
    }

    pub fn remote_tree_cursor_up(&mut self) {
        if self.remote_tree_cursor > 0 {
            self.remote_tree_cursor -= 1;
        }
    }

    pub fn toggle_expand_remote(&mut self) {
        if self.remote_tree_items.is_empty() {
            return;
        }
        let cursor = self.remote_tree_cursor;
        if let Some(item) = self.remote_tree_items.get(cursor) {
            let rel_path = item.1.relative_path.clone();
            if let Some(tree) = &mut self.remote_tree {
                toggle_node_by_path(tree, &rel_path, true, true);
                self.remote_tree_items.clear();
                tree::flatten_tree_for_display(tree, 0, &mut self.remote_tree_items);
            }
        }
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

    pub fn toggle_remote_tree_item(&mut self) {
        if self.remote_tree_items.is_empty() || !matches!(self.remote_status, RemoteStatus::Loaded)
        {
            return;
        }
        let cursor = self.remote_tree_cursor;
        if let Some(item) = self.remote_tree_items.get(cursor) {
            let rel_path = item.1.relative_path.clone();
            let is_dir = item.1.is_dir;
            if let Some(tree) = &mut self.remote_tree {
                toggle_node_by_path(tree, &rel_path, is_dir, false);
                self.remote_tree_items.clear();
                tree::flatten_tree_for_display(tree, 0, &mut self.remote_tree_items);
            }
        }
    }

    pub fn toggle_select_all_remote(&mut self) {
        if !matches!(self.remote_status, RemoteStatus::Loaded) {
            return;
        }
        self.select_all = !self.select_all;
        if let Some(tree) = &mut self.remote_tree {
            tree.select_all(self.select_all);
            self.remote_tree_items.clear();
            tree::flatten_tree_for_display(tree, 0, &mut self.remote_tree_items);
        }
    }

    pub fn reload_local_tree(&mut self) {
        if let Some(pair) = &self.current_pair {
            if let Ok(tree) = tree::build_tree(Path::new(&pair.local)) {
                let mut items = Vec::new();
                tree::flatten_tree_for_display(&tree, 0, &mut items);
                self.tree = Some(tree);
                self.tree_items = items;
            }
        }
    }

    pub fn do_dry_run(&mut self) {
        self.sync_direction = SyncDirection::Upload;
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

    pub fn do_dry_run_reverse(&mut self) {
        if !matches!(self.remote_status, RemoteStatus::Loaded) {
            return;
        }
        self.sync_direction = SyncDirection::Download;
        if let (Some(remote_tree), Some(pair)) = (&self.remote_tree, &self.current_pair) {
            let files = remote_tree.collect_selected();
            if files.is_empty() {
                self.status_msg = "no files selected".to_string();
                return;
            }
            match sync::dry_run(&pair.remote, &pair.local, &files) {
                Ok(output) => {
                    self.dry_run_output = output;
                    self.sync_command =
                        sync::build_command_display(&pair.remote, &pair.local, &files);
                    self.mode = Mode::SyncPreview;
                    self.status_msg.clear();
                }
                Err(SyncError::AuthRequired) => {
                    self.dry_run_output =
                        "Authentication required: SSH key not configured for this remote.\n\n\
                         You can still proceed — a password prompt will appear when you confirm sync."
                            .to_string();
                    self.sync_command =
                        sync::build_command_display(&pair.remote, &pair.local, &files);
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
        let is_download = self.sync_direction == SyncDirection::Download;

        let (src, dst, files) = if is_download {
            let remote_tree = match &self.remote_tree {
                Some(t) => t,
                None => return,
            };
            let pair = match &self.current_pair {
                Some(p) => p,
                None => return,
            };
            let files = remote_tree.collect_selected();
            if files.is_empty() {
                self.status_msg = "no files selected".to_string();
                return;
            }
            (pair.remote.clone(), pair.local.clone(), files)
        } else {
            let tree = match &self.tree {
                Some(t) => t,
                None => return,
            };
            let pair = match &self.current_pair {
                Some(p) => p,
                None => return,
            };
            let files = tree.collect_selected();
            if files.is_empty() {
                self.status_msg = "no files selected".to_string();
                return;
            }
            (pair.local.clone(), pair.remote.clone(), files)
        };

        match sync::do_sync(&src, &dst, &files) {
            Ok(result) => {
                self.sync_output = result.output;
                self.sync_error = !result.success;
                self.mode = Mode::SyncProgress;
                self.status_msg.clear();
                if result.success {
                    if is_download {
                        self.reload_local_tree();
                    } else {
                        self.load_remote_tree();
                    }
                }
            }
            Err(SyncError::AuthRequired) => {
                let files_clone = files.clone();
                let src_c = src.clone();
                let dst_c = dst.clone();
                match sync::do_sync_interactive(&src_c, &dst_c, &files_clone) {
                    Ok(SyncPhase::NeedPassword((session, pre_output))) => {
                        self.pty_session = Some((session, pre_output));
                        self.password_buffer.clear();
                        self.password_context = PasswordContext::Sync;
                        self.mode = Mode::PasswordInput;
                    }
                    Ok(SyncPhase::Done(result)) => {
                        self.sync_output = result.output;
                        self.sync_error = !result.success;
                        self.mode = Mode::SyncProgress;
                        self.status_msg.clear();
                        if result.success {
                            if is_download {
                                self.reload_local_tree();
                            } else {
                                self.load_remote_tree();
                            }
                        }
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

    pub fn submit_password(&mut self) {
        if let Some((session, pre_output)) = self.pty_session.take() {
            let password = std::mem::take(&mut self.password_buffer);
            let context = self.password_context.clone();

            match sync::feed_password(session, pre_output, &password) {
                Ok(FeedPasswordPhase::NeedPassword((session, pre_output))) => {
                    self.pty_session = Some((session, pre_output));
                    self.password_buffer.clear();
                    self.status_msg = "wrong password, try again".to_string();
                    self.mode = Mode::PasswordInput;
                }
                Ok(FeedPasswordPhase::Done(result)) => match context {
                    PasswordContext::Sync => {
                        self.sync_output = result.output;
                        self.sync_error = !result.success;
                        self.mode = Mode::SyncProgress;
                        if self.sync_direction == SyncDirection::Download {
                            self.reload_local_tree();
                        } else {
                            self.load_remote_tree();
                        }
                    }
                    PasswordContext::RemoteList => {
                        if result.success {
                            let tree = tree::build_tree_from_listing(&result.output);
                            let mut items = Vec::new();
                            tree::flatten_tree_for_display(&tree, 0, &mut items);
                            self.remote_tree = Some(tree);
                            self.remote_tree_items = items;
                            self.remote_tree_cursor = 0;
                            self.remote_status = RemoteStatus::Loaded;
                        } else {
                            self.remote_status = RemoteStatus::Error(result.output);
                        }
                        self.mode = Mode::FileTree;
                    }
                },
                Err(e) => match context {
                    PasswordContext::Sync => {
                        self.sync_output = e.to_string();
                        self.sync_error = true;
                        self.mode = Mode::SyncProgress;
                    }
                    PasswordContext::RemoteList => {
                        self.remote_status = RemoteStatus::Error(e.to_string());
                        self.mode = Mode::FileTree;
                    }
                },
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
        match self.password_context {
            PasswordContext::Sync => {
                self.mode = Mode::SyncPreview;
                self.status_msg = "sync cancelled".to_string();
            }
            PasswordContext::RemoteList => {
                self.mode = Mode::FileTree;
                self.status_msg = "remote list cancelled".to_string();
            }
        }
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
