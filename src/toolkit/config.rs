use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct McpConfigFile {
    pub servers: Vec<McpServerEntry>,
}

/// Reads `AGENTLITE_MCP_CONFIG` (path to JSON). Missing or empty env → no MCP servers.
pub fn load_mcp_servers_from_env() -> Result<Vec<McpServerEntry>, Box<dyn std::error::Error + Send + Sync>>
{
    let path = match env::var("AGENTLITE_MCP_CONFIG") {
        Ok(p) if !p.trim().is_empty() => p,
        _ => return Ok(Vec::new()),
    };
    let text = fs::read_to_string(&path)
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { format!("read {path}: {e}").into() })?;
    let cfg: McpConfigFile = serde_json::from_str(&text)?;
    Ok(cfg.servers)
}

#[derive(Debug, Clone, Deserialize)]
pub struct McpServerEntry {
    /// Short id used to prefix tool names, e.g. `fs__read_file`.
    pub label: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}
