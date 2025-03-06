use std::{collections::HashMap, path::PathBuf};

use document::{
    references::{LinkData, LinkHeader, Reference},
    Document,
};
use lsp::URI;

pub mod document;
pub mod lsp;
pub mod message;
pub mod rpc;

#[derive(Default)]
pub struct LspServer {
    documents: HashMap<String, Document>,
    root: Option<PathBuf>,
}

impl LspServer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_root(&mut self, uri: URI) {
        self.root = Some(uri.to_path_buf());
    }

    pub fn open_document(&mut self, uri: &str, text: &str) {
        let document = Document::new(uri, &text);
        self.documents.insert(uri.to_string(), document);
    }

    pub fn update_document(&mut self, uri: &str, text: &str) {
        if let Some(document) = self.get_document_mut(uri) {
            document.update(text);
        }
    }

    pub fn remove_document(&mut self, uri: &str) {
        self.documents.remove(uri);
    }

    pub fn get_document(&self, uri: &str) -> Option<&Document> {
        self.documents.get(uri)
    }

    fn get_document_mut(&mut self, uri: &str) -> Option<&mut Document> {
        self.documents.get_mut(uri)
    }
}
