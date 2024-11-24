use crate::editor::{PersistenceConfig, PersistenceScope};
use helix_core::Selection;
use helix_loader::{
    ensure_parent_dir, find_workspace,
    persistence::{push_history, read_history, trim_history, write_history},
    state_dir,
};
use serde::{Deserialize, Serialize};
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

enum PersistenceFiles {
    Command,
    Search,
    File,
    Clipboard,
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

    fn default_file_path(&self, file: PersistenceFiles) -> PathBuf {
        let filename = match file {
            PersistenceFiles::Command => "command_history",
            PersistenceFiles::Search => "search_history",
            PersistenceFiles::File => "file_history",
            PersistenceFiles::Clipboard => "clipboard",
        };

        let path = self.persistence_dir().join(filename);
        ensure_parent_dir(&path);

        path
    }

    pub fn push_file_history(&self, entry: &FileHistoryEntry) {
        push_history(self.default_file_path(PersistenceFiles::File), entry)
    }

    pub fn read_file_history(&self) -> Vec<FileHistoryEntry> {
        read_history(&self.default_file_path(PersistenceFiles::File))
    }

    pub fn trim_file_history(&self) {
        trim_history::<FileHistoryEntry>(
            self.default_file_path(PersistenceFiles::File),
            self.old_files_trim,
        )
    }
    pub fn push_reg_history(&self, register: char, line: &String) {
        let filepath = match register {
            ':' => self.default_file_path(PersistenceFiles::Command),
            '/' => self.default_file_path(PersistenceFiles::Search),
            _ => return,
        };

        push_history(filepath, line)
    }

    fn read_reg_history(filepath: PathBuf) -> Vec<String> {
        read_history(&filepath)
    }

    pub fn read_command_history(&self) -> Vec<String> {
        let mut hist = Self::read_reg_history(self.default_file_path(PersistenceFiles::Command));
        hist.reverse();
        hist
    }

    pub fn trim_command_history(&self) {
        trim_history::<String>(
            self.default_file_path(PersistenceFiles::Command),
            self.commands_trim,
        )
    }

    pub fn read_search_history(&self) -> Vec<String> {
        let mut hist = Self::read_reg_history(self.default_file_path(PersistenceFiles::Search));
        hist.reverse();
        hist
    }

    pub fn trim_search_history(&self) {
        trim_history::<String>(
            self.default_file_path(PersistenceFiles::Search),
            self.search_trim,
        )
    }

    pub fn write_clipboard_file(&self, values: &Vec<String>) {
        write_history(self.default_file_path(PersistenceFiles::Clipboard), values)
    }

    pub fn read_clipboard_file(&self) -> Vec<String> {
        read_history(&self.default_file_path(PersistenceFiles::Clipboard))
    }
}
