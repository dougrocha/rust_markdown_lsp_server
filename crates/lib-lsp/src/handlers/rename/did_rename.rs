use std::str::FromStr;

use gen_lsp_types::{RenameFilesParams, Uri};
use miette::Result;
use tracing::trace;

use crate::{ServerState, uri::UriExt};

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

        let Some((text, version)) = lsp
            .documents
            .get_document(&old_path)
            .map(|doc| (doc.source.to_string(), doc.version))
        else {
            continue;
        };

        lsp.documents
            .rename_document(&old_path, &new_path, version, &text)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use gen_lsp_types::{FileRename, RenameFilesParams};

    use super::*;
    use crate::test_utils::TestWorkspace;

    #[test]
    fn did_rename_moves_document_and_reindexes() {
        let mut ws = TestWorkspace::new();
        ws.add_file("/workspace/old.md", 1, "# Old");

        let params = RenameFilesParams {
            files: vec![FileRename {
                old_uri: "file:///workspace/old.md".to_string(),
                new_uri: "file:///workspace/new.md".to_string(),
            }],
        };
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
}
