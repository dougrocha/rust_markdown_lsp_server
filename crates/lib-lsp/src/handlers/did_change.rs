use lsp_types::{DidChangeTextDocumentParams, TextDocumentContentChangeEvent};
use miette::{Result, miette};

use crate::server::Server;

pub fn process_did_change(lsp: &mut Server, params: DidChangeTextDocumentParams) -> Result<()> {
    let uri = params.text_document.uri;

    for change in params.content_changes {
        let TextDocumentContentChangeEvent {
            text,
            range,
            range_length,
        } = change;
        if range.is_none() && range_length.is_none() {
            lsp.documents.update_document(&uri, &text)?;
        } else {
            return Err(miette!(
                "Incremental document changes are not yet supported. \
     Please configure your editor to send full document updates."
            ));
        }
    }

    Ok(())
}
