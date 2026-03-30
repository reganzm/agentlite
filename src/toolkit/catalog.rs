use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, CallToolResult},
    service::ServiceError,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use serde_json::Value;

use super::audit::{
    classify_status, invoked_at_timestamp_ms, preview_result, utc_timestamp, ToolAuditLog,
    ToolAuditRecord,
};
use super::config::{McpServerEntry, load_mcp_servers_from_env};
use super::local::LocalToolkit;
use super::openai::mcp_tool_to_openai_function;
use super::NativeToolSet;

/// One transport for tools: either in-process ([`NativeToolSet`]) or one MCP stdio server.
pub enum ToolBackend {
    /// Local tools: same process, async [`NativeToolSet::invoke`].
    InProcess(Arc<dyn NativeToolSet>),
    /// Remote MCP tools over stdio ([`McpBridge`]).
    Mcp(McpBridge),
}

impl ToolBackend {
    /// Returns `None` if this backend does not own `name`; otherwise the tool result (including MCP errors as text).
    pub async fn try_call(&self, name: &str, args: &Value) -> Option<String> {
        match self {
            ToolBackend::InProcess(n) => n.invoke(name, args).await,
            ToolBackend::Mcp(bridge) => bridge.try_call_remote(name, args).await,
        }
    }

    pub fn backend_kind(&self) -> &'static str {
        match self {
            ToolBackend::InProcess(_) => "in_process",
            ToolBackend::Mcp(_) => "mcp",
        }
    }

    pub fn mcp_resolved_tool_name(&self, exposed: &str) -> Option<String> {
        match self {
            ToolBackend::Mcp(b) => b.routes.get(exposed).cloned(),
            ToolBackend::InProcess(_) => None,
        }
    }
}

/// One MCP server: stdio child + mapping from exposed OpenAI names to server tool names.
pub struct McpBridge {
    /// Exposed (prefixed) OpenAI name -> MCP tool name on the server.
    pub(crate) routes: HashMap<String, String>,
    pub(crate) service: rmcp::service::RunningService<rmcp::RoleClient, ()>,
}

impl McpBridge {
    async fn call(&self, exposed_name: &str, arguments: &Value) -> Result<String, ServiceError> {
        let Some(mcp_name) = self.routes.get(exposed_name) else {
            return Ok(format!("Error: unknown MCP route {}", exposed_name));
        };
        let args_map = arguments.as_object().cloned().unwrap_or_default();
        let params = CallToolRequestParams::new(mcp_name.clone()).with_arguments(args_map);
        let result = self.service.call_tool(params).await?;
        Ok(format_call_tool_result(&result))
    }

    async fn try_call_remote(&self, name: &str, args: &Value) -> Option<String> {
        if !self.routes.contains_key(name) {
            return None;
        }
        Some(match self.call(name, args).await {
            Ok(s) => s,
            Err(e) => format!("MCP error: {}", e),
        })
    }

    pub async fn shutdown(&mut self) -> Result<rmcp::service::QuitReason, tokio::task::JoinError> {
        self.service.close().await
    }
}

fn format_call_tool_result(r: &CallToolResult) -> String {
    let mut parts = Vec::new();
    for c in &r.content {
        match c.as_text() {
            Some(t) => parts.push(t.text.clone()),
            None => {
                if let Ok(s) = serde_json::to_string(c) {
                    parts.push(s);
                }
            }
        }
    }
    if let Some(sc) = &r.structured_content {
        parts.push(sc.to_string());
    }
    let body = parts.join("\n");
    if r.is_error == Some(true) {
        format!("Error: {}", body)
    } else {
        body
    }
}

