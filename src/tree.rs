use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct FileNode {
    pub name: String,
    pub relative_path: PathBuf,
    pub is_dir: bool,
    pub selected: bool,
    pub expanded: bool,
    pub children: Vec<FileNode>,
}

impl FileNode {
    pub fn display_name(&self) -> String {
        if self.is_dir {
            format!("{}/", self.name)
        } else {
            self.name.clone()
        }
    }

    pub fn toggle_selected(&mut self) {
        self.selected = !self.selected;
        if self.is_dir {
            self.set_children_selected(self.selected);
        }
    }

    fn set_children_selected(&mut self, selected: bool) {
        for child in &mut self.children {
            child.selected = selected;
            if child.is_dir {
                child.set_children_selected(selected);
            }
        }
    }

    pub fn toggle_expanded(&mut self) {
        if self.is_dir {
            self.expanded = !self.expanded;
        }
    }

    pub fn select_all(&mut self, selected: bool) {
        self.selected = selected;
        if self.is_dir {
            self.set_children_selected(selected);
        }
    }

    pub fn collect_selected(&self) -> Vec<PathBuf> {
        let mut result = Vec::new();
        self.collect_selected_into(&mut result);
        result
    }

    fn collect_selected_into(&self, result: &mut Vec<PathBuf>) {
        if !self.is_dir && self.selected {
            result.push(self.relative_path.clone());
        }
        for child in &self.children {
            child.collect_selected_into(result);
        }
    }

    #[allow(dead_code)]
    pub fn has_selected_children(&self) -> bool {
        if !self.is_dir && self.selected {
            return true;
        }
        self.children.iter().any(|c| c.has_selected_children())
    }
}

pub fn build_tree(root: &Path) -> Result<FileNode> {
    let root_name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());

    let mut root_node = FileNode {
        name: root_name,
        relative_path: PathBuf::new(),
        is_dir: true,
        selected: false,
        expanded: true,
        children: Vec::new(),
    };

    for entry in WalkDir::new(root).min_depth(1).sort_by(|a, b| {
        let a_dir = a.file_type().is_dir();
        let b_dir = b.file_type().is_dir();
        b_dir.cmp(&a_dir).then(a.file_name().cmp(b.file_name()))
    }) {
        let entry = entry.with_context(|| "failed to read directory entry")?;
        let rel = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .to_path_buf();

        insert_node(&mut root_node, rel, entry.file_type().is_dir());
    }

    Ok(root_node)
}

fn insert_node(root: &mut FileNode, rel_path: PathBuf, is_dir: bool) {
    let parts: Vec<&Path> = rel_path.iter().map(Path::new).collect();
    if parts.is_empty() {
        return;
    }

    let mut current = root;
    for (i, part) in parts.iter().enumerate() {
        let name = part.to_string_lossy().to_string();
        let sub_rel: PathBuf = parts[..=i].iter().map(|p| *p).collect();
        let is_last = i == parts.len() - 1;
        let node_is_dir = if is_last { is_dir } else { true };

        let exists = current
            .children
            .iter()
            .position(|c| c.name == name && c.is_dir == node_is_dir);
        match exists {
            Some(idx) => {
                current = &mut current.children[idx];
            }
            None => {
                current.children.push(FileNode {
                    name,
                    relative_path: sub_rel,
                    is_dir: node_is_dir,
                    selected: false,
                    expanded: false,
                    children: Vec::new(),
                });
                current = current.children.last_mut().unwrap();
            }
        }
    }
}

pub fn flatten_tree_for_display(node: &FileNode, depth: usize, items: &mut Vec<(usize, FileNode)>) {
    if !node.relative_path.as_os_str().is_empty() || depth == 0 {
        items.push((depth, node.clone()));
    }
    if node.is_dir && node.expanded {
        for child in &node.children {
            flatten_tree_for_display(child, depth + 1, items);
        }
    }
}
