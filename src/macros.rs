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
