use std::{collections::HashMap, path::PathBuf, str::FromStr};

use gen_lsp_types::{RenameFilesParams, TextEdit, Uri, WorkspaceEdit};
use lib_core::{
    path::{combine_and_normalize, find_relative_path},
    resolver::{AnchorMatch, ReferenceTarget, find_link_references},
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

    let cx = lsp.vault_context();

    for file in &files {
        let (_old_uri, old_path) = parse_file_rename_uri(&file.old_uri)?;
        let (new_uri, new_path) = parse_file_rename_uri(&file.new_uri)?;

        // update references connected to the changed file
        let target = ReferenceTarget {
            file: old_path.clone(),
            anchor: AnchorMatch::Any,
        };
        let referencing_links: Vec<_> = find_link_references(&target, &cx)
            .map(|link_ref| (link_ref.document, link_ref.link))
            .collect();

        for (doc, link) in referencing_links {
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

#[cfg(test)]
mod tests {
    use gen_lsp_types::{FileRename, RenameFilesParams};

    use super::process_will_rename_files;
    use crate::test_utils::TestWorkspace;

    fn rename_uris(state: &mut crate::ServerState, old_uri: &str, new_uri: &str) -> Vec<(String, String)> {
        let params = RenameFilesParams {
            files: vec![FileRename {
                old_uri: old_uri.to_string(),
                new_uri: new_uri.to_string(),
            }],
        };
        let edit = process_will_rename_files(state, params).unwrap().unwrap();
        edit.changes
            .unwrap_or_default()
            .into_iter()
            .flat_map(|(uri, edits)| {
                edits
                    .into_iter()
                    .map(move |e| (uri.to_string(), e.new_text))
            })
            .collect()
    }

    #[test]
    fn rename_in_one_root_leaves_the_other_root_untouched() {
        let mut ws = TestWorkspace::new();
        ws.state.insert_root("file:///rootA".parse().unwrap());
        ws.state.insert_root("file:///rootB".parse().unwrap());
        ws.add_file("/rootA/note.md", 1, "# Note")
            .add_file("/rootA/ref.md", 1, "[x](/note.md)")
            .add_file("/rootB/note.md", 1, "# Note")
            .add_file("/rootB/ref.md", 1, "[x](/note.md)");

        let edits = rename_uris(
            &mut ws.state,
            "file:///rootA/note.md",
            "file:///rootA/renamed.md",
        );

        assert_eq!(
            edits,
            vec![(
                "file:///rootA/ref.md".to_string(),
                "[x](./renamed.md)".to_string()
            )]
        );
    }

    #[test]
    fn rename_in_nested_root_leaves_the_outer_root_untouched() {
        let mut ws = TestWorkspace::new();
        ws.state.insert_root("file:///vault".parse().unwrap());
        ws.state.insert_root("file:///vault/sub".parse().unwrap());
        ws.add_file("/vault/note.md", 1, "# Outer")
            .add_file("/vault/ref.md", 1, "[x](/note.md)")
            .add_file("/vault/sub/note.md", 1, "# Inner")
            .add_file("/vault/sub/ref.md", 1, "[x](/note.md)");

        let edits = rename_uris(
            &mut ws.state,
            "file:///vault/sub/note.md",
            "file:///vault/sub/renamed.md",
        );

        assert_eq!(
            edits,
            vec![(
                "file:///vault/sub/ref.md".to_string(),
                "[x](./renamed.md)".to_string()
            )]
        );
    }

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
    fn rename_updates_wikilink_in_referencing_doc() {
        let mut ws = TestWorkspace::new();

        ws.add_file("/workspace/notes.md", 1, "see [[target]] here")
            .add_file("/workspace/target.md", 1, "# Target");

        let changes = ws.rename("target.md", "renamed.md");

        let edits = changes.get("/workspace/notes.md").unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "[[./renamed.md]]");
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
