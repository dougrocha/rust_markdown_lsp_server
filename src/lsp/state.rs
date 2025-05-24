use lsp_types::Uri;
use miette::Result;
use std::collections::HashMap;

use crate::document::Document;

#[derive(Default)]
pub struct DocumentStore {
    documents: HashMap<Uri, Document>,
}

impl DocumentStore {
    pub fn open_document(&mut self, uri: Uri, version: i32, text: &str) -> Result<()> {
        let document = Document::new(uri.clone(), text, version)?;
        self.documents.insert(uri, document);

        Ok(())
    }

    pub fn update_document(&mut self, uri: &Uri, text: &str) -> Result<()> {
        if let Some(document) = self.get_document_mut(uri) {
            document.update(text, 0)?;
        }

        Ok(())
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
}

#[derive(Default)]
pub struct LspState {
    pub documents: DocumentStore,
    root: Option<Uri>,
}

impl LspState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_root(&mut self, uri: Uri) {
        self.root = Some(uri);
    }
}
