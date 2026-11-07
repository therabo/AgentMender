//! Read-only, bounded MCP adapter. Provider calls remain synchronous and local to workers.
mod conversation;
pub mod dto;
mod frame;
pub mod projection;
mod protocol;
pub use conversation::ConversationExecutor;
pub use protocol::{
    Executor, run_stdio, run_stdio_with_conversation, serve_io, serve_io_with_conversation,
};
