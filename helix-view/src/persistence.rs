use crate::editor::{PersistenceConfig, PersistenceScope};
use helix_core::Selection;
use helix_loader::{
    ensure_parent_dir, find_workspace,
    persistence::{push_history, read_history, trim_history, write_history},
    state_dir,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::view::ViewPosition;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileHistoryEntry {
    pub path: PathBuf,
    pub view_position: ViewPosition,
    pub selection: Selection,
}

impl FileHistoryEntry {
    pub fn new(path: PathBuf, view_position: ViewPosition, selection: Selection) -> Self {
        Self {
            path,
            view_position,
            selection,
        }
    }
}

// Data structures to represent a split view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layout {
    Horizontal,
    Vertical,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitEntryNode {
    pub layout: Layout,
    pub children: Vec<SplitEntryTree>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitEntryLeaf {
    // Path to the document.
    pub path: PathBuf,
    // Where was the position of the view.
    pub view_position: ViewPosition,
    pub selection: Selection,
    // Whether this was the focused split or not.
    pub focus: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitEntryTree {
    Leaf(Option<SplitEntryLeaf>),
    Node(SplitEntryNode),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitEntry {
    pub name: String,
    pub tree: SplitEntryTree,
}

enum PersistenceType {
    Command,
    Search,
    File,
    Clipboard,
    Splits,
}

impl PersistenceConfig {
    fn persistence_dir(&self) -> PathBuf {
        match &self.scope {
            PersistenceScope::AllInOne => state_dir(),
            PersistenceScope::PerWorkspace => state_dir().join(
                find_workspace()
                    .0
                    .strip_prefix("/")
                    .unwrap()
                    .to_string_lossy()
                    .replace('/', "%"),
            ),
            PersistenceScope::Dir(dir) => PathBuf::from(dir),
        }
    }

    fn default_file_path(&self, file: PersistenceType) -> PathBuf {
        let filename = match file {
            PersistenceType::Command => "command_history",
            PersistenceType::Search => "search_history",
            PersistenceType::File => "file_history",
            PersistenceType::Clipboard => "clipboard",
            PersistenceType::Splits => "splits",
        };

        let path = self.persistence_dir().join(filename);
        ensure_parent_dir(&path);

        path
    }

    pub fn push_file_history(&self, entry: &FileHistoryEntry) {
        push_history(self.default_file_path(PersistenceType::File), entry)
    }

    pub fn read_file_history(&self) -> Vec<FileHistoryEntry> {
        read_history(&self.default_file_path(PersistenceType::File))
    }

    pub fn trim_file_history(&self) {
        trim_history::<FileHistoryEntry>(
            self.default_file_path(PersistenceType::File),
            self.old_files_trim,
        )
    }
    pub fn push_reg_history(&self, register: char, line: &String) {
        let filepath = match register {
            ':' => self.default_file_path(PersistenceType::Command),
            '/' => self.default_file_path(PersistenceType::Search),
            _ => return,
        };

        push_history(filepath, line)
    }

    fn read_reg_history(filepath: PathBuf) -> Vec<String> {
        read_history(&filepath)
    }

    pub fn read_command_history(&self) -> Vec<String> {
        let mut hist = Self::read_reg_history(self.default_file_path(PersistenceType::Command));
        hist.reverse();
        hist
    }

    pub fn trim_command_history(&self) {
        trim_history::<String>(
            self.default_file_path(PersistenceType::Command),
            self.commands_trim,
        )
    }

    pub fn read_search_history(&self) -> Vec<String> {
        let mut hist = Self::read_reg_history(self.default_file_path(PersistenceType::Search));
        hist.reverse();
        hist
    }

    pub fn trim_search_history(&self) {
        trim_history::<String>(
            self.default_file_path(PersistenceType::Search),
            self.search_trim,
        )
    }

    pub fn write_clipboard_file(&self, values: &Vec<String>) {
        write_history(self.default_file_path(PersistenceType::Clipboard), values)
    }

    pub fn read_clipboard_file(&self) -> Vec<String> {
        read_history(&self.default_file_path(PersistenceType::Clipboard))
    }

    pub fn push_split_entry(&self, entry: &SplitEntry) {
        push_history(self.default_file_path(PersistenceType::Splits), &entry);
    }

    pub fn read_split_file(&self) -> Vec<SplitEntry> {
        read_history(&self.default_file_path(PersistenceType::Splits))
    }

    pub fn trim_split_file(&self) {
        let splits_in_file = self.read_split_file();

        if splits_in_file.len() < self.splits_trim {
            return;
        }

        let mut splits = HashMap::with_capacity(self.splits_trim);
        for entry in splits_in_file {
            splits.insert(entry.name.clone(), entry);
            if splits.len() == self.splits_trim {
                break;
            }
        }

        write_history(
            self.default_file_path(PersistenceType::Splits),
            &splits.values().collect(),
        )
    }
}
