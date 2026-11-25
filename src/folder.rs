use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::markdown::is_supported_file;

const MAX_DEPTH: usize = 8;
const MAX_FILES: usize = 2000;

const IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".venv",
    "venv",
    ".idea",
    ".vscode",
    "__pycache__",
];

#[derive(Debug, Clone, Serialize)]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Vec<FileTreeNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderTree {
    pub root_name: String,
    pub root_path: String,
    pub file_count: usize,
    pub truncated: bool,
    pub nodes: Vec<FileTreeNode>,
}

/// Strip Windows verbatim path prefix `\\?\` and normalize path string for comparison.
pub fn normalize_path_str(path: &Path) -> String {
    let raw = path.to_string_lossy();
    #[cfg(windows)]
    {
        if let Some(stripped) = raw.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{stripped}");
        }
        if let Some(stripped) = raw.strip_prefix(r"\\?\") {
            return stripped.to_string();
        }
    }
    raw.into_owned()
}

pub fn scan_folder(root: &Path) -> Option<FolderTree> {
    if !root.is_dir() {
        return None;
    }

    let canonical = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let root_path = normalize_path_str(&canonical);
    let root_name = canonical
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| root_path.clone());

    let mut file_count = 0usize;
    let mut truncated = false;
    let nodes = scan_dir_recursive(&canonical, 0, &mut file_count, &mut truncated);

    Some(FolderTree {
        root_name,
        root_path,
        file_count,
        truncated,
        nodes,
    })
}

fn is_ignored_dir(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    IGNORED_DIRS.iter().any(|&d| d == lower)
}

fn sort_nodes(nodes: &mut [FileTreeNode]) {
    nodes.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => a
            .name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
            .then_with(|| a.name.cmp(&b.name)),
    });
}

fn scan_dir_recursive(
    dir: &Path,
    depth: usize,
    file_count: &mut usize,
    truncated: &mut bool,
) -> Vec<FileTreeNode> {
    if depth >= MAX_DEPTH || *file_count >= MAX_FILES {
        if *file_count >= MAX_FILES {
            *truncated = true;
        }
        return Vec::new();
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut result = Vec::new();
    let mut subdirs: Vec<(String, PathBuf)> = Vec::new();
    let mut files: Vec<FileTreeNode> = Vec::new();

    for entry in entries.flatten() {
        if *file_count >= MAX_FILES {
            *truncated = true;
            break;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();

        if file_type.is_dir() {
            if !is_ignored_dir(&name) {
                subdirs.push((name, path));
            }
        } else if file_type.is_file() && is_supported_file(&path) {
            *file_count += 1;
            files.push(FileTreeNode {
                name,
                path: normalize_path_str(&path),
                is_dir: false,
                children: Vec::new(),
            });
        }
    }

    for (name, path) in subdirs {
        if *file_count >= MAX_FILES {
            *truncated = true;
            break;
        }
        let children = scan_dir_recursive(&path, depth + 1, file_count, truncated);
        if !children.is_empty() {
            result.push(FileTreeNode {
                name,
                path: normalize_path_str(&path),
                is_dir: true,
                children,
            });
        }
    }

    result.extend(files);
    sort_nodes(&mut result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("mdv_folder_{label}_{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn scans_supported_files_and_skips_ignored_and_empty_dirs() {
        let root = make_temp_dir("tree");
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::create_dir_all(root.join("empty_only_png")).unwrap();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();

        fs::write(root.join("README.md"), "# Hi").unwrap();
        fs::write(root.join("notes.txt"), "note").unwrap();
        fs::write(root.join("docs/guide.markdown"), "# Guide").unwrap();
        fs::write(root.join("docs/diagram.mmd"), "graph TD; A-->B;").unwrap();
        fs::write(root.join("empty_only_png/pic.png"), "png").unwrap();
        fs::write(root.join("node_modules/pkg/README.md"), "skip").unwrap();
        fs::write(root.join(".git/info.md"), "skip").unwrap();

        let tree = scan_folder(&root).expect("should scan folder");
        assert_eq!(tree.file_count, 4);
        assert!(!tree.truncated);

        // Directories sorted before files
        assert_eq!(tree.nodes.len(), 3);
        assert!(tree.nodes[0].is_dir);
        assert_eq!(tree.nodes[0].name, "docs");
        assert_eq!(tree.nodes[0].children.len(), 2);
        assert_eq!(tree.nodes[1].name, "notes.txt");
        assert_eq!(tree.nodes[2].name, "README.md");

        let _ = fs::remove_dir_all(&root);
    }
}
