use lsp_types::{ClientCapabilities, Uri, WorkspaceFolder};
use miette::{miette, Context, IntoDiagnostic, Result};
use std::collections::HashMap;

use crate::{document::Document, UriExt};

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

    pub fn get_documents(&self) -> impl Iterator<Item = &Document> {
        self.documents.values()
    }
}

#[derive(Default)]
pub struct Server {
    pub documents: DocumentStore,
    root: Option<Uri>,
    client_capabilities: Option<ClientCapabilities>,
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_root(&mut self, uri: Uri) {
        self.root = Some(uri);
    }

    pub fn set_client_capabilities(&mut self, capabilities: ClientCapabilities) {
        self.client_capabilities = Some(capabilities);
    }

    pub fn load_workspaces(
        self: &mut Self,
        workspace_folders: Option<Vec<WorkspaceFolder>>,
    ) -> Result<()> {
        let Some(folders) = workspace_folders else {
            return Err(miette!("Workspaces were not provided."));
        };

        for folder in folders {
            let uri = &folder.uri;
            // TODO: What happens when multiple workspaces are shown and I override the root
            self.set_root(uri.clone());

            let path = uri.path();
            let markdown_files = walkdir::WalkDir::new(path.as_str())
                .into_iter()
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "md"));

            for entry in markdown_files {
                let entry_path = entry.path();
                let contents = std::fs::read_to_string(entry_path)
                    .into_diagnostic()
                    .with_context(|| format!("Failed to read markdown file: {:?}", entry_path))?;

                let uri = Uri::from_file_path(entry_path)
                    .with_context(|| format!("Failed to create URI from path: {:?}", entry_path))?;

                self.documents.open_document(uri, 0, &contents)?;
            }
        }
        Ok(())
    }
}
