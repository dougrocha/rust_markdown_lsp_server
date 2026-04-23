use lib_core::{document::references::ReferenceKind, path::extract_filename_stem};
use lsp_types::{
    Location, SymbolInformation, SymbolKind, WorkspaceSymbolParams, WorkspaceSymbolResponse,
};
use miette::Result;

use crate::server::Server;

pub fn process_workspace_symbol(
    lsp: &mut Server,
    params: WorkspaceSymbolParams,
) -> Result<Option<WorkspaceSymbolResponse>> {
    let query = params.query.to_lowercase();

    let symbols = lsp
        .documents
        .iter()
        .flat_map(|doc| {
            let container_name = extract_filename_stem(&doc.uri);
            let uri = doc.uri.clone();
            let q = query.as_str();
            doc.references.iter().filter_map(move |r| {
                let ReferenceKind::Header { content, .. } = &r.kind else {
                    return None;
                };

                if !q.is_empty() && !content.to_lowercase().contains(q) {
                    return None;
                }

                #[allow(deprecated)]
                Some(SymbolInformation {
                    name: content.clone(),
                    kind: SymbolKind::STRING,
                    tags: None,
                    deprecated: None,
                    location: Location::new(uri.clone(), r.range),
                    container_name: container_name.clone(),
                })
            })
        })
        .collect();

    Ok(Some(WorkspaceSymbolResponse::Flat(symbols)))
}
