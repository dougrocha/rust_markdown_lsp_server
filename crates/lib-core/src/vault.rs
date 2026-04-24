use miette::Result;
use std::collections::HashMap;

use lsp_types::Uri;

use crate::document::{Document, references::Reference};

#[derive(Default)]
pub struct Vault {
    documents: HashMap<Uri, Document>,
}

impl Vault {
    pub fn create_document(&mut self, uri: Uri, version: i32, text: &str) -> Result<()> {
        let document = Document::new(uri.clone(), text, version)?;
        self.documents.insert(uri.clone(), document);

        Ok(())
    }

    pub fn update_document(&mut self, uri: &Uri, version: i32, text: &str) -> Result<()> {
        if let Some(document) = self.get_document_mut(uri) {
            document.update(text, version)?;
        }

        Ok(())
    }

    pub fn open_document(&mut self, uri: &Uri, version: i32, content: &str) -> Result<()> {
        if let Some(doc) = self.get_document_mut(uri) {
            doc.is_open = true;
            doc.update(content, version)?;
        }

        Ok(())
    }

    pub fn close_document(&mut self, uri: &Uri) {
        if let Some(doc) = self.get_document_mut(uri) {
            doc.is_open = false;
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

    pub fn iter(&self) -> impl Iterator<Item = &Document> {
        self.documents.values()
    }

    pub fn get_references(&self) -> impl Iterator<Item = &Reference> {
        self.iter().flat_map(|doc| doc.references.iter())
    }

    pub fn get_references_with_uri(&self) -> impl Iterator<Item = (&Uri, &Reference)> {
        self.documents
            .iter()
            .flat_map(|(uri, doc)| doc.references.iter().map(move |reference| (uri, reference)))
    }
}
