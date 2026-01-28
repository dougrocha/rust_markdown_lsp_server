use lsp_types::DidOpenTextDocumentParams;
use miette::Result;

use crate::server::Server;

pub fn process_did_open(lsp: &mut Server, params: DidOpenTextDocumentParams) -> Result<()> {
    lsp.documents.open_document(
        &params.text_document.uri,
        params.text_document.version,
        &params.text_document.text,
    )?;

    Ok(())
}
