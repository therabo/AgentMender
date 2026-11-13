//! Bounded MCP adapter. Native deletion uses a separate, human-confirmed dispatch.
mod conversation;
pub mod dto;
mod find;
mod frame;
pub use find::{FindExecutor, FindRequest, ScopedResolver};
pub mod projection;
mod protocol;
pub use conversation::ConversationExecutor;
pub use protocol::{
    Executor, NativeDeleteExecutor, SessionSearch, run_stdio, run_stdio_with_conversation,
    serve_io, serve_io_with_conversation,
};
