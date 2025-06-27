#[macro_export]
macro_rules! dispatch_lsp_request {
    ($lsp:expr, $request:expr, $writer:expr, {
        $($req_type:path => $handler:ident),* $(,)?
    }) => {
        match $request.method.as_str() {
            $(
                <$req_type as lsp_types::request::Request>::METHOD => {
                    handle_request::<$req_type, _, _>($lsp, $request, $writer, $handler)?;
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
        $lsp.documents.get_document($uri).context(format!(
            "Document '{:?}' not found in workspace",
            $uri.as_str()
        ))?
    };
}

#[macro_export]
macro_rules! get_document_mut {
    ($lsp:expr, $uri:expr) => {
        $lsp.documents.get_document_mut($uri).context(format!(
            "Document '{:?}' not found in workspace",
            $uri.as_str()
        ))?
    };
}
