//! LLM Multi-Agent Analysis Adapter
//!
//! This module provides the multi-agent analysis pipeline that mirrors the
//! original Understand-Anything's Claude Code sub-agent pattern (Phase 2).
//!
//! # Architecture
//!
//! The original Understand-Anything pipeline uses Claude Code sub-agents to
//! analyze each file semantically: extracting functions/classes/imports,
//! writing summaries, assigning tags and complexity, and producing structured
//! JSON output matching the KnowledgeGraph types.
//!
//! This module provides:
//!
//! - **Prompt templates** for each agent role (file-analyzer, architecture-analyzer,
//!   tour-builder, graph-reviewer). Each template is a function that produces
//!   an LLM-ready prompt with instructions on what to extract and how to format it.
//!
//! - **Batch computation** (`compute_batches`) — splits the scan result into
//!   LLM-sized batches to control token usage and enable parallel processing.
//!
//! - **`AgentDispatcher` trait** — a trait that the host runtime (e.g., Hermes)
//!   implements to provide actual LLM invocation. This keeps ua-core runtime-agnostic.
//!
//! - **`build_graph_with_llm`** — the full Phase 2 pipeline that scans files,
//!   dispatches file-analyzer agents, collects results, runs the architecture
//!   analyzer, tour builder, and optional graph reviewer.
//!
//! # Pipeline mapping
//!
//! | Original Phase | Our equivalent                         |
//! |----------------|----------------------------------------|
//! | Phase 1: Scan  | `scanner::scan_project()`              |
//! | Phase 2: Analyze| `build_graph_with_llm()` (this module) |
//! | Phase 3: Export | `report::export_report()`              |
//!
//! # Feature gate
//!
//! This entire module is behind the `llm-analysis` feature flag so that
//! projects that only need deterministic parsing do not pull in the
//! prompt-generation and batch-computation logic.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::types::{
    Complexity, Direction, EdgeType, GraphEdge, GraphNode, KnowledgeGraph, Layer, NodeType,
    ProjectMeta, ScanEntry, ScanResult, TourStep,
};

// ── AgentDispatcher trait ────────────────────────────────────────────────────

/// Trait that the host runtime (e.g., Hermes or another agent framework)
/// implements to provide LLM-powered file analysis.
///
/// The trait is deliberately minimal: the host only needs to provide a single
/// `dispatch` method that sends a prompt to a named agent and returns the
/// response text. All prompt construction, result parsing, and orchestration
/// is handled within this module.
///
/// # Example (conceptual)
///
/// ```ignore
/// struct HermesDispatcher;
///
/// impl AgentDispatcher for HermesDispatcher {
///     fn dispatch(&self, agent_name: &str, prompt: &str) -> Result<String> {
///         // Call the Hermes agent runtime with the given prompt
///         hermes_runtime::invoke_agent(agent_name, prompt)
///     }
/// }
/// ```
pub trait AgentDispatcher: Send + Sync {
    /// Dispatch a prompt to an LLM agent and return the response text.
    ///
    /// # Arguments
    ///
    /// * `agent_name` — identifies the agent role (e.g., `"file-analyzer"`,
    ///   `"architecture-analyzer"`). The host decides how to map these names
    ///   to actual LLM instances or system prompts.
    /// * `prompt` — the full prompt string to send to the LLM.
    ///
    /// # Errors
    ///
    /// Returns an error string if the LLM invocation fails (network error,
    /// token limit exceeded, timeout, etc.).
    fn dispatch(&self, agent_name: &str, prompt: &str) -> Result<String>;
}

// A simple alias so we can use `Result<String>` throughout this module.
type Result<T> = std::result::Result<T, String>;

// ── AnalysisConfig ───────────────────────────────────────────────────────────

/// Configuration for the multi-agent analysis pipeline.
///
/// Controls batching, concurrency, and whether LLM analysis is enabled at all.
/// When `use_llm_analysis` is `false`, the pipeline falls back to the
/// deterministic graph builder (`graph::build_graph`).
#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    /// Maximum number of files to include in a single agent batch.
    /// Controls token usage: more files per batch = larger prompts but
    /// fewer round-trips. Default: 5.
    pub max_files_per_batch: usize,

    /// Maximum number of file-analyzer sub-agents to run concurrently.
    /// The host is responsible for respecting this limit; this module
    /// provides batches but does not enforce concurrency internally.
    /// Default: 3.
    pub max_concurrent: usize,

    /// Whether to use LLM-powered semantic analysis.
    /// When `false`, the pipeline should use the deterministic graph builder.
    /// Default: `false`.
    pub use_llm_analysis: bool,

    /// Maximum file size (in lines) to include in LLM analysis.
    /// Files larger than this are skipped with a note. Default: 2000.
    pub max_file_lines: usize,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            max_files_per_batch: 5,
            max_concurrent: 3,
            use_llm_analysis: false,
            max_file_lines: 2000,
        }
    }
}

// ── Prompt Templates ─────────────────────────────────────────────────────────

