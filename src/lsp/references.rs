use lsp_types::{Location, Position, Range, ReferenceParams, Uri};
use miette::{Context, Result};

use crate::UriExt;

use super::server::Server;

pub fn process_references(
    lsp: &mut Server,
    params: ReferenceParams,
) -> Result<Option<Vec<Location>>> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    let document = lsp.documents.get_document(&uri).context(format!(
        "Document '{:?}' not found in workspace",
        uri.as_str()
    ))?;

    let reference = document.get_reference_at_position(position);

    // Check the current file I am on, maybe even header and return all the locations where I am
    // referencing this file in a link

    Ok(Some(vec![
        Location::new(
            Uri::from_file_path("/Users/douglasrocha/dev/rust_markdown_lsp/test.md").unwrap(),
            Range::new(Position::new(0, 0), Position::new(0, 10)),
        ),
        Location::new(
            Uri::from_file_path("/Users/douglasrocha/dev/rust_markdown_lsp/test.md").unwrap(),
            Range::new(Position::new(2, 0), Position::new(2, 10)),
        ),
    ]))
}
