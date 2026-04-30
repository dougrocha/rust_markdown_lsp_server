pub mod helpers;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use miette::Result;

use crate::document::{Document, references::Reference};

#[derive(Default)]
pub struct Vault {
    documents: HashMap<PathBuf, Document>,
}

impl Vault {
    pub fn create_document(&mut self, path: PathBuf, version: i32, text: &str) -> Result<()> {
        let document = Document::new(path.clone(), text, version)?;
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
            doc.is_open = true;
            doc.update(content, version)?;
        }

        Ok(())
    }

    pub fn close_document(&mut self, path: &Path) {
        if let Some(doc) = self.get_document_mut(path) {
            doc.is_open = false;
        }
    }

    pub fn remove_document(&mut self, path: &Path) {
        self.documents.remove(path);
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

    pub fn get_references(&self) -> impl Iterator<Item = &Reference> {
        self.iter().flat_map(|doc| doc.references.iter())
    }

    pub fn get_references_with_path(&self) -> impl Iterator<Item = (&PathBuf, &Reference)> {
        self.documents.iter().flat_map(|(path, doc)| {
            doc.references
                .iter()
                .map(move |reference| (path, reference))
        })
    }
}
