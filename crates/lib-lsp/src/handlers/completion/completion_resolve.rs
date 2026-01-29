use miette::Result;

use lsp_types::CompletionItem;

use crate::server::Server;

pub fn process_completion_resolve(
    _lsp: &mut Server,
    item: CompletionItem,
) -> Result<CompletionItem> {
    // log::debug!("Completion Resolve {:#?}", item);
    Ok(CompletionItem {
        label: item.label,
        ..Default::default()
    })
}
