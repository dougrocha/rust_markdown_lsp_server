use lsp_types::DidCloseTextDocumentParams;
use miette::Result;
use tracing::trace;

use crate::server::Server;

pub fn process_did_close(lsp: &mut Server, params: DidCloseTextDocumentParams) -> Result<()> {
    let uri = params.text_document.uri;

    trace!("Closing file {}", uri.as_str());

    if let Some(doc) = lsp.documents.get_document_mut(&uri) {
        doc.close();
    }

    Ok(())
}
