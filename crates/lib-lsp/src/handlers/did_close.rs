use lib_core::uri::UriExt;
use lsp_types::DidCloseTextDocumentParams;
use miette::Result;

use crate::server::Server;

pub fn process_did_close(lsp: &mut Server, params: DidCloseTextDocumentParams) -> Result<()> {
    let uri = params.text_document.uri;

    let Some(path) = uri.to_file_path() else {
        lsp.documents.remove_document(&uri);
        return Ok(());
    };

    match std::fs::read_to_string(&path) {
        Ok(contents) => lsp.documents.update_document(&uri, &contents)?,
        Err(_) => lsp.documents.remove_document(&uri),
    }

    Ok(())
}
