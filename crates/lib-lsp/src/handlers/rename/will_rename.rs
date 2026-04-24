use std::{collections::HashMap, str::FromStr};

use lib_core::{
    document::references::{Reference, ReferenceKind},
    get_document,
    uri::UriExt,
};
use lsp_types::{FileRename, RenameFilesParams, TextEdit, Uri, WorkspaceEdit};
use miette::{Context, IntoDiagnostic, Result};
use path_clean::PathClean;
use tracing::debug;

use crate::Server;

pub fn process_will_rename_files(
    lsp: &mut Server,
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

        // let references = ReferenceCollector::collect_file_references(lsp, &old_uri);
        let references = find_references_to_uri(lsp, &old_uri);
        for (doc_uri, reference) in references {
            debug!(?doc_uri, ?reference);

            let mut new_path_str = pathdiff::diff_paths(
                new_uri.to_file_path().unwrap(),
                doc_uri.to_file_path().unwrap().parent().unwrap(),
            )
            .unwrap()
            .to_string_lossy()
            .to_string();

            debug!(?new_path_str);

            if !new_path_str.starts_with('.') {
                new_path_str = format!("./{}", new_path_str);
            }

            let new_ref = create_reference_with_new_uri(reference, new_path_str);
            let text_edit = TextEdit::new(reference.range, new_ref.to_file_text());

            changes.entry(doc_uri.clone()).or_default().push(text_edit);
        }

        // renames the references inside the file that was moved,
        // not working right now
        debug!("Start Phase of renaming");

        let doc = get_document!(lsp, &new_uri);
        let references = &doc.references;

        for reference in references
            .iter()
            .filter(|reference| reference.kind.is_link())
        {
            debug!(?reference);

            let Some(target_file_path) = old_uri.to_file_path().and_then(|path| {
                path.parent()
                    .map(|p| p.to_path_buf().join(reference.kind.get_target().unwrap()))
            }) else {
                continue;
            };

            let mut new_path_str = pathdiff::diff_paths(
                target_file_path,
                new_uri.to_file_path().unwrap().parent().unwrap(),
            )
            .unwrap()
            .to_string_lossy()
            .to_string();

            debug!(?new_path_str);

            if !new_path_str.starts_with('.') {
                new_path_str = format!("./{}", new_path_str);
            }

            let new_ref = create_reference_with_new_uri(reference, new_path_str);
            let text_edit = TextEdit::new(reference.range, new_ref.to_file_text());

            changes.entry(doc.uri.clone()).or_default().push(text_edit);
        }
    }

    debug!(?changes);

    Ok(Some(WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    }))
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
    lsp: &'a Server,
    match_uri: &Uri,
) -> impl Iterator<Item = (&'a Uri, &'a Reference)> {
    lsp.documents
        .get_references_with_uri()
        .filter(move |(base_uri, reference)| {
            let Some(reference_target) = reference.kind.get_target() else {
                return false;
            };

            let Some(base_file_path) = base_uri
                .to_file_path()
                .and_then(|path| path.parent().map(|p| p.to_path_buf()))
            else {
                return false;
            };

            let resolved_path = base_file_path.join(reference_target).clean();

            match_uri
                .to_file_path()
                .is_some_and(|uri_path| resolved_path == uri_path)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::Server;
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

    fn open(server: &mut Server, path: &str, content: &str) {
        server
            .documents
            .open_document(uri(path), 1, content)
            .unwrap();
    }

    #[test]
    fn rename_updates_link_in_referencing_doc() {
        init_tracing();
        let mut server = Server::new();
        open(&mut server, "/workspace/notes.md", "[link](./target.md)");
        open(&mut server, "/workspace/renamed.md", "# Target");

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
        let mut server = Server::new();
        open(&mut server, "/workspace/notes.md", "[link](./target.md)");
        open(
            &mut server,
            "/workspace/docs/target.md",
            "[notes](./notes.md)",
        );

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
