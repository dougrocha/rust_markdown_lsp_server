use lsp_types::DidOpenTextDocumentParams;
use miette::Result;
use tracing::trace;

use crate::server::Server;

pub fn process_did_open(lsp: &mut Server, params: DidOpenTextDocumentParams) -> Result<()> {
    let uri = params.text_document.uri;

    trace!("Opening file {}", uri.as_str());

    lsp.documents.open_document(
        uri,
        params.text_document.version,
        &params.text_document.text,
    )?;

    Ok(())
}
