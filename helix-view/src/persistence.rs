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

enum PersistenceFiles {
    Command,
    Search,
    File,
    Clipboard,
    Splits,
}

fn persistence_dir(config: &PersistenceConfig) -> PathBuf {
    match &config.scope {
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

fn default_file_path(file: PersistenceFiles, config: &PersistenceConfig) -> PathBuf {
    let filename = match file {
        PersistenceFiles::Command => "command_history",
        PersistenceFiles::Search => "search_history",
        PersistenceFiles::File => "file_history",
        PersistenceFiles::Clipboard => "clipboard",
        PersistenceFiles::Splits => "splits",
    };

    let path = persistence_dir(config).join(filename);
    ensure_parent_dir(&path);

    path
}

#[derive(Debug, Clone)]
pub struct Persistence {
    config: PersistenceConfig,
}

impl Persistence {
    pub fn new(config: PersistenceConfig) -> Self {
        Self { config }
    }

    pub fn refresh_config(&mut self, config: PersistenceConfig) {
        self.config = config;
    }

    pub fn push_file_history(&self, entry: &FileHistoryEntry) {
        push_history(
            default_file_path(PersistenceFiles::File, &self.config),
            entry,
        )
    }

    pub fn read_file_history(&self) -> Vec<FileHistoryEntry> {
        read_history(&default_file_path(PersistenceFiles::File, &self.config))
    }

    pub fn trim_file_history(&self) {
        trim_history::<FileHistoryEntry>(
            default_file_path(PersistenceFiles::File, &self.config),
            self.config.old_files_trim,
        )
    }
    pub fn push_reg_history(&self, register: char, line: &String) {
        let filepath = match register {
            ':' => default_file_path(PersistenceFiles::Command, &self.config),
            '/' => default_file_path(PersistenceFiles::Search, &self.config),
            _ => return,
        };

        push_history(filepath, line)
    }

    fn read_reg_history(filepath: PathBuf) -> Vec<String> {
        read_history(&filepath)
    }

    pub fn read_command_history(&self) -> Vec<String> {
        let mut hist =
            Self::read_reg_history(default_file_path(PersistenceFiles::Command, &self.config));
        hist.reverse();
        hist
    }

    pub fn trim_command_history(&self) {
        trim_history::<String>(
            default_file_path(PersistenceFiles::Command, &self.config),
            self.config.commands_trim,
        )
    }

    pub fn read_search_history(&self) -> Vec<String> {
        let mut hist =
            Self::read_reg_history(default_file_path(PersistenceFiles::Search, &self.config));
        hist.reverse();
        hist
    }

    pub fn trim_search_history(&self) {
        trim_history::<String>(
            default_file_path(PersistenceFiles::Search, &self.config),
            self.config.search_trim,
        )
    }

    pub fn write_clipboard_file(&self, values: &Vec<String>) {
        write_history(
            default_file_path(PersistenceFiles::Clipboard, &self.config),
            values,
        )
    }

    pub fn read_clipboard_file(&self) -> Vec<String> {
        read_history(&default_file_path(
            PersistenceFiles::Clipboard,
            &self.config,
        ))
    }

    pub fn push_split_entry(&self, entry: &SplitEntry) {
        push_history(
            default_file_path(PersistenceFiles::Splits, &self.config),
            &entry,
        );
    }

    pub fn read_split_file(&self) -> Vec<SplitEntry> {
        read_history(&default_file_path(PersistenceFiles::Splits, &self.config))
    }

    pub fn trim_split_file(&self) {
        let splits_in_file = self.read_split_file();

        if splits_in_file.len() < self.config.splits_trim {
            return;
        }

        let mut splits = HashMap::with_capacity(self.config.splits_trim);
        for entry in splits_in_file {
            splits.insert(entry.name.clone(), entry);
            if splits.len() == self.config.splits_trim {
                break;
            }
        }

        write_history(
            default_file_path(PersistenceFiles::Splits, &self.config),
            &splits.values().collect(),
        )
    }
}
