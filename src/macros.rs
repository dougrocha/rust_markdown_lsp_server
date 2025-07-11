#[macro_export]
macro_rules! dispatch_lsp_request {
    ($lsp:expr, $request:expr, $writer:expr, {
        $($req_type:path => $handler:ident),* $(,)?
    }) => {
        match $request.method.as_str() {
            $(
                <$req_type as lsp_types::request::Request>::METHOD => {
                    if let Err(e) = handle_request::<$req_type, _, _>($lsp, $request, $writer, $handler) {
                        log::error!("Failed to handle {}: {}", stringify!($req_type), e);
                    }
                }
            )*
            method => {
                log::warn!("Unimplemented request method: {}", method);
            }
        }
    };
}

#[macro_export]
macro_rules! get_document {
    ($lsp:expr, $uri:expr) => {
        $lsp.documents
            .get_document($uri)
            .with_context(|| format!("Document '{}' not found", $uri.as_str()))?
    };
}

#[macro_export]
macro_rules! get_document_mut {
    ($lsp:expr, $uri:expr) => {
        $lsp.documents
            .get_document_mut($uri)
            .with_context(|| format!("Document '{}' not found", $uri.as_str()))?
    };
}
