use gen_lsp_types::DidCloseTextDocumentParams;
use miette::{Result, miette};
use tracing::trace;

use crate::{server_state::ServerState, uri::UriExt};

pub fn process_did_close(lsp: &mut ServerState, params: DidCloseTextDocumentParams) -> Result<()> {
    let uri = params.text_document.uri;

    trace!("Closing file {}", uri.as_ref());

    let path = uri
        .to_file_path()
        .ok_or_else(|| miette!("Invalid URI: {}", uri.as_ref()))?;

    lsp.documents.close_document(&path);

    Ok(())
}
