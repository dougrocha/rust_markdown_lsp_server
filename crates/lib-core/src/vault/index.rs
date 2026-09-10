use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::path::{extract_filename_stem, slug::filename_slug};

/// In-memory index that maps a normalized filename to document paths.
#[derive(Default)]
pub struct VaultIndex {
    by_name: HashMap<String, Vec<PathBuf>>,
}

impl VaultIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a document path to the name index.
    pub(crate) fn insert(&mut self, path: &Path) {
        let Some(key) = name_key(path) else {
            return;
        };
        let bucket = self.by_name.entry(key).or_default();
        if let Err(pos) = bucket.binary_search(&path.to_path_buf()) {
            bucket.insert(pos, path.to_path_buf());
        }
    }

    /// Remove a document path from the name index.
    pub(crate) fn remove(&mut self, path: &Path) {
        let Some(key) = name_key(path) else {
            return;
        };
        let Some(bucket) = self.by_name.get_mut(&key) else {
            return;
        };
        if let Ok(pos) = bucket.binary_search(&path.to_path_buf()) {
            bucket.remove(pos);
        }
        if bucket.is_empty() {
            self.by_name.remove(&key);
        }
    }

    /// Move a document path from `from` to `to` in the name index.
    pub(crate) fn rename(&mut self, from: &Path, to: &Path) {
        self.remove(from);
        self.insert(to);
    }

    /// Return the first path that matches `slug`, sorted for determinism.
    pub fn resolve_name(&self, slug: &str) -> Option<&Path> {
        self.by_name
            .get(slug)
            .and_then(|bucket| bucket.first())
            .map(PathBuf::as_path)
    }

    /// Return every path that matches `slug`.
    pub fn candidates(&self, slug: &str) -> &[PathBuf] {
        self.by_name.get(slug).map_or(&[], Vec::as_slice)
    }
}

/// Compute the index key for a document path.
fn name_key(path: &Path) -> Option<String> {
    extract_filename_stem(path).map(|stem| filename_slug(&stem))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_resolve_by_slug() {
        let mut index = VaultIndex::new();
        index.insert(Path::new("/vault/My Note.md"));

        assert_eq!(
            index.resolve_name("my-note"),
            Some(Path::new("/vault/My Note.md"))
        );
        assert_eq!(index.resolve_name("missing"), None);
    }

    #[test]
    fn collisions_resolve_to_the_sorted_first_path() {
        let mut index = VaultIndex::new();
        index.insert(Path::new("/vault/b/note.md"));
        index.insert(Path::new("/vault/a/note.md"));

        assert_eq!(
            index.resolve_name("note"),
            Some(Path::new("/vault/a/note.md"))
        );
        assert_eq!(index.candidates("note").len(), 2);
    }

    #[test]
    fn remove_drops_the_path_and_empty_bucket() {
        let mut index = VaultIndex::new();
        index.insert(Path::new("/vault/note.md"));
        index.remove(Path::new("/vault/note.md"));

        assert_eq!(index.resolve_name("note"), None);
        assert!(index.candidates("note").is_empty());
    }

    #[test]
    fn rename_moves_the_entry() {
        let mut index = VaultIndex::new();
        index.insert(Path::new("/vault/old.md"));
        index.rename(Path::new("/vault/old.md"), Path::new("/vault/new.md"));

        assert_eq!(index.resolve_name("old"), None);
        assert_eq!(index.resolve_name("new"), Some(Path::new("/vault/new.md")));
    }

    #[test]
    fn insert_is_idempotent() {
        let mut index = VaultIndex::new();
        index.insert(Path::new("/vault/note.md"));
        index.insert(Path::new("/vault/note.md"));

        assert_eq!(index.candidates("note").len(), 1);
    }
}
