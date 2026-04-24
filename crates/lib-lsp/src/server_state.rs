use lsp_types::{ClientCapabilities, Uri, WorkspaceFolder};
use miette::Result;

use lib_core::{uri::UriExt, vault::Vault};

use crate::config::Config;

#[derive(Default)]
pub struct ServerState {
    pub documents: Vault,
    pub config: Config,
    workspace_roots: Vec<Uri>,
    client_capabilities: Option<ClientCapabilities>,
}

impl ServerState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from a file path
    pub fn load_config<P: AsRef<std::path::Path>>(&mut self, config_path: P) {
        tracing::info!("Loading configuration: {:?}", self.config);

        self.config = Config::from_file_or_default(config_path);
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
                    self.documents.create_document(uri, 0, &contents)?;
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
