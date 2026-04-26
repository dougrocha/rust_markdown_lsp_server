use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

use lib_core::{
    document::{
        Document,
        references::{Reference, ReferenceKind},
    },
    uri::UriExt,
};
use lsp_types::{RenameFilesParams, TextEdit, Uri, WorkspaceEdit};
use miette::{IntoDiagnostic, Result};

use crate::{
    ServerState,
    helpers::path::{relative_path, resolve_reference_target},
};

pub fn process_will_rename_files(
    lsp: &mut ServerState,
    params: RenameFilesParams,
) -> Result<Option<WorkspaceEdit>> {
    let files = params.files;

    #[allow(clippy::mutable_key_type)]
    let mut changes: HashMap<Uri, Vec<TextEdit>> = HashMap::new();

    for file in &files {
        let old_uri = Uri::from_str(&file.old_uri).into_diagnostic()?;
        let new_uri = Uri::from_str(&file.new_uri).into_diagnostic()?;

        let old_path = Path::new(&file.old_uri);
        let new_path = Path::new(&file.new_uri);

        // update references connected to the changed file
        for (doc, reference) in find_references_to_uri(lsp, &old_uri) {
            let new_rel = relative_path(old_path, new_path)?;
            let new_ref = create_reference_with_new_uri(reference, new_rel);

            changes
                .entry(doc.uri.clone())
                .or_default()
                .push(TextEdit::new(reference.range, new_ref.to_file_text()));
        }

        // update references inside the renamed file itself
        if let Some(doc) = lsp.documents.get_document(&old_uri) {
            for edit in doc
                .references
                .iter()
                .filter(|r| r.kind.is_link())
                .filter_map(|reference| {
                    let resolved = resolve_reference_target(old_path, reference).ok()?;
                    let new_rel = relative_path(new_path, resolved).ok()?;
                    let new_ref = create_reference_with_new_uri(reference, new_rel);
                    Some(TextEdit::new(reference.range, new_ref.to_file_text()))
                })
            {
                changes.entry(new_uri.clone()).or_default().push(edit);
            }
        }
    }

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

// Find all references that match a uri
fn find_references_to_uri<'a>(
    lsp: &'a ServerState,
    match_uri: &Uri,
) -> impl Iterator<Item = (&'a Document, &'a Reference)> {
    let match_path: Option<PathBuf> = match_uri.to_file_path().map(|c| c.into_owned());

    lsp.documents.iter().flat_map(move |doc| {
        doc.references
            .iter()
            .filter(|r| r.kind.is_link())
            .filter_map({
                let match_path = match_path.clone();

                move |reference| {
                    let doc_path = doc.uri.to_file_path()?;
                    let resolved_path = resolve_reference_target(doc_path, reference).ok()?;

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
    use crate::test_utils::TestWorkspace;

    #[test]
    fn rename_updates_link_in_referencing_doc() {
        let mut ws = TestWorkspace::new();

        ws.add_file("/workspace/notes.md", 1, "[link](./target.md)")
            .add_file("/workspace/target.md", 1, "# Target");

        let changes = ws.rename("target.md", "renamed.md");

        let edits = changes.get("/workspace/notes.md").unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "[link](./renamed.md)");
    }

    #[test]
    fn move_to_subfolder_updates_link_in_referencing_doc() {
        let mut ws = TestWorkspace::new();

        ws.add_file("/workspace/notes.md", 1, "[link](./target.md)")
            .add_file("/workspace/target.md", 1, "[notes](./notes.md)");

        let changes = ws.rename("target.md", "docs/target.md");

        let edits = changes.get("/workspace/notes.md").unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "[link](./docs/target.md)");

        let edits = changes.get("/workspace/docs/target.md").unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "[notes](../notes.md)");
    }
}
