use lsp_types::{ClientCapabilities, Uri, WorkspaceFolder};
use miette::Result;
use std::collections::{HashMap, HashSet};

use lib_core::{
    document::{Document, references::Reference},
    uri::UriExt,
};

use crate::config::Config;

#[derive(Default)]
pub struct DocumentStore {
    documents: HashMap<Uri, Document>,
    open_documents: HashSet<Uri>,
}

impl DocumentStore {
    pub fn index_document(&mut self, uri: Uri, text: &str) -> Result<()> {
        let document = Document::new(uri.clone(), text, 0)?;
        self.documents.insert(uri, document);

        Ok(())
    }

    pub fn open_document(&mut self, uri: Uri, version: i32, text: &str) -> Result<()> {
        let document = Document::new(uri.clone(), text, version)?;
        self.documents.insert(uri.clone(), document);
        self.open_documents.insert(uri);

        Ok(())
    }

    pub fn close_document(&mut self, uri: &Uri) {
        self.open_documents.remove(uri);
    }

    pub fn is_open(&self, uri: &Uri) -> bool {
        self.open_documents.contains(uri)
    }

    pub fn update_document(&mut self, uri: &Uri, text: &str) -> Result<()> {
        if let Some(document) = self.get_document_mut(uri) {
            document.update(text, 0)?;
        }

        Ok(())
    }

    pub fn remove_document(&mut self, uri: &Uri) {
        self.open_documents.remove(uri);
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

    pub fn get_references(&self) -> impl Iterator<Item = &Reference> {
        self.get_documents().flat_map(|doc| doc.references.iter())
    }

    pub fn get_references_with_uri(&self) -> impl Iterator<Item = (&Uri, &Reference)> {
        self.documents
            .iter()
            .flat_map(|(uri, doc)| doc.references.iter().map(move |reference| (uri, reference)))
    }
}

#[derive(Default)]
pub struct Server {
    pub documents: DocumentStore,
    pub config: Config,
    workspace_roots: Vec<Uri>,
    client_capabilities: Option<ClientCapabilities>,
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from a file path
    pub fn load_config<P: AsRef<std::path::Path>>(&mut self, config_path: P) {
        self.config = Config::from_file_or_default(config_path);
        tracing::info!("Loaded configuration: {:?}", self.config);
    }

    pub fn insert_root(&mut self, uri: Uri) {
        self.workspace_roots.push(uri);
    }

    pub fn set_client_capabilities(&mut self, capabilities: ClientCapabilities) {
        self.client_capabilities = Some(capabilities);
    }

    /// Returns the "primary" root (the first one opened), if any.
    /// Useful for fallback scenarios, but prefer `get_workspace_root_for_uri`.
    pub fn primary_root(&self) -> Option<&Uri> {
        self.workspace_roots.first()
    }

    pub fn load_workspaces(
        &mut self,
        workspace_folders: Option<Vec<WorkspaceFolder>>,
    ) -> Result<()> {
        let Some(folders) = workspace_folders else {
            tracing::info!("No workspace folders provided - running in single-file mode.");
            return Ok(());
        };

        for folder in folders {
            let root_uri = folder.uri;

            tracing::info!("Adding workspace root: {:?}", root_uri);
            self.workspace_roots.push(root_uri.clone());

            // 2. Scan the files in this specific root
            let Some(root_path) = root_uri.to_file_path() else {
                tracing::warn!("Skipping invalid workspace path: {:?}", root_uri);
                continue;
            };

            let markdown_files = walkdir::WalkDir::new(&root_path)
                .into_iter()
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    let path = entry.path();
                    path.is_file() && path.extension().is_some_and(|ext| ext == "md")
                });

            for entry in markdown_files {
                let entry_path = entry.path();
                let contents = match std::fs::read_to_string(entry_path) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::debug!("Could not read file {:?}: {}", entry_path, e);
                        continue;
                    }
                };

                if let Some(uri) = Uri::from_file_path(entry_path) {
                    self.documents.index_document(uri, &contents)?;
                }
            }
        }

        Ok(())
    }

    /// Look at all workspaces and take the shortest root
    ///
    /// Sort by length ensures we get the most specific (deepest) folder if they are nested
    pub fn get_workspace_root_for_uri(&self, document_uri: &Uri) -> Option<&Uri> {
        if self.workspace_roots.len() == 1 {
            return self.workspace_roots.first();
        }

        let doc_path = document_uri.to_file_path()?;

        self.workspace_roots
            .iter()
            .filter(|root_uri| {
                if let Some(root_path) = root_uri.to_file_path() {
                    doc_path.starts_with(root_path)
                } else {
                    false
                }
            })
            .max_by_key(|uri| uri.path().as_estr().as_str().len())
    }
}
