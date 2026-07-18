//! Understand Anything MCP Server
//!
//! Exposes code analysis tools via stdio JSON-RPC 2.0.
//! Tools: understand_scan, understand_graph, understand_report, understand_incremental

use std::io::{BufRead, Write};
use std::path::Path;

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
                "serverInfo": {"name": "understand_anything", "version": "0.2.0"}
            })),
            "tools/list" => Some(json!({"tools": [
                {
                    "name": "understand_scan",
                    "description": "Scan a project directory and return file inventory with language detection and complexity estimation",
                    "inputSchema": {
                        "type": "object",
                        "properties": {"path": {"type": "string", "description": "Project root directory path"}},
                        "required": ["path"]
                    }
                },
                {
                    "name": "understand_graph",
                    "description": "Build a complete knowledge graph (JSON) from a project directory. Returns nodes, edges, layers, and guided tour.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "Project root directory path"},
                            "incremental": {"type": "boolean", "description": "Only re-analyze changed files"}
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": "understand_report",
                    "description": "Generate a human-readable report (HTML or Markdown) from project analysis",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "Project root directory path"},
                            "format": {"type": "string", "enum": ["html", "md", "json"], "description": "Output format"}
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
                        match ua_core::scanner::scan_project(Path::new(path)) {
                            Ok(result) => {
                                let json = serde_json::to_value(&result).unwrap_or_default();
                                Some(json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&json).unwrap_or_default()}]}))
                            }
                            Err(e) => Some(json!({"content": [{"type": "text", "text": format!("Error: {}", e)}], "isError": true})),
                        }
                    }
                    "understand_graph" => {
                        let path = args["path"].as_str().unwrap_or(".");
                        let root = Path::new(path);
                        match build_graph_internal(root) {
                            Ok(graph) => {
                                let json = serde_json::to_value(&graph).unwrap_or_default();
                                Some(json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&json).unwrap_or_default()}]}))
                            }
                            Err(e) => Some(json!({"content": [{"type": "text", "text": format!("Error: {}", e)}], "isError": true})),
                        }
                    }
                    "understand_report" => {
                        let path = args["path"].as_str().unwrap_or(".");
                        let format = args["format"].as_str().unwrap_or("json");
                        let root = Path::new(path);
                        match build_graph_internal(root) {
                            Ok(graph) => {
                                let text = match format {
                                    "html" => ua_core::report::to_html(&graph),
                                    "md" | "markdown" => ua_core::report::to_markdown(&graph),
                                    _ => serde_json::to_string_pretty(&graph).unwrap_or_default(),
                                };
                                Some(json!({"content": [{"type": "text", "text": text}]}))
                            }
                            Err(e) => Some(json!({"content": [{"type": "text", "text": format!("Error: {}", e)}], "isError": true})),
                        }
                    }
                    _ => Some(json!({"content": [{"type": "text", "text": format!("Unknown tool: {}", name)}], "isError": true})),
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

fn build_graph_internal(root: &Path) -> anyhow::Result<ua_core::types::KnowledgeGraph> {
    let scan = ua_core::scanner::scan_project(root)?;
    let registry = ua_core::parser::ParserRegistry::default();
    let mut parsed = Vec::new();
    for file in &scan.files {
        if file.file_category == ua_core::types::FileCategory::Code {
            if let Ok(p) = registry.parse(&root.join(&file.path)) {
                parsed.push(p);
            }
        }
    }
    Ok(ua_core::graph::build_graph(root, &scan, &parsed))
}
