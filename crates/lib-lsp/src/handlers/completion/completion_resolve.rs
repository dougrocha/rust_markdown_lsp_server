use lsp_types::CompletionItem;
use miette::Result;

use crate::server::Server;

pub fn process_completion_resolve(
    _lsp: &mut Server,
    item: CompletionItem,
) -> Result<CompletionItem> {
    Ok(item)
}
