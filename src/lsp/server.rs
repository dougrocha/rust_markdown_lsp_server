use std::collections::HashMap;

use lsp_types::Uri;

use crate::document::Document;

#[derive(Default)]
pub struct LspServer {
    documents: HashMap<Uri, Document>,
    root: Option<Uri>,
}

impl LspServer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_root(&mut self, uri: Uri) {
        self.root = Some(uri);
    }

    pub fn open_document(&mut self, uri: Uri, text: &str) {
        let document = Document::new(uri.clone(), text);
        self.documents.insert(uri, document);
    }

    pub fn update_document(&mut self, uri: &Uri, text: &str) {
        if let Some(document) = self.get_document_mut(&uri) {
            document.update(text);
        }
    }

    pub fn remove_document(&mut self, uri: &Uri) {
        self.documents.remove(uri);
    }

    pub fn get_document(&self, uri: &Uri) -> Option<&Document> {
        self.documents.get(uri)
    }

    pub fn get_document_mut(&mut self, uri: &Uri) -> Option<&mut Document> {
        self.documents.get_mut(uri)
    }

    pub fn get_root(&self) -> Option<&Uri> {
        self.root.as_ref()
    }
}
