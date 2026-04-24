use lsp_types::DidCloseTextDocumentParams;
use miette::Result;
use tracing::trace;

use crate::server_state::ServerState;

pub fn process_did_close(lsp: &mut ServerState, params: DidCloseTextDocumentParams) -> Result<()> {
    let uri = params.text_document.uri;

    trace!("Closing file {}", uri.as_str());

    lsp.documents.close_document(&uri);

    Ok(())
}
