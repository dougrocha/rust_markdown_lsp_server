pub mod index;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use miette::Result;

use crate::document::Document;
use crate::vault::index::VaultIndex;

#[derive(Default)]
pub struct Vault {
    documents: HashMap<PathBuf, Document>,
    index: VaultIndex,
}

impl Vault {
    pub fn create_document(&mut self, path: PathBuf, version: i32, text: &str) -> Result<()> {
        let document = Document::new(path.clone(), text, version)?;
        self.index.insert(&path);
        self.documents.insert(path, document);

        Ok(())
    }

    pub fn update_document(&mut self, path: &Path, version: i32, text: &str) -> Result<()> {
        if let Some(document) = self.get_document_mut(path) {
            document.update(text, version)?;
        }

        Ok(())
    }

    pub fn open_document(&mut self, path: &Path, version: i32, content: &str) -> Result<()> {
        if let Some(doc) = self.get_document_mut(path) {
            doc.update(content, version)?;
        }

        Ok(())
    }

    /// Move a document to a new path and reparse its content.
    pub fn rename_document(
        &mut self,
        from: &Path,
        to: &Path,
        version: i32,
        text: &str,
    ) -> Result<()> {
        self.documents.remove(from);
        self.index.rename(from, to);
        let document = Document::new(to.to_path_buf(), text, version)?;
        self.documents.insert(to.to_path_buf(), document);

        Ok(())
    }

    pub fn remove_document(&mut self, path: &Path) {
        self.documents.remove(path);
        self.index.remove(path);
    }

    /// Return the name index for link resolution.
    pub fn index(&self) -> &VaultIndex {
        &self.index
    }

    pub fn get_document(&self, path: &Path) -> Option<&Document> {
        self.documents.get(path)
    }

    pub fn get_document_mut(&mut self, path: &Path) -> Option<&mut Document> {
        self.documents.get_mut(path)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Document> {
        self.documents.values()
    }
}
