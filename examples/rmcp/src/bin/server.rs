use std::borrow::Cow;
use std::sync::Arc;

use rmcp::{
    ErrorData, RoleServer, ServerHandler, ServiceExt,
    model::{
        CallToolRequestParams, CallToolResult, Content, ListToolsResult, PaginatedRequestParams,
        ServerCapabilities, ServerInfo, Tool, object,
    },
    service::RequestContext,
    transport::stdio,
};
use serde_json::{Value, json};

#[derive(Clone)]
struct AdderServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    AdderServer
        .serve(stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}

impl ServerHandler for AdderServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Adds two numbers (demo MCP server).")
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, ErrorData>> + Send + '_ {
        let mut tool = Tool::default();
        tool.name = Cow::Borrowed("add");
        tool.description = Some(Cow::Borrowed("Add two numbers."));
        tool.input_schema = Arc::new(object(json!({
            "type": "object",
            "properties": {
                "a": { "type": "number" },
                "b": { "type": "number" }
            },
            "required": ["a", "b"]
        })));
        std::future::ready(Ok(ListToolsResult::with_all_items(vec![tool])))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, ErrorData>> + Send + '_ {
        let out = (|| {
            if request.name.as_ref() != "add" {
                return Err(ErrorData::invalid_params("unknown tool", None));
            }
            let args_val = request
                .arguments
                .as_ref()
                .map(|m| Value::Object(m.clone()))
                .unwrap_or_else(|| json!({}));
            let a = args_val["a"]
                .as_f64()
                .ok_or_else(|| ErrorData::invalid_params("missing or invalid `a`", None))?;
            let b = args_val["b"]
                .as_f64()
                .ok_or_else(|| ErrorData::invalid_params("missing or invalid `b`", None))?;
            Ok(CallToolResult::success(vec![Content::text((a + b).to_string())]))
        })();
        std::future::ready(out)
    }
}
