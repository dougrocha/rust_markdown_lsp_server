use std::{collections::HashMap, str::FromStr};

use lib_core::{
    document::references::{Reference, ReferenceKind},
    uri::UriExt,
};
use lsp_types::{FileRename, RenameFilesParams, TextEdit, Uri, WorkspaceEdit};
use miette::{IntoDiagnostic, Result};
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
