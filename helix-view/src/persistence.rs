use crate::editor::{OptionToml, PersistenceConfig, PersistenceConfigOption, PersistenceScope};
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

pub enum PersistenceType {
    Command,
    Search,
    File,
    Clipboard,
    Splits,
}

impl PersistenceConfig {
    fn get_option(&self, file: PersistenceType) -> &OptionToml<PersistenceConfigOption> {
        match file {
            PersistenceType::Command => &self.commands,
            PersistenceType::Search => &self.search,
            PersistenceType::Clipboard => &self.clipboard,
            PersistenceType::File => &self.old_files,
            PersistenceType::Splits => &self.splits,
        }
    }

    pub fn enabled(&self, file: PersistenceType) -> bool {
        match self.get_option(file) {
            OptionToml::None => self.all.enabled,
            OptionToml::Some(opt) => opt.enabled,
        }
    }

    pub fn max_entries(&self, file: PersistenceType) -> usize {
        match self.get_option(file) {
            OptionToml::None => self.all.max_entries,
            OptionToml::Some(opt) => opt.max_entries,
        }
    }

    pub fn scope(&self, file: PersistenceType) -> &PersistenceScope {
        match self.get_option(file) {
            OptionToml::None => &self.all.scope,
            OptionToml::Some(opt) => &opt.scope,
        }
    }

    fn persistence_dir(scope: &PersistenceScope) -> PathBuf {
        match scope {
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

        let path = PersistenceConfig::persistence_dir(self.scope(file)).join(filename);
        ensure_parent_dir(&path);

        path
    }

    pub fn exclude(&self, path: &str) -> bool {
        self.files_exclusions.iter().any(|r| r.is_match(path))
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
            self.max_entries(PersistenceType::File),
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
            self.max_entries(PersistenceType::Command),
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
            self.max_entries(PersistenceType::Search),
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
        let max_entries = self.max_entries(PersistenceType::Splits);

        if splits_in_file.len() < max_entries {
            return;
        }

        let mut splits = HashMap::with_capacity(max_entries);
        for entry in splits_in_file {
            splits.insert(entry.name.clone(), entry);
            if splits.len() == max_entries {
                break;
            }
        }

        write_history(
            self.default_file_path(PersistenceType::Splits),
            &splits.values().collect(),
        )
    }
}
