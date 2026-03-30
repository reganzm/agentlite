use async_openai::{Client, config::OpenAIConfig};
use serde_json::{Value, json};

use crate::toolkit::{new_trace_id, ToolCatalog};

pub struct Agent {
    client: Client<OpenAIConfig>,
    model: String,
    messages: Vec<Value>,
    tools: Value,
    catalog: ToolCatalog,
    /// User/session scope for audit: in-product this should be **one id per chat** (from host).
    /// CLI placeholder: new id each run unless `AGENTLITE_SESSION_ID` is set.
    session_id: String,
    /// One id per user task (`run`); all tool audit lines share this for correlation.
    trace_id: String,
}

impl Agent {
    pub fn new(
        client: Client<OpenAIConfig>,
        model: &str,
        catalog: ToolCatalog,
        session_id: String,
    ) -> Self {
        Self::with_session_and_trace(client, model, catalog, session_id, new_trace_id())
    }

    /// Same as [`Agent::new`] but fixes both session and trace (e.g. external tracing).
    pub fn with_session_and_trace(
        client: Client<OpenAIConfig>,
        model: &str,
        catalog: ToolCatalog,
        session_id: String,
        trace_id: String,
    ) -> Self {
        let tools = catalog.openai_definitions();
        Self {
            client,
            model: model.to_string(),
            messages: Vec::new(),
            tools,
            catalog,
            session_id,
            trace_id,
        }
    }

    /// Id for this run; same value on every tool audit line for this task.
    #[allow(dead_code)]
    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    /// Session id for audit (future: one per chat from host; CLI: placeholder per process unless env set).
    #[allow(dead_code)]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn catalog_mut(&mut self) -> &mut ToolCatalog {
        &mut self.catalog
    }

    /// Add a user message to start the conversation
    pub fn add_user_message(&mut self, content: &str) {
        self.messages.push(json!({
            "role": "user",
            "content": content
        }));
    }

    /// Run the agent loop until completion
    pub async fn run(&mut self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        loop {
            let response = self.send_request().await?;
            let message = response["choices"][0]["message"].clone();

            self.messages.push(message.clone());

            if let Some(tool_calls) = message["tool_calls"].as_array() {
                if tool_calls.is_empty() {
                    return Ok(self.extract_content(&message));
                }

                for tool_call in tool_calls {
                    self.handle_tool_call(tool_call).await;
                }
            } else {
                return Ok(self.extract_content(&message));
            }
        }
    }

    async fn send_request(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let response: Value = self
            .client
            .chat()
            .create_byot(json!({
                "messages": self.messages,
                "model": self.model,
                "tools": self.tools
            }))
            .await?;

        Ok(response)
    }

    async fn handle_tool_call(&mut self, tool_call: &Value) {
        let tool_call_id = tool_call["id"].as_str().unwrap_or("");
        let function_name = tool_call["function"]["name"].as_str().unwrap_or("");
        let arguments_str = tool_call["function"]["arguments"].as_str().unwrap_or("{}");
        let arguments: Value = serde_json::from_str(arguments_str).unwrap_or(json!({}));

        let result = self
            .catalog
            .execute(
                &self.session_id,
                &self.trace_id,
                Some(tool_call_id).filter(|s| !s.is_empty()),
                function_name,
                &arguments,
            )
            .await;

        self.messages.push(json!({
            "role": "tool",
            "tool_call_id": tool_call_id,
            "content": result
        }));
    }

    fn extract_content(&self, message: &Value) -> String {
        message["content"].as_str().unwrap_or("").to_string()
    }
}
