//! Durable AgentMender-owned transaction knowledge; never provider execution.
mod handoff;
mod owned;
mod recovery;
pub mod search_index;
mod store;
pub use search_index::*;
#[cfg(windows)]
mod windows_permissions;
pub use handoff::*;
pub use owned::*;
pub use recovery::*;
pub use store::*;
#[cfg(test)]
#[path = "../tests/common/mod.rs"]
mod test_fixture;
