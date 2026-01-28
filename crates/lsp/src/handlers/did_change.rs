use lsp_types::{DidChangeTextDocumentParams, TextDocumentContentChangeEvent};
use miette::Result;

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
            todo!("Incremental Changes Not Supported!");
        }
    }

    Ok(())
}
