use async_openai::{Client, config::OpenAIConfig};
use serde_json::{Value, json};

use crate::tools::{execute_tool, get_tool_definitions};

pub struct Agent {
    client: Client<OpenAIConfig>,
    model: String,
    messages: Vec<Value>,
    tools: Value,
}

impl Agent {
    pub fn new(client: Client<OpenAIConfig>, model: &str) -> Self {
        Self {
            client,
            model: model.to_string(),
            messages: Vec::new(),
            tools: get_tool_definitions(),
        }
    }

    /// Add a user message to start the conversation
    pub fn add_user_message(&mut self, content: &str) {
        self.messages.push(json!({
            "role": "user",
            "content": content
        }));
    }

    /// Run the agent loop until completion
    pub async fn run(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        loop {
            let response = self.send_request().await?;
            let message = response["choices"][0]["message"].clone();

            // Add assistant message to history
            self.messages.push(message.clone());

            // Check for tool calls
            if let Some(tool_calls) = message["tool_calls"].as_array() {
                if tool_calls.is_empty() {
                    return Ok(self.extract_content(&message));
                }

                // Execute each tool call
                for tool_call in tool_calls {
                    self.handle_tool_call(tool_call)?;
                }
            } else {
                // No tool calls, return final content
                return Ok(self.extract_content(&message));
            }
        }
    }

    async fn send_request(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let response: Value = self
            .client
            .chat()
            .create_byot(json!({
                "messages": self.messages,
                "model": self.model,
                "tools": self.tools
            }))
            .await?;

        eprintln!("Logs from your program will appear here!");
        eprintln!("{}", response);
        Ok(response)
    }

    fn handle_tool_call(&mut self, tool_call: &Value) -> Result<(), Box<dyn std::error::Error>> {
        let tool_call_id = tool_call["id"].as_str().unwrap_or("");
        let function_name = tool_call["function"]["name"].as_str().unwrap_or("");
        let arguments_str = tool_call["function"]["arguments"].as_str().unwrap_or("{}");
        let arguments: Value = serde_json::from_str(arguments_str)?;

        let result = execute_tool(function_name, &arguments);

        // Add tool result to messages
        self.messages.push(json!({
            "role": "tool",
            "tool_call_id": tool_call_id,
            "content": result
        }));

        Ok(())
    }

    fn extract_content(&self, message: &Value) -> String {
        message["content"].as_str().unwrap_or("").to_string()
    }
}
