use std::{collections::HashMap, path::PathBuf};

use lsp_types::uri::URI;

use crate::document::Document;

#[derive(Default)]
pub struct LspServer {
    documents: HashMap<String, Document>,
    root: Option<PathBuf>,
}

impl LspServer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_root(&mut self, uri: &URI) {
        self.root = Some(uri.to_path_buf());
    }

    pub fn open_document(&mut self, uri: URI, text: &str) {
        let document = Document::new(uri.clone(), text);
        self.documents.insert(uri.to_string(), document);
    }

    pub fn update_document(&mut self, uri: impl AsRef<str>, text: &str) {
        if let Some(document) = self.get_document_mut(uri) {
            document.update(text);
        }
    }

    pub fn remove_document(&mut self, uri: impl AsRef<str>) {
        self.documents.remove(uri.as_ref());
    }

    pub fn get_document(&self, uri: impl AsRef<str>) -> Option<&Document> {
        self.documents.get(uri.as_ref())
    }

    fn get_document_mut(&mut self, uri: impl AsRef<str>) -> Option<&mut Document> {
        self.documents.get_mut(uri.as_ref())
    }
}
