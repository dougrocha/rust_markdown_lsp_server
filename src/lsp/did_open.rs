use serde::Deserialize;

use crate::{message::Notification, LspServer};

use super::{TextDocumentItem, URI};

#[derive(Deserialize, Debug)]
pub struct DidOpenTextDocumentParams {
    #[serde(rename = "textDocument")]
    text_document: TextDocumentItem,
}

pub fn process_did_open(lsp: &mut LspServer, notification: Notification) {
    let did_open_params: DidOpenTextDocumentParams =
        serde_json::from_value(notification.params).unwrap();

    let URI(uri) = did_open_params.text_document.uri;

    lsp.open_document(&uri, &did_open_params.text_document.text);
}
