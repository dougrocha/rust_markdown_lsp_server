use std::{collections::HashMap, path::PathBuf};

use gen_lsp_types::{FileRename, RenameFilesParams, TextEdit};

use crate::{ServerState, handlers::rename::will_rename::process_will_rename_files, uri::UriExt};

pub struct TestWorkspace {
    pub state: ServerState,
    pub root: PathBuf,
}

impl TestWorkspace {
    pub fn new() -> Self {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_test_writer()
            .try_init();

        Self {
            state: ServerState::new(),
            root: PathBuf::from("/workspace"),
        }
    }

    pub fn add_file(&mut self, path: &str, version: i32, content: &str) -> &mut Self {
        self.state
            .documents
            .create_document(PathBuf::from(path), version, content)
            .unwrap();

        self
    }

    pub fn rename(&mut self, old: &str, new: &str) -> HashMap<String, Vec<TextEdit>> {
        let params = RenameFilesParams {
            files: vec![FileRename {
                old_uri: format!("file://{}", self.root.join(old).display()),
                new_uri: format!("file://{}", self.root.join(new).display()),
            }],
        };

        let edits = process_will_rename_files(&mut self.state, params)
            .unwrap()
            .unwrap();

        edits
            .changes
            .unwrap_or_default()
            .into_iter()
            .map(|(uri, text_edits)| {
                let path = uri
                    .to_file_path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| uri.as_ref().to_string());
                (path, text_edits)
            })
            .collect()
    }
}