/// Generate a prompt for the **file-analyzer** agent.
///
/// This agent is the workhorse: it receives a single source file (content,
/// language, path) along with optional project context, and is asked to:
///
/// 1. Extract all functions, classes, structs, traits, modules, and concepts.
/// 2. Identify import and export statements.
/// 3. Write a plain-English summary of the file's purpose.
/// 4. Assign relevant tags and a complexity level.
/// 5. Output the result as a JSON object matching `GraphNode` and `GraphEdge`
///    types (nodes + edges the file contributes to the knowledge graph).
///
/// The JSON schema in the prompt mirrors the actual Rust structs so that
/// the LLM output can be deserialized directly into `GraphNode`/`GraphEdge`.
pub fn file_analyzer_prompt(
    file_path: &str,
    language: &str,
    content: &str,
    project_context: &str,
) -> String {
    format!(
        r##"You are a code analysis agent. Analyze the following source file and produce a structured JSON report.

## Project Context
{project_context}

## File Information
- Path: {file_path}
- Language: {language}

## Source Code
```{language}
{content}
```

## Instructions
1. Extract every function, class, struct, trait, interface, module, and significant concept defined in this file.
2. Identify all import/export statements and what they reference.
3. Write a concise plain-English summary (1-3 sentences) describing what this file does and its role in the project.
4. Assign relevant tags (e.g., language name, framework hints, domain terms).
5. Assign a complexity level: "simple" (< 200 lines, straightforward logic), "moderate" (200-1000 lines or some complexity), "complex" (> 1000 lines or intricate logic).
6. For each extracted entity, determine the node type from this taxonomy:
   - "file" — a source file
   - "function" — a function/method
   - "class" — a class, struct, or interface
   - "module" — a module or namespace
   - "concept" — a domain concept (error types, utility patterns, etc.)
   - "endpoint" — an API endpoint or route
   - "config" — configuration definition
   - "schema" — data schema or type definition

## Output Format
Return ONLY valid JSON with the following structure (no markdown fences, no explanation):

{{
  "file_summary": "string",
  "complexity": "simple" | "moderate" | "complex",
  "tags": ["string"],
  "nodes": [
    {{
      "id": "unique-node-id",
      "type": "function",
      "name": "function_name",
      "file_path": "{file_path}",
      "line_range": [start_line, end_line],
      "summary": "What this entity does",
      "tags": ["tag1"],
      "complexity": "simple"
    }}
  ],
  "edges": [
    {{
      "source": "node-id",
      "target": "node-id",
      "type": "calls",
      "direction": "forward",
      "description": "imports std::collections::HashMap",
      "weight": 0.5
    }}
  ],
  "imports": ["module_name"],
  "exports": ["exported_name"]
}}

## Node type aliases allowed
You may use these aliases for the "type" field:
- "func", "fn", "method" → function
- "class", "struct", "interface" → class
- "mod", "pkg", "package" → module
- "route", "api", "query", "mutation" → endpoint
- "setting", "env", "configuration" → config

## Edge type taxonomy (for edges[].type)
Structural: "imports", "exports", "contains", "inherits", "implements"
Behavioral: "calls", "subscribes", "publishes", "middleware"
Data: "reads_from", "writes_to", "transforms", "validates"
Dependencies: "depends_on", "tested_by", "configures"
Semantic: "related", "similar_to"

## Important
- Use snake_case for all type names.
- Make node IDs unique within this file.
- Only include edges that are clearly evidenced by the code.
- Do NOT wrap the output in ```json fences. Output raw JSON only."##
    )
}

/// Generate a prompt for the **architecture-analyzer** agent.
///
/// This agent receives the complete set of graph nodes and edges and is
/// asked to identify architectural layers, grouping nodes by their logical
/// role in the system (e.g., "Core Logic", "Configuration", "API Layer",
/// "Data Access", "Infrastructure").
///
/// The output is a JSON array of `Layer` objects, each containing an id,
/// name, description, and list of node_ids belonging to that layer.
pub fn architecture_analyzer_prompt(
    nodes_json: &str,
    edges_json: &str,
    project_context: &str,
) -> String {
    format!(
        r##"You are an architecture analysis agent. Given the following knowledge graph, identify the architectural layers and assign each node to the appropriate layer.

## Project Context
{project_context}

## Nodes
```json
{nodes_json}
```

## Edges
```json
{edges_json}
```

## Instructions
1. Analyze the nodes and edges to understand the system's architecture.
2. Group nodes into logical architectural layers. Common patterns include:
   - "core" — Core domain logic and business rules
   - "api" — API endpoints, controllers, routes
   - "data" — Database schemas, models, migrations
   - "config" — Configuration and settings
   - "infra" — Infrastructure, deployment, CI/CD
   - "docs" — Documentation and guides
   - "ui" — User interface components
   - "utils" — Utility functions and helpers
   - "tests" — Test files and fixtures
3. Assign every node to exactly one layer.
4. Provide a descriptive name and description for each layer.

## Output Format
Return ONLY valid JSON (no markdown fences):

[
  {{
    "id": "layer-id",
    "name": "Layer Display Name",
    "description": "What this layer contains and its role",
    "node_ids": ["node-id-1", "node-id-2"]
  }}
]

Output raw JSON only."##
    )
}

/// Generate a prompt for the **tour-builder** agent.
///
/// This agent receives the knowledge graph nodes and architectural layers
/// and is asked to produce a guided tour — an ordered sequence of steps
/// that walks a newcomer through the codebase, starting with high-level
/// structure and progressing to detail.
///
/// Each step includes a title, description, relevant node IDs, and
/// optionally a "language lesson" explaining a relevant programming concept
/// or project convention.
///
/// The output is a JSON array of `TourStep` objects.
pub fn tour_builder_prompt(nodes_json: &str, layers_json: &str, project_context: &str) -> String {
    format!(
        r##"You are a codebase tour guide agent. Create a guided tour that helps a newcomer understand this project step by step.

## Project Context
{project_context}

## Nodes (for reference)
```json
{nodes_json}
```

## Architectural Layers
```json
{layers_json}
```

## Instructions
1. Create 5-10 tour steps that progressively introduce the codebase.
2. Order steps by dependency: start with entry points and overview, then drill into specific layers.
3. Each step should focus on 1-5 related nodes.
4. For each step, optionally include a "language_lesson" that explains a relevant concept (e.g., design pattern, framework convention, language idiom).
5. The tour should tell a story: why the project exists, how it's organized, and how the pieces fit together.

## Output Format
Return ONLY valid JSON (no markdown fences):

[
  {{
    "order": 1,
    "title": "Step Title",
    "description": "What the reader learns at this step",
    "node_ids": ["node-id-1"],
    "language_lesson": "Optional: a teaching note about a pattern or convention"
  }}
]

Output raw JSON only."##
    )
}

/// Generate a prompt for the **graph-reviewer** agent.
///
/// This is an optional quality-assurance step. After the knowledge graph is
/// built, the reviewer validates:
///
/// - All edge sources and targets reference existing node IDs (no dangling edges).
/// - Node types are consistent with their content.
/// - The graph coverage is reasonable (no major files missed).
/// - Architectural layering makes sense.
///
/// The output is a list of issues/warnings and suggested improvements.
pub fn graph_reviewer_prompt(graph_json: &str) -> String {
    format!(
        r##"You are a knowledge graph quality reviewer. Validate the structure and completeness of the following knowledge graph.

## Graph
```json
{graph_json}
```

## Instructions
1. Check that every edge.source and edge.target references an existing node ID in the nodes array. Report any dangling edges.
2. Verify that the layers cover all nodes (every node should appear in exactly one layer's node_ids). Report missing or duplicated nodes.
3. Check for consistency: do node types match their file paths and summaries?
4. Look for gaps: are there major architectural patterns or files that seem missing?
5. Suggest 1-3 concrete improvements.

## Output Format
Return ONLY valid JSON (no markdown fences):

{{
  "valid": true,
  "issues": [
    {{
      "severity": "error" | "warning" | "info",
      "message": "Description of the issue",
      "node_id": "optional affected node",
      "suggestion": "How to fix it"
    }}
  ],
  "dangling_edges": ["edge description"],
  "missing_coverage": ["suggestion"],
  "improvements": ["concrete suggestion"]
}}

Output raw JSON only."##
    )
}

// ── Batch Computation ────────────────────────────────────────────────────────

/// Split the scan results into batches suitable for LLM analysis.
///
/// Files are grouped by directory proximity (same parent directory stays
/// together) and batched up to `config.max_files_per_batch` per batch.
/// Files exceeding `config.max_file_lines` are excluded to avoid token
/// overflow.
///
/// Returns a vector of batches, where each batch is a vector of `ScanEntry`
/// references. The caller is responsible for reading file contents and
/// dispatching to the LLM.
///
/// # Batching strategy
///
/// Files are first sorted by path so that files in the same directory are
/// adjacent. Then they are chunked into batches of at most
/// `max_files_per_batch`. This keeps related files together, which improves
/// the LLM's ability to understand cross-file relationships.
pub fn compute_batches<'a>(
    files: &'a [ScanEntry],
    config: &AnalysisConfig,
) -> Vec<Vec<&'a ScanEntry>> {
    // Filter out files that are too large for LLM analysis
    let mut eligible: Vec<&ScanEntry> = files
        .iter()
        .filter(|e| e.size_lines <= config.max_file_lines)
        .collect();

    // Sort by path so adjacent files stay together in batches
    eligible.sort_by(|a, b| a.path.cmp(&b.path));

    // Chunk into batches
    eligible
        .chunks(config.max_files_per_batch)
        .map(|chunk| chunk.to_vec())
        .collect()
}

// ── LLM-Powered Graph Building ───────────────────────────────────────────────

/// Build a complete knowledge graph using LLM-powered multi-agent analysis.
///
/// This is the main entry point for the Phase 2 pipeline when `use_llm_analysis`
/// is enabled. It orchestrates the full pipeline:
///
/// 1. **Scan files** — reads file contents from disk for all scanned entries.
/// 2. **Compute batches** — groups files into LLM-appropriate batches.
/// 3. **Dispatch file-analyzer agents** — sends each batch to the LLM for
///    semantic analysis, collecting node and edge definitions.
/// 4. **Build graph** — assembles the knowledge graph from LLM output.
/// 5. **Run architecture analyzer** — identifies architectural layers.
/// 6. **Run tour builder** — generates a guided tour.
/// 7. **Run graph reviewer** (optional) — validates the final graph.
///
/// # Arguments
///
/// * `project_root` — root directory of the project (used to resolve file paths).
/// * `scan_result` — the Phase 1 scan result (file inventory).
/// * `dispatcher` — the host's LLM agent dispatcher.
/// * `config` — analysis configuration (batching, concurrency, etc.).
///
/// # Errors
///
/// Returns an error if file reading fails, or if any agent dispatch fails.
///
/// # Note on concurrency
///
/// This implementation processes batches sequentially to keep the logic simple
/// and deterministic. The `max_concurrent` setting in `AnalysisConfig` is
/// documented for the host to use when wrapping this function with parallel
/// dispatch (e.g., using `rayon` or `tokio::task::spawn`).
pub fn build_graph_with_llm(
    project_root: &Path,
    scan_result: &ScanResult,
    dispatcher: &dyn AgentDispatcher,
    config: &AnalysisConfig,
) -> Result<KnowledgeGraph> {
    // ── 0. Guard: fall back to deterministic if LLM is disabled ──────────
    if !config.use_llm_analysis {
        return Err(
            "LLM analysis is disabled (config.use_llm_analysis = false). \
             Use graph::build_graph() for deterministic analysis."
                .to_string(),
        );
    }

    // ── 1. Read file contents ───────────────────────────────────────────
    // Build a lookup: file path → (language, content)
    let mut file_contents: HashMap<String, (String, String)> = HashMap::new();
    let mut skipped: Vec<String> = Vec::new();

    for entry in &scan_result.files {
        if entry.size_lines > config.max_file_lines {
            skipped.push(format!(
                "{} ({} lines, exceeds max {})",
                entry.path, entry.size_lines, config.max_file_lines
            ));
            continue;
        }

        let full_path = project_root.join(&entry.path);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                file_contents.insert(entry.path.clone(), (entry.language.clone(), content));
            }
            Err(e) => {
                skipped.push(format!("{} (read error: {})", entry.path, e));
            }
        }
    }

    if !skipped.is_empty() {
        eprintln!(
            "Skipped {} files during LLM analysis: {:?}",
            skipped.len(),
            skipped
        );
    }

    // ── 2. Compute batches ──────────────────────────────────────────────
    let batches = compute_batches(&scan_result.files, config);
    if batches.is_empty() {
        return Err("No files available for LLM analysis after filtering.".to_string());
    }

    // ── 3. Build project context string ─────────────────────────────────
    let project_context = build_project_context(scan_result);

    // ── 4. Dispatch file-analyzer agents ────────────────────────────────
    let mut all_nodes: Vec<GraphNode> = Vec::new();
    let mut all_edges: Vec<GraphEdge> = Vec::new();
    let mut node_id_counter: u64 = 0;

    for (batch_idx, batch) in batches.iter().enumerate() {
        let mut batch_prompts = String::new();
        batch_prompts.push_str(&format!(
            "Analyze the following batch of files (batch {}/{}). For each file, ",
            batch_idx + 1,
            batches.len()
        ));
        batch_prompts.push_str("produce the structured JSON output as specified.\n\n");

        for entry in batch {
            if let Some((language, content)) = file_contents.get(&entry.path) {
                batch_prompts.push_str(&format!("\n--- FILE: {} ---\n", entry.path));
                let single_prompt =
                    file_analyzer_prompt(&entry.path, language, content, &project_context);
                batch_prompts.push_str(&single_prompt);
                batch_prompts.push_str("\n");
            }
        }

        let response = dispatcher.dispatch("file-analyzer", &batch_prompts)?;

        // Parse the LLM response — try to extract JSON
        match parse_llm_graph_output(&response, &mut node_id_counter) {
            Ok((mut nodes, edges)) => {
                all_nodes.append(&mut nodes);
                all_edges.extend(edges);
            }
            Err(e) => {
                eprintln!(
                    "Warning: failed to parse LLM output for batch {}: {}",
                    batch_idx + 1,
                    e
                );
            }
        }
    }

    // Ensure node IDs are unique (prepend batch info if needed)
    // Also add file-level nodes for any files that didn't get analyzed
    ensure_file_nodes(&scan_result.files, &mut all_nodes, &mut node_id_counter);

    // ── 5. Build project metadata ───────────────────────────────────────
    let project = build_project_meta(project_root, scan_result);

    // ── 6. Assemble initial graph ───────────────────────────────────────
    let mut graph = KnowledgeGraph {
        version: option_env!("CARGO_PKG_VERSION")
            .unwrap_or("0.1.0")
            .to_string(),
        kind: Some("llm-analyzed-codebase".to_string()),
        project,
        nodes: all_nodes,
        edges: all_edges,
        layers: Vec::new(),
        tour: Vec::new(),
    };

    // ── 7. Architecture analysis ────────────────────────────────────────
    let nodes_json = serde_json::to_string(&graph.nodes).map_err(|e| e.to_string())?;
    let edges_json = serde_json::to_string(&graph.edges).map_err(|e| e.to_string())?;

    let arch_prompt = architecture_analyzer_prompt(&nodes_json, &edges_json, &project_context);
    let arch_response = dispatcher.dispatch("architecture-analyzer", &arch_prompt)?;

    if let Ok(layers) = parse_layers_output(&arch_response) {
        graph.layers = layers;
    } else {
        // Fall back to basic layer assignment if LLM output is unparseable
        graph.layers = build_fallback_layers(&graph.nodes);
    }

    // ── 8. Tour generation ──────────────────────────────────────────────
    let layers_json = serde_json::to_string(&graph.layers).map_err(|e| e.to_string())?;

    let tour_prompt = tour_builder_prompt(&nodes_json, &layers_json, &project_context);
    let tour_response = dispatcher.dispatch("tour-builder", &tour_prompt)?;

    if let Ok(tour) = parse_tour_output(&tour_response) {
        graph.tour = tour;
    } else {
        // Fall back to basic tour if LLM output is unparseable
        graph.tour = build_fallback_tour(&graph.nodes);
    }

    // ── 9. Optional graph review ────────────────────────────────────────
    let graph_json = serde_json::to_string(&graph).map_err(|e| e.to_string())?;
    let review_prompt = graph_reviewer_prompt(&graph_json);

    match dispatcher.dispatch("graph-reviewer", &review_prompt) {
        Ok(response) => {
            if let Ok(review) = serde_json::from_str::<serde_json::Value>(&response) {
                if let Some(issues) = review.get("issues").and_then(|i| i.as_array()) {
                    for issue in issues {
                        if let Some(severity) = issue.get("severity").and_then(|s| s.as_str()) {
                            if severity == "error" {
                                if let Some(msg) = issue.get("message").and_then(|m| m.as_str()) {
                                    eprintln!("Graph review ERROR: {}", msg);
                                }
                            }
                        }
                    }
                }
                if let Some(improvements) = review.get("improvements").and_then(|i| i.as_array()) {
                    for imp in improvements {
                        if let Some(s) = imp.as_str() {
                            eprintln!("Graph review improvement: {}", s);
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Graph review skipped: {}", e);
        }
    }

    Ok(graph)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build a project context string from the scan result for inclusion in prompts.
fn build_project_context(scan_result: &ScanResult) -> String {
    let lang_list: Vec<String> = scan_result.stats.by_language.keys().cloned().collect();
    format!(
        "This is a {} project with {} files across {} language(s): {}.",
        match scan_result.estimated_complexity {
            Complexity::Simple => "small/simple",
            Complexity::Moderate => "medium/moderate",
            Complexity::Complex => "large/complex",
        },
        scan_result.total_files,
        lang_list.len(),
        lang_list.join(", ")
    )
}

/// Parse the LLM's JSON response into GraphNode and GraphEdge vectors.
///
/// The LLM may wrap its output in markdown fences or include explanatory text.
/// This function attempts to extract the first valid JSON object/array.
fn parse_llm_graph_output(
    response: &str,
    counter: &mut u64,
) -> Result<(Vec<GraphNode>, Vec<GraphEdge>)> {
    // Try to extract JSON from the response (handle markdown fences)
    let json_str = extract_json(response);

    let value: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("JSON parse error: {}", e))?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // The response could be a single file result or an array of results
    match &value {
        // Single file result: { nodes, edges, ... }
        v if v.is_object() => {
            extract_nodes_and_edges(v, counter, &mut nodes, &mut edges);
        }
        // Array of file results: [ { nodes, edges, ... }, ... ]
        v if v.is_array() => {
            for item in v.as_array().unwrap() {
                extract_nodes_and_edges(item, counter, &mut nodes, &mut edges);
            }
        }
        _ => {
            return Err("Unexpected JSON structure: expected object or array".to_string());
        }
    }

    Ok((nodes, edges))
}

/// Extract nodes and edges from a single LLM output object.
fn extract_nodes_and_edges(
    value: &serde_json::Value,
    counter: &mut u64,
    nodes: &mut Vec<GraphNode>,
    edges: &mut Vec<GraphEdge>,
) {
    if let Some(raw_nodes) = value.get("nodes").and_then(|n| n.as_array()) {
        for raw_node in raw_nodes {
            *counter += 1;
            if let Ok(node) = parse_llm_node(raw_node, *counter) {
                nodes.push(node);
            }
        }
    }

    if let Some(raw_edges) = value.get("edges").and_then(|e| e.as_array()) {
        for raw_edge in raw_edges {
            if let Ok(edge) = parse_llm_edge(raw_edge) {
                edges.push(edge);
            }
        }
    }
}

/// Parse a single node from LLM JSON output, normalizing to our GraphNode type.
fn parse_llm_node(
    value: &serde_json::Value,
    fallback_id: u64,
) -> std::result::Result<GraphNode, String> {
    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("node-{}", fallback_id));

    let raw_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("file");

    let node_type = NodeType::from_alias(raw_type).unwrap_or(NodeType::File);

    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let file_path = value
        .get("file_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let line_range = value.get("line_range").and_then(|v| {
        v.as_array().and_then(|arr| {
            if arr.len() >= 2 {
                Some((
                    arr[0].as_u64().unwrap_or(0) as u32,
                    arr[1].as_u64().unwrap_or(0) as u32,
                ))
            } else {
                None
            }
        })
    });

    let summary = value
        .get("summary")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let tags: Vec<String> = value
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let complexity = value
        .get("complexity")
        .and_then(|v| v.as_str())
        .map(|s| match s.to_lowercase().as_str() {
            "complex" => Complexity::Complex,
            "moderate" => Complexity::Moderate,
            _ => Complexity::Simple,
        })
        .unwrap_or(Complexity::Simple);

    let language_notes = value
        .get("language_notes")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(GraphNode {
        id,
        node_type,
        name,
        file_path,
        line_range,
        summary,
        tags,
        complexity,
        language_notes,
        domain_meta: None,
        knowledge_meta: None,
    })
}

/// Parse a single edge from LLM JSON output.
fn parse_llm_edge(value: &serde_json::Value) -> std::result::Result<GraphEdge, String> {
    let source = value
        .get("source")
        .and_then(|v| v.as_str())
        .ok_or("edge missing source")?
        .to_string();

    let target = value
        .get("target")
        .and_then(|v| v.as_str())
        .ok_or("edge missing target")?
        .to_string();

    let raw_type = value
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("related");

    let edge_type = parse_edge_type(raw_type);

    let direction = value
        .get("direction")
        .and_then(|v| v.as_str())
        .map(|s| match s.to_lowercase().as_str() {
            "backward" | "reverse" => Direction::Backward,
            "bidirectional" | "both" | "bi" => Direction::Bidirectional,
            _ => Direction::Forward,
        })
        .unwrap_or(Direction::Forward);

    let description = value
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let weight = value.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;

    Ok(GraphEdge {
        source,
        target,
        edge_type,
        direction,
        description,
        weight,
    })
}

/// Parse edge type from LLM output, handling aliases.
fn parse_edge_type(raw: &str) -> EdgeType {
    match raw.to_lowercase().as_str() {
        "imports" => EdgeType::Imports,
        "exports" => EdgeType::Exports,
        "contains" => EdgeType::Contains,
        "inherits" | "extends" => EdgeType::Inherits,
        "implements" => EdgeType::Implements,
        "calls" | "invokes" => EdgeType::Calls,
        "subscribes" => EdgeType::Subscribes,
        "publishes" | "emits" => EdgeType::Publishes,
        "middleware" => EdgeType::Middleware,
        "reads_from" | "reads" => EdgeType::ReadsFrom,
        "writes_to" | "writes" => EdgeType::WritesTo,
        "transforms" => EdgeType::Transforms,
        "validates" => EdgeType::Validates,
        "depends_on" | "depends" | "dependency" => EdgeType::DependsOn,
        "tested_by" | "tests" => EdgeType::TestedBy,
        "configures" => EdgeType::Configures,
        "related" | "relates" => EdgeType::Related,
        "similar_to" | "similar" => EdgeType::SimilarTo,
        "deploys" => EdgeType::Deploys,
        "serves" => EdgeType::Serves,
        "provisions" => EdgeType::Provisions,
        "triggers" => EdgeType::Triggers,
        "migrates" => EdgeType::Migrates,
        "documents" | "documented_by" => EdgeType::Documents,
        "routes" => EdgeType::Routes,
        "defines_schema" | "defines" => EdgeType::DefinesSchema,
        "contains_flow" => EdgeType::ContainsFlow,
        "flow_step" => EdgeType::FlowStep,
        "cross_domain" | "crossdomain" => EdgeType::CrossDomain,
        "cites" => EdgeType::Cites,
        "contradicts" => EdgeType::Contradicts,
        "builds_on" | "buildson" => EdgeType::BuildsOn,
        "exemplifies" => EdgeType::Exemplifies,
        "categorized_under" | "categorized" => EdgeType::CategorizedUnder,
        "authored_by" | "authored" => EdgeType::AuthoredBy,
        _ => EdgeType::Related, // fallback
    }
}

/// Ensure every scanned file has at least one node in the graph.
/// Creates a basic `File` node for any file that wasn't covered by LLM output.
fn ensure_file_nodes(files: &[ScanEntry], nodes: &mut Vec<GraphNode>, counter: &mut u64) {
    use std::collections::HashSet;

    // Collect all file_paths already present in nodes
    let covered: HashSet<&str> = nodes
        .iter()
        .filter_map(|n| n.file_path.as_deref())
        .collect();

    for entry in files {
        if covered.contains(entry.path.as_str()) {
            continue;
        }

        *counter += 1;
        let name = entry
            .path
            .rsplit('/')
            .next()
            .unwrap_or(&entry.path)
            .to_string();

        nodes.push(GraphNode {
            id: format!("file-{}", *counter),
            node_type: NodeType::File,
            name,
            file_path: Some(entry.path.clone()),
            line_range: None,
            summary: format!(
                "{} file ({} lines, {})",
                entry.language, entry.size_lines, entry.path
            ),
            tags: vec![entry.language.clone()],
            complexity: match entry.size_lines {
                0..=200 => Complexity::Simple,
                201..=1000 => Complexity::Moderate,
                _ => Complexity::Complex,
            },
            language_notes: None,
            domain_meta: None,
            knowledge_meta: None,
        });
    }
}

/// Build project metadata from the scan result.
fn build_project_meta(root: &Path, scan_result: &ScanResult) -> ProjectMeta {
    let project_name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let languages: Vec<String> = {
        let mut langs: Vec<String> = scan_result.stats.by_language.keys().cloned().collect();
        langs.sort();
        langs
    };

    let description = format!(
        "{} project with {} files across {} languages (LLM-analyzed).",
        project_name,
        scan_result.total_files,
        languages.len()
    );

    // Simple timestamp
    let analyzed_at = {
        use std::time::{SystemTime, UNIX_EPOCH};
        if let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) {
            format!("{}", dur.as_secs())
        } else {
            "unknown".to_string()
        }
    };

    let git_commit_hash = {
        let head_path = root.join(".git").join("HEAD");
        if let Ok(content) = std::fs::read_to_string(&head_path) {
            let content = content.trim();
            if let Some(ref_path) = content.strip_prefix("ref: ") {
                let ref_file = root.join(".git").join(ref_path);
                if let Ok(hash) = std::fs::read_to_string(&ref_file) {
                    hash.trim().to_string()
                } else {
                    content.to_string()
                }
            } else if content.len() >= 7 {
                content.to_string()
            } else {
                "unknown".to_string()
            }
        } else {
            "unknown".to_string()
        }
    };

    ProjectMeta {
        name: project_name,
        languages,
        frameworks: Vec::new(),
        description,
        analyzed_at,
        git_commit_hash,
    }
}

/// Parse the architecture analyzer's JSON response into Layer objects.
fn parse_layers_output(response: &str) -> std::result::Result<Vec<Layer>, String> {
    let json_str = extract_json(response);
    let values: Vec<serde_json::Value> =
        serde_json::from_str(json_str).map_err(|e| format!("Layer parse error: {}", e))?;

    values
        .into_iter()
        .map(|v| {
            Ok(Layer {
                id: v
                    .get("id")
                    .and_then(|s| s.as_str())
                    .ok_or("layer missing id")?
                    .to_string(),
                name: v
                    .get("name")
                    .and_then(|s| s.as_str())
                    .ok_or("layer missing name")?
                    .to_string(),
                description: v
                    .get("description")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
                node_ids: v
                    .get("node_ids")
                    .and_then(|a| a.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|s| s.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// Parse the tour builder's JSON response into TourStep objects.
fn parse_tour_output(response: &str) -> std::result::Result<Vec<TourStep>, String> {
    let json_str = extract_json(response);
    let values: Vec<serde_json::Value> =
        serde_json::from_str(json_str).map_err(|e| format!("Tour parse error: {}", e))?;

    values
        .into_iter()
        .map(|v| {
            Ok(TourStep {
                order: v.get("order").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
                title: v
                    .get("title")
                    .and_then(|s| s.as_str())
                    .unwrap_or("Untitled Step")
                    .to_string(),
                description: v
                    .get("description")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
                node_ids: v
                    .get("node_ids")
                    .and_then(|a| a.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|s| s.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                language_lesson: v
                    .get("language_lesson")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string()),
            })
        })
        .collect()
}

/// Build a simple fallback layer assignment when LLM output is unparseable.
/// Groups nodes by their node_type into basic architectural layers.
fn build_fallback_layers(nodes: &[GraphNode]) -> Vec<Layer> {
    let mut layers: HashMap<&str, (String, String, Vec<String>)> = HashMap::new();

    for node in nodes {
        let (layer_id, layer_name, desc) = match node.node_type {
            NodeType::File | NodeType::Function | NodeType::Class | NodeType::Module => {
                ("code", "Core Code", "Source code and modules")
            }
            NodeType::Concept => ("concepts", "Concepts", "Domain concepts and abstractions"),
            NodeType::Config => ("config", "Configuration", "Configuration files"),
            NodeType::Document => ("docs", "Documentation", "Documentation"),
            NodeType::Service | NodeType::Pipeline | NodeType::Resource => {
                ("infra", "Infrastructure", "Infrastructure and deployment")
            }
            NodeType::Table | NodeType::Schema => {
                ("data", "Data & Schema", "Data models and schemas")
            }
            NodeType::Endpoint => ("api", "API Layer", "API endpoints"),
            NodeType::Domain | NodeType::Flow | NodeType::Step => {
                ("domain", "Domain Logic", "Domain models and flows")
            }
            NodeType::Article
            | NodeType::Entity
            | NodeType::Topic
            | NodeType::Claim
            | NodeType::Source => ("knowledge", "Knowledge", "Knowledge artifacts"),
        };
        layers
            .entry(layer_id)
            .or_insert_with(|| (layer_name.to_string(), desc.to_string(), Vec::new()))
            .2
            .push(node.id.clone());
    }

    let mut result: Vec<Layer> = layers
        .into_iter()
        .map(|(id, (name, description, node_ids))| Layer {
            id: id.to_string(),
            name,
            description,
            node_ids,
        })
        .collect();
    result.sort_by(|a, b| a.id.cmp(&b.id));
    result
}

/// Build a simple fallback tour when LLM output is unparseable.
fn build_fallback_tour(nodes: &[GraphNode]) -> Vec<TourStep> {
    let file_nodes: Vec<&GraphNode> = nodes.iter().filter(|n| n.file_path.is_some()).collect();

    vec![
        TourStep {
            order: 1,
            title: "Project Overview".to_string(),
            description: format!(
                "This project contains {} files. Start here to understand the high-level structure.",
                file_nodes.len()
            ),
            node_ids: file_nodes.iter().take(5).map(|n| n.id.clone()).collect(),
            language_lesson: Some(
                "A knowledge graph represents code as interconnected nodes with edges showing relationships."
                    .to_string(),
            ),
        },
        TourStep {
            order: 2,
            title: "Key Files".to_string(),
            description: "Explore the main source files to understand the project structure."
                .to_string(),
            node_ids: file_nodes.iter().map(|n| n.id.clone()).collect(),
            language_lesson: Some(
                "LLM-powered analysis extracts semantic meaning beyond what static parsers can capture."
                    .to_string(),
            ),
        },
    ]
}

/// Extract clean JSON from an LLM response that may include markdown fences,
/// explanatory text, or other noise.
///
/// Strategy:
/// 1. If the response starts with `{` or `[`, use it as-is.
/// 2. Look for ```json ... ``` fences and extract content between them.
/// 3. Look for ``` ... ``` fences (no language tag) and extract.
/// 4. As a last resort, find the first `{` or `[` and take from there.
fn extract_json(response: &str) -> &str {
    let trimmed = response.trim();

    // Case 1: Clean JSON
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return trimmed;
    }

    // Case 2: ```json ... ``` fence
    if let Some(start) = trimmed.find("```json") {
        let after_fence = &trimmed[start + 7..];
        if let Some(end) = after_fence.find("```") {
            let inner = after_fence[..end].trim();
            if inner.starts_with('{') || inner.starts_with('[') {
                return inner;
            }
        }
    }

    // Case 3: ``` ... ``` fence (no language tag)
    if let Some(start) = trimmed.find("```") {
        let after_fence = &trimmed[start + 3..];
        if let Some(end) = after_fence.find("```") {
            let inner = after_fence[..end].trim();
            if inner.starts_with('{') || inner.starts_with('[') {
                return inner;
            }
        }
    }

    // Case 4: Find first { or [ and take everything from there
    if let Some(pos) = trimmed.find(|c| c == '{' || c == '[') {
        return &trimmed[pos..];
    }

    // Fallback: return as-is
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_batches_small() {
        let files = vec![
            ScanEntry {
                path: "src/a.rs".into(),
                language: "rust".into(),
                size_lines: 100,
                file_category: crate::types::FileCategory::Code,
            },
            ScanEntry {
                path: "src/b.rs".into(),
                language: "rust".into(),
                size_lines: 200,
                file_category: crate::types::FileCategory::Code,
            },
        ];

        let config = AnalysisConfig {
            max_files_per_batch: 2,
            ..Default::default()
        };

        let batches = compute_batches(&files, &config);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 2);
    }

    #[test]
    fn test_compute_batches_splits() {
        let files: Vec<ScanEntry> = (0..10)
            .map(|i| ScanEntry {
                path: format!("src/file_{}.rs", i),
                language: "rust".into(),
                size_lines: 100,
                file_category: crate::types::FileCategory::Code,
            })
            .collect();

        let config = AnalysisConfig {
            max_files_per_batch: 3,
            ..Default::default()
        };

        let batches = compute_batches(&files, &config);
        assert_eq!(batches.len(), 4); // 10 / 3 = 4 batches (3+3+3+1)
        assert_eq!(batches[3].len(), 1);
    }

    #[test]
    fn test_compute_batches_skips_large() {
        let files = vec![
            ScanEntry {
                path: "small.rs".into(),
                language: "rust".into(),
                size_lines: 100,
                file_category: crate::types::FileCategory::Code,
            },
            ScanEntry {
                path: "huge.rs".into(),
                language: "rust".into(),
                size_lines: 5000,
                file_category: crate::types::FileCategory::Code,
            },
        ];

        let config = AnalysisConfig {
            max_files_per_batch: 5,
            max_file_lines: 2000,
            ..Default::default()
        };

        let batches = compute_batches(&files, &config);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 1);
        assert_eq!(batches[0][0].path, "small.rs");
    }

    #[test]
    fn test_extract_json_clean() {
        assert_eq!(extract_json(r#"{"key": "value"}"#), r#"{"key": "value"}"#);
        assert_eq!(extract_json(r#"[1, 2, 3]"#), r#"[1, 2, 3]"#);
    }

    #[test]
    fn test_extract_json_fenced() {
        let input = "```json\n{\"key\": \"value\"}\n```";
        assert_eq!(extract_json(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_extract_json_with_explanation() {
        let input = "Here is the analysis:\n\n```json\n{\"nodes\": []}\n```\n\nThat's all.";
        assert_eq!(extract_json(input), r#"{"nodes": []}"#);
    }

    #[test]
    fn test_extract_json_no_fence_with_text() {
        let input = "Some text before {\"key\": \"value\"} and after";
        assert_eq!(extract_json(input), r#"{\"key\": \"value\"} and after"#);
    }

    #[test]
    fn test_parse_edge_type_aliases() {
        assert_eq!(parse_edge_type("calls"), EdgeType::Calls);
        assert_eq!(parse_edge_type("invokes"), EdgeType::Calls);
        assert_eq!(parse_edge_type("extends"), EdgeType::Inherits);
        assert_eq!(parse_edge_type("reads"), EdgeType::ReadsFrom);
        assert_eq!(parse_edge_type("depends"), EdgeType::DependsOn);
        assert_eq!(parse_edge_type("unknown_edge"), EdgeType::Related);
    }

    #[test]
    fn test_llm_node_parse() {
        let json = serde_json::json!({
            "id": "func-1",
            "type": "fn",
            "name": "calculate",
            "file_path": "src/math.rs",
            "line_range": [10, 25],
            "summary": "Calculates the result",
            "tags": ["math", "core"],
            "complexity": "moderate"
        });

        let node = parse_llm_node(&json, 0).unwrap();
        assert_eq!(node.id, "func-1");
        assert_eq!(node.node_type, NodeType::Function);
        assert_eq!(node.name, "calculate");
        assert_eq!(node.summary, "Calculates the result");
        assert_eq!(node.complexity, Complexity::Moderate);
    }

    #[test]
    fn test_file_analyzer_prompt_structure() {
        let prompt = file_analyzer_prompt(
            "src/main.rs",
            "rust",
            "fn main() {}",
            "A small Rust project",
        );
        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("fn main() {}"));
        assert!(prompt.contains("A small Rust project"));
        assert!(prompt.contains("rust"));
        assert!(prompt.contains("Output Format"));
        // Should not have markdown fences in the JSON instructions
        assert!(prompt.contains("file_summary"));
        assert!(prompt.contains("nodes"));
        assert!(prompt.contains("edges"));
    }

    #[test]
    fn test_fallback_layers() {
        let nodes = vec![
            GraphNode {
                id: "1".into(),
                node_type: NodeType::File,
                name: "main.rs".into(),
                file_path: Some("src/main.rs".into()),
                line_range: None,
                summary: "entry point".into(),
                tags: vec![],
                complexity: Complexity::Simple,
                language_notes: None,
                domain_meta: None,
                knowledge_meta: None,
            },
            GraphNode {
                id: "2".into(),
                node_type: NodeType::Config,
                name: "Cargo.toml".into(),
                file_path: Some("Cargo.toml".into()),
                line_range: None,
                summary: "config".into(),
                tags: vec![],
                complexity: Complexity::Simple,
                language_notes: None,
                domain_meta: None,
                knowledge_meta: None,
            },
        ];

        let layers = build_fallback_layers(&nodes);
        assert!(layers.len() >= 2);
        let code_layer = layers.iter().find(|l| l.id == "code").unwrap();
        assert!(code_layer.node_ids.contains(&"1".to_string()));
        let config_layer = layers.iter().find(|l| l.id == "config").unwrap();
        assert!(config_layer.node_ids.contains(&"2".to_string()));
    }

    #[test]
    fn test_fallback_tour() {
        let nodes = vec![GraphNode {
            id: "1".into(),
            node_type: NodeType::File,
            name: "main.rs".into(),
            file_path: Some("src/main.rs".into()),
            line_range: None,
            summary: "".into(),
            tags: vec![],
            complexity: Complexity::Simple,
            language_notes: None,
            domain_meta: None,
            knowledge_meta: None,
        }];

        let tour = build_fallback_tour(&nodes);
        assert_eq!(tour.len(), 2);
        assert_eq!(tour[0].order, 1);
        assert!(tour[0].language_lesson.is_some());
    }
}
