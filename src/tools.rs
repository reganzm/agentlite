use serde_json::{Value, json};
use std::fs;
use std::process::Command;

/// Returns the tool specifications for the LLM
pub fn get_tool_definitions() -> Value {
    json!([
        {
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
        },
        {
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
        },
        {
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
        }
    ])
}

/// Execute a tool call and return the result
pub fn execute_tool(function_name: &str, arguments: &Value) -> String {
    match function_name {
        "Read" => execute_read(arguments),
        "Write" => execute_write(arguments),
        "Bash" => execute_bash(arguments),
        _ => format!("Error: Unknown function {}", function_name),
    }
}

fn execute_read(arguments: &Value) -> String {
    match arguments["file_path"].as_str() {
        Some(file_path) => {
            fs::read_to_string(file_path).unwrap_or_else(|e| format!("Error: {}", e))
        }
        None => "Error: file_path not provided".to_string(),
    }
}

fn execute_write(arguments: &Value) -> String {
    let file_path = arguments["file_path"].as_str();
    let content = arguments["content"].as_str();

    match (file_path, content) {
        (Some(path), Some(data)) => match fs::write(path, data) {
            Ok(_) => format!("Successfully wrote to {}", path),
            Err(e) => format!("Error writing file: {}", e),
        },
        _ => "Error: file_path and content are required".to_string(),
    }
}

fn execute_bash(arguments: &Value) -> String {
    match arguments["command"].as_str() {
        Some(command) => match Command::new("sh").arg("-c").arg(command).output() {
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
        },
        None => "Error: command not provided".to_string(),
    }
}
