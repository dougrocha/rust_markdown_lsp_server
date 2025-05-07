use serde::Deserialize;

use crate::message::Notification;

use super::{server::LspServer, TextDocumentItem};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DidOpenTextDocumentParams {
    text_document: TextDocumentItem,
}

pub fn process_did_open(lsp: &mut LspServer, notification: Notification) {
    let did_open_params: DidOpenTextDocumentParams =
        serde_json::from_value(notification.params).unwrap();

    lsp.open_document(
        did_open_params.text_document.uri,
        &did_open_params.text_document.text,
    );
}
