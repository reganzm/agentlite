#![allow(unused_imports)] // `pub use` is the crate API surface; the binary does not reference every item.

//! Tooling layer: OpenAI-style function definitions, local in-process tools, and MCP-backed tools.
//!
//! Dispatch is unified: [`ToolCatalog::execute`] walks [`ToolBackend`] entries in order and uses
//! [`ToolBackend::try_call`] (local [`NativeToolSet::invoke`] vs MCP [`McpBridge`]).
//!
//! - Register additional in-process tools by implementing [`NativeToolSet`] and passing
//!   `Arc<dyn NativeToolSet>` into [`ToolCatalogBuilder::connect`].
//! - Register MCP servers via JSON (see [`config::McpConfigFile`]) and `AGENTLITE_MCP_CONFIG`.

mod audit;
mod catalog;
mod config;
mod local;
mod openai;
mod session;

pub use audit::{executable_dir, new_trace_id, ToolAuditLog, ToolAuditRecord};
pub use session::resolve_session_id;
pub use catalog::{McpBridge, ToolBackend, ToolCatalog, ToolCatalogBuilder};
pub use config::{McpConfigFile, McpServerEntry};
pub use local::LocalToolkit;
pub use openai::mcp_tool_to_openai_function;

/// In-process tools exposed to the model as OpenAI `function` entries.
///
/// Implement this trait to add custom native tools; combine instances in a [`ToolCatalogBuilder`].
#[async_trait::async_trait]
pub trait NativeToolSet: Send + Sync {
    fn openai_functions(&self) -> Vec<serde_json::Value>;

    /// Return `None` if this set does not handle `name`.
    async fn invoke(&self, name: &str, arguments: &serde_json::Value) -> Option<String>;
}
