//! Read-only, bounded MCP adapter. Provider calls remain synchronous and local to workers.
pub mod dto;
mod frame;
pub mod projection;
mod protocol;
pub use protocol::{Executor, run_stdio, serve_io};
