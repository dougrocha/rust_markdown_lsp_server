use std::{collections::HashMap, path::PathBuf, str::FromStr};

use lib_core::{
    document::{
        Document,
        references::{Reference, ReferenceKind},
    },
    uri::UriExt,
};
use lsp_types::{FileRename, RenameFilesParams, TextEdit, Uri, WorkspaceEdit};
use miette::{IntoDiagnostic, Result, miette};
use path_clean::PathClean;
use tracing::debug;

use crate::ServerState;

pub fn process_will_rename_files(
    lsp: &mut ServerState,
    params: RenameFilesParams,
) -> Result<Option<WorkspaceEdit>> {
    let files = params.files;

    #[allow(clippy::mutable_key_type)]
    let mut changes: HashMap<Uri, Vec<TextEdit>> = HashMap::new();

    debug!("Starting files");

    for file in &files {
        let FileRename { old_uri, new_uri } = file;
        debug!(?old_uri, ?new_uri);

        let old_uri = Uri::from_str(old_uri).into_diagnostic()?;
        let new_uri = Uri::from_str(new_uri).into_diagnostic()?;

        // update references connected to the changed file
        for (doc, reference) in find_references_to_uri(lsp, &old_uri) {
            let new_path = calculate_inbound_path(&new_uri, &doc.uri);
            let new_ref = create_reference_with_new_uri(reference, new_path);

            changes
                .entry(doc.uri.clone())
                .or_default()
                .push(TextEdit::new(reference.range, new_ref.to_file_text()));
        }

        // update references inside the renamed file itself
        if let Some(doc) = lsp.documents.get_document(&old_uri) {
            let old_dir = old_uri
                .to_file_path()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .ok_or(miette!("Could not find parent"))?;
            let new_dir = new_uri
                .to_file_path()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .ok_or(miette!("Could not find parent"))?;

            let edits: Vec<TextEdit> = doc
                .references
                .iter()
                .filter(|r| r.kind.is_link())
                .filter_map(|reference| {
                    let target_str = reference.kind.get_target()?;
                    let target_abs = old_dir.join(target_str).clean();

                    let mut new_path_str = pathdiff::diff_paths(&target_abs, &new_dir)?
                        .to_string_lossy()
                        .to_string();

                    if !new_path_str.starts_with('.') {
                        new_path_str = format!("./{}", new_path_str);
                    }

                    let new_ref = create_reference_with_new_uri(reference, new_path_str);
                    Some(TextEdit::new(reference.range, new_ref.to_file_text()))
                })
                .collect();

            if !edits.is_empty() {
                changes.entry(new_uri.clone()).or_default().extend(edits);
            }
        }
    }

    debug!(?changes);

    Ok(Some(WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    }))
}

fn calculate_inbound_path(new_uri: &Uri, doc_uri: &Uri) -> String {
    let mut new_path_str = pathdiff::diff_paths(
        new_uri.to_file_path().unwrap(),
        doc_uri.to_file_path().unwrap().parent().unwrap(),
    )
    .unwrap()
    .to_string_lossy()
    .to_string();

    if !new_path_str.starts_with('.') {
        new_path_str = format!("./{}", new_path_str);
    }

    new_path_str
}

fn create_reference_with_new_uri(reference: &Reference, new_target: String) -> Reference {
    let ref_kind = match reference.kind.clone() {
        ReferenceKind::WikiLink { alias, header, .. } => ReferenceKind::WikiLink {
            target: new_target,
            alias,
            header,
        },
        ReferenceKind::Link {
            alt_text, header, ..
        } => ReferenceKind::Link {
            target: new_target,
            header,
            alt_text,
            title: None,
        },
        other => other,
    };

    Reference {
        kind: ref_kind,
        range: reference.range,
    }
}

fn find_references_to_uri<'a>(
    lsp: &'a ServerState,
    match_uri: &Uri,
) -> impl Iterator<Item = (&'a Document, &'a Reference)> {
    let match_path: Option<PathBuf> = match_uri.to_file_path().map(|c| c.into_owned());

    lsp.documents.iter().flat_map(move |doc| {
        doc.references.iter().flat_map({
            let match_path = match_path.clone();
            move |reference| {
                let target_str = reference.kind.get_target()?;

                let doc_path = doc.uri.to_file_path()?;
                let base_path = doc_path.parent()?;

                let resolved_path = base_path.join(target_str).clean();

                if let Some(ref m_path) = match_path
                    && m_path == &resolved_path
                {
                    return Some((doc, reference));
                }

                None
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server_state::ServerState;
    use lsp_types::{FileRename, RenameFilesParams};
    use std::str::FromStr;

    fn uri(path: &str) -> Uri {
        Uri::from_str(&format!("file://{}", path)).unwrap()
    }

    fn init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_test_writer()
            .try_init();
    }

    fn open(server: &mut ServerState, path: &str, content: &str) {
        server
            .documents
            .create_document(uri(path), 1, content)
            .unwrap();
    }

    #[test]
    fn rename_updates_link_in_referencing_doc() {
        init_tracing();
        let mut server = ServerState::new();
        open(&mut server, "/workspace/notes.md", "[link](./target.md)");
        open(&mut server, "/workspace/target.md", "# Target");

        let params = RenameFilesParams {
            files: vec![FileRename {
                old_uri: uri("/workspace/target.md").to_string(),
                new_uri: uri("/workspace/renamed.md").to_string(),
            }],
        };

        let edit = process_will_rename_files(&mut server, params)
            .unwrap()
            .unwrap();

        #[allow(clippy::mutable_key_type)]
        let changes = edit.changes.unwrap();
        let edits = changes.get(&uri("/workspace/notes.md")).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "[link](./renamed.md)");
    }

    #[test]
    fn move_to_subfolder_updates_link_in_referencing_doc() {
        init_tracing();
        let mut server = ServerState::new();
        open(&mut server, "/workspace/notes.md", "[link](./target.md)");
        open(&mut server, "/workspace/target.md", "[notes](./notes.md)");

        let params = RenameFilesParams {
            files: vec![FileRename {
                old_uri: uri("/workspace/target.md").to_string(),
                new_uri: uri("/workspace/docs/target.md").to_string(),
            }],
        };

        let edit = process_will_rename_files(&mut server, params)
            .unwrap()
            .unwrap();

        #[allow(clippy::mutable_key_type)]
        let changes = edit.changes.unwrap();
        let edits = changes.get(&uri("/workspace/notes.md")).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "[link](./docs/target.md)");

        let edits = changes.get(&uri("/workspace/docs/target.md")).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "[notes](../notes.md)");
    }
}
