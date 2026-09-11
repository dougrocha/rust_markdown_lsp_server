use std::str::FromStr;

use gen_lsp_types::{RenameFilesParams, Uri};
use miette::Result;
use tracing::trace;

use crate::{
    ServerState,
    handlers::rename::{apply_edits, moved_file_link_edits},
    uri::UriExt,
};

pub fn process_did_rename(lsp: &mut ServerState, params: RenameFilesParams) -> Result<()> {
    trace!(?params);

    for file in &params.files {
        let (Ok(old_uri), Ok(new_uri)) = (
            Uri::from_str(&file.old_uri),
            Uri::from_str(&file.new_uri),
        ) else {
            continue;
        };

        let (Some(old_path), Some(new_path)) = (old_uri.to_file_path(), new_uri.to_file_path())
        else {
            continue;
        };

        // Rewrite the moved file's own links here too, because the client echoes will_rename edits back only for open buffers.
        let cx = lsp.vault_context();
        let Some(doc) = cx.vault.get_document(&old_path) else {
            continue;
        };
        let version = doc.version;
        let mut text = doc.source.to_string();
        apply_edits(&mut text, moved_file_link_edits(doc, &old_path, &new_path, &cx));
        drop(cx);

        let was_open = lsp.is_document_open(&old_path);

        lsp.documents
            .rename_document(&old_path, &new_path, version, &text)?;

        // Move the open-document marker so pull diagnostics still fire for the renamed buffer.
        if was_open {
            lsp.close_document(&old_path);
            lsp.open_document(new_path.to_path_buf());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use gen_lsp_types::{FileRename, RenameFilesParams};

    use super::*;
    use crate::test_utils::TestWorkspace;

    fn rename_params(old_uri: &str, new_uri: &str) -> RenameFilesParams {
        RenameFilesParams {
            files: vec![FileRename {
                old_uri: old_uri.to_string(),
                new_uri: new_uri.to_string(),
            }],
        }
    }

    #[test]
    fn did_rename_moves_document_and_reindexes() {
        let mut ws = TestWorkspace::new();
        ws.add_file("/workspace/old.md", 1, "# Old");

        let params = rename_params(
            "file:///workspace/old.md",
            "file:///workspace/new.md",
        );
        process_did_rename(&mut ws.state, params).unwrap();

        let docs = &ws.state.documents;
        assert!(docs.get_document(Path::new("/workspace/old.md")).is_none());
        assert!(docs.get_document(Path::new("/workspace/new.md")).is_some());
        assert_eq!(
            docs.index().resolve_name("new"),
            Some(Path::new("/workspace/new.md"))
        );
        assert_eq!(docs.index().resolve_name("old"), None);
    }

    #[test]
    fn did_rename_moves_the_open_marker_to_the_new_path() {
        let mut ws = TestWorkspace::new();
        ws.open_file("/workspace/old.md", 1, "# Old");

        process_did_rename(
            &mut ws.state,
            rename_params("file:///workspace/old.md", "file:///workspace/new.md"),
        )
        .unwrap();

        assert!(!ws.state.is_document_open(Path::new("/workspace/old.md")));
        assert!(ws.state.is_document_open(Path::new("/workspace/new.md")));
    }

    #[test]
    fn did_rename_rewrites_links_inside_a_moved_closed_file() {
        let mut ws = TestWorkspace::new();
        ws.add_file("/workspace/notes.md", 1, "# Notes");
        ws.add_file("/workspace/hub.md", 1, "[n](./notes.md)");

        process_did_rename(
            &mut ws.state,
            rename_params("file:///workspace/hub.md", "file:///workspace/sub/hub.md"),
        )
        .unwrap();

        let doc = ws
            .state
            .documents
            .get_document(Path::new("/workspace/sub/hub.md"))
            .unwrap();
        assert_eq!(doc.source.to_string(), "[n](../notes.md)");
    }
}
