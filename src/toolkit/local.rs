use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::process::Command;

use super::NativeToolSet;

/// Built-in filesystem / shell tools (same behavior as the original hard-coded set).
pub struct LocalToolkit;

#[async_trait]
impl NativeToolSet for LocalToolkit {
    fn openai_functions(&self) -> Vec<Value> {
        vec![
            json!({
                "type": "function",
                "function": {
                    "name": "Read",
                    "description": "Read and return the contents of a file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "The path to the file to read"
                            }
                        },
                        "required": ["file_path"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "Write",
                    "description": "Write content to a file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "The path of the file to write to"
                            },
                            "content": {
                                "type": "string",
                                "description": "The content to write to the file"
                            }
                        },
                        "required": ["file_path", "content"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "Bash",
                    "description": "Execute a shell command",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "command": {
                                "type": "string",
                                "description": "The command to execute"
                            }
                        },
                        "required": ["command"]
                    }
                }
            }),
        ]
    }

    async fn invoke(&self, name: &str, arguments: &Value) -> Option<String> {
        match name {
            "Read" => Some(execute_read(arguments).await),
            "Write" => Some(execute_write(arguments).await),
            "Bash" => Some(execute_bash(arguments).await),
            _ => None,
        }
    }
}

async fn execute_read(arguments: &Value) -> String {
    match arguments["file_path"].as_str() {
        Some(file_path) => tokio::fs::read_to_string(file_path)
            .await
            .unwrap_or_else(|e| format!("Error: {}", e)),
        None => "Error: file_path not provided".to_string(),
    }
}

async fn execute_write(arguments: &Value) -> String {
    let file_path = arguments["file_path"].as_str();
    let content = arguments["content"].as_str();

    match (file_path, content) {
        (Some(path), Some(data)) => match tokio::fs::write(path, data.as_bytes()).await {
            Ok(_) => format!("Successfully wrote to {}", path),
            Err(e) => format!("Error writing file: {}", e),
        },
        _ => "Error: file_path and content are required".to_string(),
    }
}

async fn execute_bash(arguments: &Value) -> String {
    match arguments["command"].as_str() {
        Some(command) => {
            let output = {
                #[cfg(windows)]
                {
                    Command::new("cmd").arg("/C").arg(command).output()
                }
                #[cfg(not(windows))]
                {
                    Command::new("sh").arg("-c").arg(command).output()
                }
            };
            match output.await {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.is_empty() {
                        format!("{}{}", stdout, stderr)
                    } else {
                        stdout.to_string()
                    }
                }
                Err(e) => format!("Error executing command: {}", e),
            }
        }
        None => "Error: command not provided".to_string(),
    }
}
