//! Durable AgentMender-owned transaction knowledge; never provider execution.
mod owned;
mod recovery;
mod store;
#[cfg(windows)]
mod windows_permissions;
pub use owned::*;
pub use recovery::*;
pub use store::*;
#[cfg(test)]
#[path = "../tests/common/mod.rs"]
mod test_fixture;
