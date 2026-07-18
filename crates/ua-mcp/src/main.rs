//! Understand Anything MCP Server
//!
//! Exposes code analysis tools via stdio JSON-RPC 2.0.
//! Tools: understand_scan, understand_graph, understand_search

use std::io::{BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize)]
struct Request {
    id: Option<Value>,
    method: Option<String>,
    params: Option<Value>,
}

#[derive(Serialize)]
struct Response {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

fn main() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        let req: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let method = req.method.as_deref().unwrap_or("");
        let id = req.id.unwrap_or(Value::Null);

        let result = match method {
            "initialize" => Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "understand_anything", "version": "0.1.0"}
            })),
            "tools/list" => Some(json!({"tools": [
                {
                    "name": "understand_scan",
                    "description": "Scan a project directory and return file inventory with language detection and complexity estimation",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "Project root directory path"}
                        },
                        "required": ["path"]
                    }
                }
            ]})),
            "tools/call" => {
                let params = req.params.unwrap_or_default();
                let name = params["name"].as_str().unwrap_or("");
                let args = &params["arguments"];

                match name {
                    "understand_scan" => {
                        let path = args["path"].as_str().unwrap_or(".");
                        match ua_core::scanner::scan_project(std::path::Path::new(path)) {
                            Ok(result) => {
                                let json = serde_json::to_value(&result).unwrap_or_default();
                                Some(
                                    json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&json).unwrap_or_default()}]}),
                                )
                            }
                            Err(e) => Some(
                                json!({"content": [{"type": "text", "text": format!("Error: {}", e)}], "isError": true}),
                            ),
                        }
                    }
                    _ => Some(
                        json!({"content": [{"type": "text", "text": format!("Unknown tool: {}", name)}], "isError": true}),
                    ),
                }
            }
            _ => None,
        };

        let resp = Response {
            jsonrpc: "2.0".into(),
            id,
            result,
            error: None,
        };

        let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
        let _ = stdout.flush();
    }
}
