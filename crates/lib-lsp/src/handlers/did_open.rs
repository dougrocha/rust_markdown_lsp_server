use gen_lsp_types::DidOpenTextDocumentParams;
use miette::{Result, miette};
use tracing::trace;

use crate::{server_state::ServerState, uri::UriExt};

pub fn process_did_open(lsp: &mut ServerState, params: DidOpenTextDocumentParams) -> Result<()> {
    let uri = params.text_document.uri;

    trace!("Opening file {}", uri.as_ref());

    let path = uri
        .to_file_path()
        .ok_or_else(|| miette!("Invalid URI: {}", uri.as_ref()))?;

    lsp.documents.open_document(
        &path,
        params.text_document.version,
        &params.text_document.text,
    )?;

    Ok(())
}
