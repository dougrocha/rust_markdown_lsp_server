use std::{collections::HashMap, path::PathBuf, str::FromStr};

use gen_lsp_types::{RenameFilesParams, TextEdit, Uri, WorkspaceEdit};
use lib_core::{
    document::{Document, index::Link},
    path::{combine_and_normalize, find_relative_path},
};
use miette::{IntoDiagnostic, Result};

use crate::{ServerState, text_buffer_conversions::TextBufferConversions, uri::UriExt};

fn parse_file_rename_uri(uri_str: &str) -> Result<(Uri, PathBuf)> {
    let uri = Uri::from_str(uri_str).into_diagnostic()?;
    let path = uri
        .to_file_path()
        .map(|c| c.into_owned())
        .ok_or_else(|| miette::miette!("Invalid URI: {}", uri_str))?;

    Ok((uri, path))
}

pub fn process_will_rename_files(
    lsp: &mut ServerState,
    params: RenameFilesParams,
) -> Result<Option<WorkspaceEdit>> {
    let files = params.files;

    #[allow(clippy::mutable_key_type)]
    let mut changes: HashMap<Uri, Vec<TextEdit>> = HashMap::new();

    for file in &files {
        let (old_uri, old_path) = parse_file_rename_uri(&file.old_uri)?;
        let (new_uri, new_path) = parse_file_rename_uri(&file.new_uri)?;

        // update references connected to the changed file
        for (doc, link) in find_references_to_uri(lsp, &old_uri) {
            let new_rel = find_relative_path(&doc.path, &new_path)?;
            let new_text = link.render_with_target(&doc.source, &new_rel);

            let Some(doc_uri) = Uri::from_file_path(&doc.path) else {
                tracing::debug!("Failed to convert path to URI: {:?}", doc.path);
                continue;
            };

            let range = doc.source.slice(..).byte_to_lsp_range(link.span);
            changes.entry(doc_uri).or_default().push(TextEdit::new(range, new_text));
        }

        // update references in the moved file
        if let Some(doc) = lsp.documents.get_document(&old_path) {
            for edit in doc.links().filter_map(|link| {
                let target = link.target_str(&doc.source);
                let resolved = combine_and_normalize(&old_path, &target).ok()?;
                let new_rel = find_relative_path(&new_path, resolved).ok()?;
                let new_text = link.render_with_target(&doc.source, &new_rel);
                let range = doc.source.slice(..).byte_to_lsp_range(link.span);
                Some(TextEdit::new(range, new_text))
            }) {
                changes.entry(new_uri.clone()).or_default().push(edit);
            }
        }
    }

    Ok(Some(WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    }))
}

// Find all links that resolve to a matching uri
fn find_references_to_uri<'a>(
    lsp: &'a ServerState,
    match_uri: &Uri,
) -> impl Iterator<Item = (&'a Document, &'a Link)> {
    let match_path: Option<PathBuf> = match_uri.to_file_path().map(|c| c.into_owned());

    lsp.documents.iter().flat_map(move |doc| {
        let match_path = match_path.clone();

        doc.links().filter_map(move |link| {
            let target = link.target_str(&doc.source);
            let resolved_path = combine_and_normalize(&doc.path, &target).ok()?;

            if let Some(ref m_path) = match_path
                && m_path == &resolved_path
            {
                return Some((doc, link));
            }

            None
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
