use gen_lsp_types::{
    BaseSymbolInformation, Location, SymbolInformation, SymbolKind, Uri, WorkspaceSymbolParams,
    WorkspaceSymbolResponse,
};
use lib_core::{document::references::ReferenceKind, path::extract_filename_stem};
use miette::Result;

use crate::{server_state::ServerState, uri::UriExt};

pub fn process_workspace_symbol(
    lsp: &mut ServerState,
    params: WorkspaceSymbolParams,
) -> Result<Option<WorkspaceSymbolResponse>> {
    let query = &params.query.to_lowercase();

    let symbols: Vec<SymbolInformation> = lsp
        .documents
        .iter()
        .flat_map(|doc| {
            let container_name = extract_filename_stem(&doc.path);
            let uri = Uri::from_file_path(doc.path.as_path());

            doc.references.iter().filter_map(move |r| {
                let ReferenceKind::Header { content, .. } = &r.kind else {
                    return None;
                };

                if !query.is_empty() && !content.to_lowercase().contains(query) {
                    return None;
                }

                Some(SymbolInformation::new(
                    None,
                    Location::new(uri.as_ref()?.clone(), r.range),
                    BaseSymbolInformation::new(
                        content.clone(),
                        SymbolKind::String,
                        None,
                        container_name.clone(),
                    ),
                ))
            })
        })
        .collect();

    Ok(Some(WorkspaceSymbolResponse::SymbolInformationList(
        symbols,
    )))
}
