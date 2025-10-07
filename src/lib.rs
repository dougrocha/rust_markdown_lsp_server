use document::references::Reference;

pub mod config;
pub mod document;
pub mod lsp;
pub mod macros;
pub mod message;
pub mod path;
pub mod rpc;
mod text_buffer_conversions;
mod uri;

pub use text_buffer_conversions::TextBufferConversions;
pub use uri::UriExt;
