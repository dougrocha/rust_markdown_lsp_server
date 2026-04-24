use lsp_types::CompletionItem;
use miette::Result;

use crate::server_state::ServerState;

pub fn process_completion_resolve(
    _lsp: &mut ServerState,
    item: CompletionItem,
) -> Result<CompletionItem> {
    Ok(item)
}