fn sanitize_label(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub struct ToolCatalog {
    backends: Vec<ToolBackend>,
    openai_tools: Value,
    audit: Option<Arc<ToolAuditLog>>,
}

impl ToolCatalog {
    /// Loads optional MCP servers from `AGENTLITE_MCP_CONFIG` and includes [`LocalToolkit`].
    pub async fn bootstrap() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mcp = load_mcp_servers_from_env()?;
        let audit = ToolAuditLog::from_env()?;
        ToolCatalogBuilder::with_default_natives()
            .audit_log(audit)
            .mcp_servers(mcp)
            .connect()
            .await
    }

    pub fn openai_definitions(&self) -> Value {
        self.openai_tools.clone()
    }

    /// `session_id` spans multiple tasks; `trace_id` is one task; `tool_call_id` is one model tool step.
    pub async fn execute(
        &self,
        session_id: &str,
        trace_id: &str,
        tool_call_id: Option<&str>,
        function_name: &str,
        arguments: &Value,
    ) -> String {
        let invoked_at_ms = invoked_at_timestamp_ms();
        let timestamp = utc_timestamp();
        let scan_start = Instant::now();
        for b in &self.backends {
            let t_backend = Instant::now();
            if let Some(out) = b.try_call(function_name, arguments).await {
                let duration_ms = t_backend.elapsed().as_secs_f64() * 1000.0;
                if let Some(ref log) = self.audit {
                    log.record(&ToolAuditRecord {
                        session_id: session_id.to_string(),
                        trace_id: trace_id.to_string(),
                        tool_call_id: tool_call_id.map(String::from),
                        timestamp: timestamp.clone(),
                        invoked_at_ms,
                        tool: function_name.to_string(),
                        arguments: arguments.clone(),
                        backend: b.backend_kind().to_string(),
                        mcp_server_tool: b.mcp_resolved_tool_name(function_name),
                        duration_ms,
                        status: classify_status(&out),
                        result_length: out.len(),
                        result_preview: preview_result(&out),
                    });
                }
                return out;
            }
        }
        let duration_ms = scan_start.elapsed().as_secs_f64() * 1000.0;
        if let Some(ref log) = self.audit {
            log.record(&ToolAuditRecord {
                session_id: session_id.to_string(),
                trace_id: trace_id.to_string(),
                tool_call_id: tool_call_id.map(String::from),
                timestamp,
                invoked_at_ms,
                tool: function_name.to_string(),
                arguments: arguments.clone(),
                backend: "none".to_string(),
                mcp_server_tool: None,
                duration_ms,
                status: "unknown_tool",
                result_length: 0,
                result_preview: String::new(),
            });
        }
        format!("Error: Unknown function {}", function_name)
    }

    /// Gracefully close MCP subprocess connections (in-process backends are kept).
    pub async fn shutdown_mcp(&mut self) {
        let mut kept = Vec::new();
        for b in std::mem::take(&mut self.backends) {
            match b {
                ToolBackend::InProcess(p) => kept.push(ToolBackend::InProcess(p)),
                ToolBackend::Mcp(mut bridge) => {
                    let _ = bridge.shutdown().await;
                }
            }
        }
        self.backends = kept;
    }
}

pub struct ToolCatalogBuilder {
    natives: Vec<Arc<dyn NativeToolSet>>,
    mcp_servers: Vec<McpServerEntry>,
    audit: Option<Arc<ToolAuditLog>>,
}

impl ToolCatalogBuilder {
    pub fn with_default_natives() -> Self {
        Self {
            natives: vec![Arc::new(LocalToolkit)],
            mcp_servers: Vec::new(),
            audit: None,
        }
    }

    /// No in-process tools; use before `register_native` to replace defaults.
    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self {
            natives: Vec::new(),
            mcp_servers: Vec::new(),
            audit: None,
        }
    }

    pub fn audit_log(mut self, log: Option<Arc<ToolAuditLog>>) -> Self {
        self.audit = log;
        self
    }

    #[allow(dead_code)]
    pub fn register_native(mut self, t: Arc<dyn NativeToolSet>) -> Self {
        self.natives.push(t);
        self
    }

    pub fn mcp_servers(mut self, servers: Vec<McpServerEntry>) -> Self {
        self.mcp_servers = servers;
        self
    }

    pub async fn connect(self) -> Result<ToolCatalog, Box<dyn std::error::Error + Send + Sync>> {
        let mut defs = Vec::new();
        let mut seen = HashSet::new();

        for n in &self.natives {
            for d in n.openai_functions() {
                let name = d["function"]["name"]
                    .as_str()
                    .ok_or("native tool missing function.name")?;
                if !seen.insert(name.to_string()) {
                    return Err(format!("duplicate tool name: {}", name).into());
                }
                defs.push(d);
            }
        }

        let mut backends: Vec<ToolBackend> = self
            .natives
            .into_iter()
            .map(ToolBackend::InProcess)
            .collect();

        for entry in &self.mcp_servers {
            let label = sanitize_label(&entry.label);
            let transport =
                TokioChildProcess::new(tokio::process::Command::new(&entry.command).configure(
                    |cmd| {
                        cmd.args(&entry.args);
                        for (k, v) in &entry.env {
                            cmd.env(k, v);
                        }
                    },
                ))?;

            let client = ().serve(transport).await?;
            let tools = client.list_all_tools().await?;
            let mut routes = HashMap::new();

            for tool in tools {
                let inner = tool.name.as_ref();
                let exposed = format!("{}__{}", label, inner);
                if !seen.insert(exposed.clone()) {
                    return Err(format!("duplicate tool name after MCP prefix: {}", exposed).into());
                }
                defs.push(mcp_tool_to_openai_function(&exposed, &tool));
                routes.insert(exposed, inner.to_string());
            }

            backends.push(ToolBackend::Mcp(McpBridge {
                routes,
                service: client,
            }));
        }

        Ok(ToolCatalog {
            backends,
            openai_tools: Value::Array(defs),
            audit: self.audit,
        })
    }
}
