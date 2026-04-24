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
                tracing::warn!("Unimplemented request method: {}", method);
            }
        }
    };
}

#[macro_export]
macro_rules! dispatch_lsp_notification {
    ($lsp:expr, $notification:expr, {
        $($notification_type:path => $handler:ident),* $(,)?
    }) => {
        match $notification.method.as_str() {
            $(
                <$notification_type as lsp_types::notification::Notification>::METHOD => {
                    handle_notification::<$notification_type, _>($lsp, $notification, $handler)?;
                }
            )*
            method => {
                tracing::warn!("Unimplemented notification method: {}", method);
            }
        }
    };
}

#[macro_export]
macro_rules! get_document {
    ($state:expr, $uri:expr) => {
        $state
            .documents
            .get_document($uri)
            .with_context(|| format!("Document '{}' not found", $uri.as_str()))?
    };
}

#[macro_export]
macro_rules! get_document_mut {
    ($state:expr, $uri:expr) => {
        $state
            .documents
            .get_document_mut($uri)
            .with_context(|| format!("Document '{}' not found", $uri.as_str()))?
    };
}
