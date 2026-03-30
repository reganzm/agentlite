use rmcp::model::Tool;
use serde_json::{Value, json};

/// Convert an MCP [`Tool`] into an OpenAI chat `tools[]` function entry under a stable exposed name.
pub fn mcp_tool_to_openai_function(exposed_name: &str, tool: &Tool) -> Value {
    let description = tool
        .description
        .as_deref()
        .map(|s| s.to_string())
        .unwrap_or_default();

    let parameters = serde_json::to_value(&*tool.input_schema).unwrap_or_else(|_| {
        json!({
            "type": "object",
            "properties": {},
        })
    });

    json!({
        "type": "function",
        "function": {
            "name": exposed_name,
            "description": description,
            "parameters": parameters,
        }
    })
}
