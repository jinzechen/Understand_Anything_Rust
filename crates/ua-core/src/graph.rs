//! Knowledge graph builder — converts scan + parse results into KnowledgeGraph.
//!
//! This module transforms raw file scanning and parsing results into a structured
//! knowledge graph with nodes, edges, layers, and a guided tour.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::parser::ParsedFile;
use crate::types::{
    Complexity, Direction, EdgeType, FileCategory, GraphEdge, GraphNode, KnowledgeGraph, Layer,
    NodeType, ProjectMeta, ScanEntry, ScanResult, TourStep,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Generate a deterministic node ID from a path string using SHA-256.
fn node_id_from_path(path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    let result = hasher.finalize();
    // Take first 8 bytes → 16 hex chars
    result
        .iter()
        .take(8)
        .map(|b| format!("{:02x}", b))
        .collect()
}

/// Generate a display name from a file path (last component without extension).
fn display_name_from_path(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    // Strip extension for nicer display
    if let Some(dot) = name.rfind('.') {
        name[..dot].to_string()
    } else {
        name.to_string()
    }
}

/// Get current time as ISO 8601 string (simple UTC format).
fn now_iso() -> String {
    if let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) {
        let secs = dur.as_secs();
        // Simple ISO 8601: YYYY-MM-DDTHH:MM:SSZ
        let days = secs / 86400;
        let time_secs = secs % 86400;
        let hours = time_secs / 3600;
        let mins = (time_secs % 3600) / 60;
        let secs_rem = time_secs % 60;

        // Convert days since epoch to date (simplified, good enough)
        // Using a simple algorithm: 1970-01-01 + days
        let mut y = 1970i64;
        let mut d = days as i64;
        loop {
            let days_in_year = if is_leap(y) { 366 } else { 365 };
            if d < days_in_year {
                break;
            }
            d -= days_in_year;
            y += 1;
        }
        let (month, day) = month_day_from_year_day(y, d as u32);

        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            y, month, day, hours, mins, secs_rem
        )
    } else {
        "unknown".to_string()
    }
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

fn month_day_from_year_day(year: i64, day_of_year: u32) -> (u32, u32) {
    let days_in_month = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut remaining = day_of_year;
    for (i, &dim) in days_in_month.iter().enumerate() {
        if remaining < dim {
            return (i as u32 + 1, remaining + 1);
        }
        remaining -= dim;
    }
    (12, 31)
}

/// Try to get the current git commit hash from the project root.
fn get_git_hash(root: &Path) -> String {
    // Try reading .git/HEAD first
    let head_path = root.join(".git").join("HEAD");
    if let Ok(content) = std::fs::read_to_string(&head_path) {
        let content = content.trim();
        // If it's a ref (starts with "ref:"), follow it
        if let Some(ref_path) = content.strip_prefix("ref: ") {
            let ref_file = root.join(".git").join(ref_path);
            if let Ok(hash) = std::fs::read_to_string(&ref_file) {
                let hash = hash.trim().to_string();
                if !hash.is_empty() {
                    return hash;
                }
            }
        } else if content.len() >= 7 {
            return content.to_string();
        }
    }
    "unknown".to_string()
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Build a complete knowledge graph from scan and parse results.
///
/// This is the main entry point — it orchestrates node creation, edge building,
/// layer grouping, and tour generation into a single KnowledgeGraph.
pub fn build_graph(
    project_root: &Path,
    scan_result: &ScanResult,
    parsed_files: &[ParsedFile],
) -> KnowledgeGraph {
    // ── Project metadata ───────────────────────────────────────────────
    let project_name = project_root
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
        "{} project with {} files across {} languages. Complexity: {:?}.",
        project_name,
        scan_result.total_files,
        languages.len(),
        scan_result.estimated_complexity
    );

    let analyzed_at = now_iso();
    let git_commit_hash = get_git_hash(project_root);

    let project = ProjectMeta {
        name: project_name,
        languages,
        frameworks: Vec::new(),
        description,
        analyzed_at,
        git_commit_hash,
    };

    // ── File nodes ─────────────────────────────────────────────────────
    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut path_to_id: HashMap<String, String> = HashMap::new();

    for entry in &scan_result.files {
        let id = node_id_from_path(&entry.path);
        path_to_id.insert(entry.path.clone(), id.clone());

        let node_type = file_category_to_node_type(&entry.file_category);
        let name = display_name_from_path(&entry.path);

        // Collect summary from parsed file if available
        let summary = parsed_files
            .iter()
            .find(|pf| pf.path == entry.path)
            .map(|pf| {
                let def_count = pf.definitions.len();
                let import_count = pf.imports.len();
                if def_count > 0 || import_count > 0 {
                    format!(
                        "{} ({} lines, {} definitions, {} imports)",
                        entry.language, entry.size_lines, def_count, import_count
                    )
                } else {
                    format!("{} ({} lines)", entry.language, entry.size_lines)
                }
            })
            .unwrap_or_else(|| format!("{} ({} lines)", entry.language, entry.size_lines));

        let complexity = match entry.size_lines {
            0..=200 => Complexity::Simple,
            201..=1000 => Complexity::Moderate,
            _ => Complexity::Complex,
        };

        nodes.push(GraphNode {
            id,
            node_type,
            name,
            file_path: Some(entry.path.clone()),
            line_range: None,
            summary,
            tags: vec![entry.language.clone()],
            complexity,
            language_notes: None,
            domain_meta: None,
            knowledge_meta: None,
        });
    }

    // ── Directory nodes ────────────────────────────────────────────────
    let mut seen_dirs: HashSet<String> = HashSet::new();
    for entry in &scan_result.files {
        let parts: Vec<&str> = entry.path.split('/').collect();
        // For paths like "src/parser/mod.rs", create: "src", "src/parser"
        for i in 0..parts.len() {
            let dir_path = parts[..i].join("/");
            if dir_path.is_empty() {
                continue;
            }
            if seen_dirs.insert(dir_path.clone()) {
                let id = node_id_from_path(&dir_path);
                let name = parts[i - 1].to_string();
                path_to_id.insert(dir_path.clone(), id.clone());
                nodes.push(GraphNode {
                    id,
                    node_type: NodeType::Module,
                    name,
                    file_path: None,
                    line_range: None,
                    summary: format!("Directory: {}", dir_path),
                    tags: vec!["directory".to_string()],
                    complexity: Complexity::Simple,
                    language_notes: None,
                    domain_meta: None,
                    knowledge_meta: None,
                });
            }
        }
    }

    // ── Edges ──────────────────────────────────────────────────────────
    let mut dir_edges = build_directory_edges(scan_result);
    let import_edges = build_import_edges(parsed_files);
    let mut edges = Vec::new();
    edges.append(&mut dir_edges);
    edges.extend(import_edges);

    // ── Layers ─────────────────────────────────────────────────────────
    let layers = build_layers(&nodes);

    // ── Tour ───────────────────────────────────────────────────────────
    let tour = build_tour(&nodes);

    KnowledgeGraph {
        version: option_env!("CARGO_PKG_VERSION")
            .unwrap_or("0.1.0")
            .to_string(),
        kind: Some("codebase".to_string()),
        project,
        nodes,
        edges,
        layers,
        tour,
    }
}

// ── File Category → Node Type Mapping ────────────────────────────────────────

/// Map a scanner file category to the most appropriate knowledge graph node type.
pub fn file_category_to_node_type(category: &FileCategory) -> NodeType {
    match category {
        FileCategory::Code => NodeType::File,
        FileCategory::Config => NodeType::Config,
        FileCategory::Docs => NodeType::Document,
        FileCategory::Infra => NodeType::Resource,
        FileCategory::Script => NodeType::File,
        FileCategory::Data => NodeType::Schema,
        FileCategory::Test => NodeType::File,
        FileCategory::Unknown => NodeType::File,
    }
}

// ── Directory Edges ──────────────────────────────────────────────────────────

/// Build parent→child "contains" edges from the file tree.
///
/// Creates a chain of `Contains` edges for each file path (e.g.,
/// `"src" → "src/parser" → "src/parser/mod.rs"`). Directory nodes
/// are deduplicated via the deterministic SHA-256 path-based IDs.
pub fn build_directory_edges(scan_result: &ScanResult) -> Vec<GraphEdge> {
    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();

    for entry in &scan_result.files {
        let parts: Vec<&str> = entry.path.split('/').collect();

        for i in 0..parts.len() {
            let parent_path = parts[..i].join("/");
            let child_path = parts[..=i].join("/");

            if parent_path.is_empty() || child_path.is_empty() {
                continue;
            }

            let source = node_id_from_path(&parent_path);
            let target = node_id_from_path(&child_path);

            // Deduplicate edges
            let key = (source.clone(), target.clone());
            if seen.insert(key) {
                edges.push(GraphEdge {
                    source,
                    target,
                    edge_type: EdgeType::Contains,
                    direction: Direction::Forward,
                    description: Some(format!("{} contains {}", parent_path, child_path)),
                    weight: 0.3,
                });
            }
        }
    }

    edges
}

// ── Import Edges ─────────────────────────────────────────────────────────────

/// Build "imports" edges between files based on parsed import data.
///
/// For each file with imports, tries to resolve the imported module name
/// to a known file path (by matching the last path segment), creating
/// `Imports` edges from the importing file to the imported file.
pub fn build_import_edges(parsed_files: &[ParsedFile]) -> Vec<GraphEdge> {
    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();

    // Build a lookup: last path component → file path
    // E.g., "mod" → "src/parser/mod.rs"
    let mut name_to_path: HashMap<String, &str> = HashMap::new();
    for pf in parsed_files {
        let last = pf.path.rsplit('/').next().unwrap_or(&pf.path);
        let stem = if let Some(dot) = last.rfind('.') {
            &last[..dot]
        } else {
            last
        };
        name_to_path.entry(stem.to_string()).or_insert(&pf.path);

        // Also index by full path for exact matches
        name_to_path.entry(pf.path.clone()).or_insert(&pf.path);
    }

    for pf in parsed_files {
        let source_id = node_id_from_path(&pf.path);

        for import in &pf.imports {
            // Try to resolve the import source to a known file
            // The import source might be like "std::collections::HashMap" or "crate::types"
            // Try to match by the target module name
            let import_name = import.source.split("::").last().unwrap_or(&import.source);

            // Try matching: first by exact path, then by stem
            let target_path = name_to_path.get(import_name).or_else(|| {
                // Try as a relative module path (e.g., "crate::types" → "types")
                name_to_path.get(&import.source)
            });

            if let Some(&target_path) = target_path {
                if target_path != pf.path {
                    let target_id = node_id_from_path(target_path);
                    let key = (source_id.clone(), target_id.clone());
                    if seen.insert(key) {
                        edges.push(GraphEdge {
                            source: source_id.clone(),
                            target: target_id,
                            edge_type: EdgeType::Imports,
                            direction: Direction::Forward,
                            description: Some(format!(
                                "{} imports {}",
                                display_name_from_path(&pf.path),
                                import_name
                            )),
                            weight: 0.5,
                        });
                    }
                }
            }
        }
    }

    edges
}

// ── Layers ───────────────────────────────────────────────────────────────────

/// Group nodes into architectural layers based on file category.
///
/// Returns a set of `Layer` definitions that partition the graph nodes
/// by their logical role (core logic, configuration, documentation, etc.).
pub fn build_layers(nodes: &[GraphNode]) -> Vec<Layer> {
    let mut layer_map: HashMap<String, (String, Vec<String>)> = HashMap::new();

    for node in nodes {
        let (layer_id, layer_name, _description) = match node.node_type {
            NodeType::File | NodeType::Function | NodeType::Class | NodeType::Module => {
                ("code", "Core Code", "Source code files and modules")
            }
            NodeType::Config => (
                "config",
                "Configuration",
                "Configuration and settings files",
            ),
            NodeType::Document => ("docs", "Documentation", "Documentation and markdown files"),
            NodeType::Resource | NodeType::Service | NodeType::Pipeline => (
                "infra",
                "Infrastructure",
                "Infrastructure and deployment files",
            ),
            NodeType::Schema | NodeType::Table => (
                "data",
                "Data & Schema",
                "Data models and schema definitions",
            ),
            NodeType::Endpoint => ("api", "API Layer", "API endpoints and routes"),
            _ => ("other", "Other", "Miscellaneous nodes"),
        };

        let entry = layer_map
            .entry(layer_id.to_string())
            .or_insert_with(|| (layer_name.to_string(), Vec::new()));
        entry.1.push(node.id.clone());
    }

    let mut layers: Vec<Layer> = layer_map
        .into_iter()
        .map(|(id, (name, node_ids))| {
            let description = match id.as_str() {
                "code" => "Source code files and modules".to_string(),
                "config" => "Configuration and settings files".to_string(),
                "docs" => "Documentation and markdown files".to_string(),
                "infra" => "Infrastructure and deployment files".to_string(),
                "data" => "Data models and schema definitions".to_string(),
                "api" => "API endpoints and routes".to_string(),
                "scripts" => "Build and automation scripts".to_string(),
                _ => "Miscellaneous nodes".to_string(),
            };
            Layer {
                id,
                name,
                description,
                node_ids,
            }
        })
        .collect();

    // Sort by ID for deterministic output
    layers.sort_by(|a, b| a.id.cmp(&b.id));
    layers
}

// ── Guided Tour ──────────────────────────────────────────────────────────────

/// Generate a simple guided tour to help users explore the knowledge graph.
///
/// Creates sequential `TourStep` entries that walk through the project
/// at a high level: overview, directory structure, key files, and layers.
pub fn build_tour(nodes: &[GraphNode]) -> Vec<TourStep> {
    let mut tour = Vec::new();

    // Step 1: Project overview
    let file_nodes: Vec<&GraphNode> = nodes.iter().filter(|n| n.file_path.is_some()).collect();
    let dir_nodes: Vec<&GraphNode> = nodes
        .iter()
        .filter(|n| n.file_path.is_none() && n.node_type == NodeType::Module)
        .collect();

    tour.push(TourStep {
        order: 1,
        title: "Project Overview".to_string(),
        description: format!(
            "This project contains {} files organized across {} directories. Start here to understand the high-level structure.",
            file_nodes.len(),
            dir_nodes.len()
        ),
        node_ids: file_nodes.iter().take(5).map(|n| n.id.clone()).collect(),
        language_lesson: Some("A knowledge graph represents code as interconnected nodes — files, functions, classes — with edges showing how they relate.".to_string()),
    });

    // Step 2: Directory structure (top-level directories only)
    let top_dirs: Vec<&GraphNode> = dir_nodes
        .iter()
        .filter(|n| !n.name.contains('/'))
        .copied()
        .collect();

    if !top_dirs.is_empty() {
        tour.push(TourStep {
            order: 2,
            title: "Directory Structure".to_string(),
            description: format!(
                "The project has {} top-level directories. Each contains related files grouped by purpose.",
                top_dirs.len()
            ),
            node_ids: top_dirs.iter().map(|n| n.id.clone()).collect(),
            language_lesson: Some(
                "Contains edges (parent→child) show the file tree hierarchy in graph form."
                    .to_string(),
            ),
        });
    }

    // Step 3: Key files by category
    let categories = [
        (NodeType::File, "Source Code"),
        (NodeType::Config, "Configuration"),
        (NodeType::Document, "Documentation"),
        (NodeType::Resource, "Infrastructure"),
        (NodeType::Schema, "Data & Schemas"),
    ];

    let mut order = 3u32;
    for (node_type, label) in &categories {
        let matching: Vec<&GraphNode> = nodes
            .iter()
            .filter(|n| n.node_type == *node_type && n.file_path.is_some())
            .collect();
        if !matching.is_empty() {
            tour.push(TourStep {
                order,
                title: format!("{} Files", label),
                description: format!(
                    "{} {} files found. These define the project's {}.",
                    matching.len(),
                    label.to_lowercase(),
                    label.to_lowercase()
                ),
                node_ids: matching.iter().take(10).map(|n| n.id.clone()).collect(),
                language_lesson: None,
            });
            order += 1;
        }
    }

    // Step N: Connection summary
    tour.push(TourStep {
        order,
        title: "Explore Connections".to_string(),
        description:
            "Use the graph to trace imports, dependencies, and directory structure. Each edge type reveals a different relationship.".to_string(),
        node_ids: Vec::new(),
        language_lesson: Some(
            "Imports edges show module dependencies. Contains edges show directory hierarchy. Together they reveal the project's architecture.".to_string(),
        ),
    });

    tour
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ImportInfo, ScanStats};

    fn make_scan_result() -> ScanResult {
        ScanResult {
            files: vec![
                ScanEntry {
                    path: "src/main.rs".to_string(),
                    language: "rust".to_string(),
                    size_lines: 150,
                    file_category: FileCategory::Code,
                },
                ScanEntry {
                    path: "src/lib.rs".to_string(),
                    language: "rust".to_string(),
                    size_lines: 50,
                    file_category: FileCategory::Code,
                },
                ScanEntry {
                    path: "src/types.rs".to_string(),
                    language: "rust".to_string(),
                    size_lines: 300,
                    file_category: FileCategory::Code,
                },
                ScanEntry {
                    path: "Cargo.toml".to_string(),
                    language: "toml".to_string(),
                    size_lines: 20,
                    file_category: FileCategory::Config,
                },
                ScanEntry {
                    path: "README.md".to_string(),
                    language: "markdown".to_string(),
                    size_lines: 40,
                    file_category: FileCategory::Docs,
                },
                ScanEntry {
                    path: "deploy.sh".to_string(),
                    language: "shell".to_string(),
                    size_lines: 25,
                    file_category: FileCategory::Script,
                },
            ],
            total_files: 6,
            filtered_by_ignore: 0,
            estimated_complexity: Complexity::Simple,
            stats: ScanStats {
                files_scanned: 6,
                by_category: {
                    let mut m = HashMap::new();
                    m.insert("code".to_string(), 3);
                    m.insert("config".to_string(), 1);
                    m.insert("docs".to_string(), 1);
                    m.insert("script".to_string(), 1);
                    m
                },
                by_language: {
                    let mut m = HashMap::new();
                    m.insert("rust".to_string(), 3);
                    m.insert("toml".to_string(), 1);
                    m.insert("markdown".to_string(), 1);
                    m.insert("shell".to_string(), 1);
                    m
                },
            },
        }
    }

    fn make_parsed_files() -> Vec<ParsedFile> {
        vec![
            ParsedFile {
                path: "src/main.rs".to_string(),
                language: "rust".to_string(),
                line_count: 150,
                definitions: vec![],
                imports: vec![ImportInfo {
                    name: "lib".to_string(),
                    source: "crate::lib".to_string(),
                    line_range: (1, 1),
                }],
                sections: vec![],
                services: vec![],
                endpoints: vec![],
                steps: vec![],
            },
            ParsedFile {
                path: "src/lib.rs".to_string(),
                language: "rust".to_string(),
                line_count: 50,
                definitions: vec![],
                imports: vec![ImportInfo {
                    name: "types".to_string(),
                    source: "crate::types".to_string(),
                    line_range: (1, 1),
                }],
                sections: vec![],
                services: vec![],
                endpoints: vec![],
                steps: vec![],
            },
            ParsedFile {
                path: "src/types.rs".to_string(),
                language: "rust".to_string(),
                line_count: 300,
                definitions: vec![],
                imports: vec![],
                sections: vec![],
                services: vec![],
                endpoints: vec![],
                steps: vec![],
            },
            ParsedFile {
                path: "Cargo.toml".to_string(),
                language: "toml".to_string(),
                line_count: 20,
                definitions: vec![],
                imports: vec![],
                sections: vec![],
                services: vec![],
                endpoints: vec![],
                steps: vec![],
            },
            ParsedFile {
                path: "README.md".to_string(),
                language: "markdown".to_string(),
                line_count: 40,
                definitions: vec![],
                imports: vec![],
                sections: vec![],
                services: vec![],
                endpoints: vec![],
                steps: vec![],
            },
            ParsedFile {
                path: "deploy.sh".to_string(),
                language: "shell".to_string(),
                line_count: 25,
                definitions: vec![],
                imports: vec![],
                sections: vec![],
                services: vec![],
                endpoints: vec![],
                steps: vec![],
            },
        ]
    }

    #[test]
    fn test_file_category_to_node_type() {
        assert_eq!(
            file_category_to_node_type(&FileCategory::Code),
            NodeType::File
        );
        assert_eq!(
            file_category_to_node_type(&FileCategory::Config),
            NodeType::Config
        );
        assert_eq!(
            file_category_to_node_type(&FileCategory::Docs),
            NodeType::Document
        );
        assert_eq!(
            file_category_to_node_type(&FileCategory::Infra),
            NodeType::Resource
        );
        assert_eq!(
            file_category_to_node_type(&FileCategory::Script),
            NodeType::File
        );
        assert_eq!(
            file_category_to_node_type(&FileCategory::Data),
            NodeType::Schema
        );
        assert_eq!(
            file_category_to_node_type(&FileCategory::Test),
            NodeType::File
        );
        assert_eq!(
            file_category_to_node_type(&FileCategory::Unknown),
            NodeType::File
        );
    }

    #[test]
    fn test_node_id_deterministic() {
        let id1 = node_id_from_path("src/main.rs");
        let id2 = node_id_from_path("src/main.rs");
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 16);

        let id_different = node_id_from_path("src/lib.rs");
        assert_ne!(id1, id_different);
    }

    #[test]
    fn test_display_name_from_path() {
        assert_eq!(display_name_from_path("src/main.rs"), "main");
        assert_eq!(display_name_from_path("README.md"), "README");
        assert_eq!(display_name_from_path("Cargo.toml"), "Cargo");
        assert_eq!(display_name_from_path("deploy.sh"), "deploy");
        assert_eq!(display_name_from_path("src/parser/mod.rs"), "mod");
    }

    #[test]
    fn test_build_directory_edges() {
        let scan = make_scan_result();
        let edges = build_directory_edges(&scan);

        // Should have edges for all directory paths
        // "src" → "src/main.rs", "src" → "src/lib.rs", "src" → "src/types.rs"
        assert!(!edges.is_empty());

        // All edges should be Contains type
        for edge in &edges {
            assert_eq!(edge.edge_type, EdgeType::Contains);
        }

        // No duplicate edges
        let mut keys = HashSet::new();
        for edge in &edges {
            let key = (edge.source.clone(), edge.target.clone());
            assert!(keys.insert(key.clone()), "Duplicate edge: {:?}", key);
        }
    }

    #[test]
    fn test_build_import_edges() {
        let parsed = make_parsed_files();
        let edges = build_import_edges(&parsed);

        // main.rs imports lib, lib.rs imports types
        assert!(edges.len() >= 2);

        for edge in &edges {
            assert_eq!(edge.edge_type, EdgeType::Imports);
        }

        // Check that main.rs → lib.rs exists
        let main_id = node_id_from_path("src/main.rs");
        let lib_id = node_id_from_path("src/lib.rs");
        let has_main_to_lib = edges
            .iter()
            .any(|e| e.source == main_id && e.target == lib_id);
        assert!(has_main_to_lib, "Missing main.rs → lib.rs import edge");

        // Check that lib.rs → types.rs exists
        let types_id = node_id_from_path("src/types.rs");
        let has_lib_to_types = edges
            .iter()
            .any(|e| e.source == lib_id && e.target == types_id);
        assert!(has_lib_to_types, "Missing lib.rs → types.rs import edge");
    }

    #[test]
    fn test_build_layers() {
        let scan = make_scan_result();
        let mut nodes = Vec::new();

        for entry in &scan.files {
            nodes.push(GraphNode {
                id: node_id_from_path(&entry.path),
                node_type: file_category_to_node_type(&entry.file_category),
                name: display_name_from_path(&entry.path),
                file_path: Some(entry.path.clone()),
                line_range: None,
                summary: String::new(),
                tags: vec![],
                complexity: Complexity::Simple,
                language_notes: None,
                domain_meta: None,
                knowledge_meta: None,
            });
        }

        let layers = build_layers(&nodes);
        assert!(!layers.is_empty());

        // Should have layers for code, config, docs, scripts
        let layer_ids: Vec<&str> = layers.iter().map(|l| l.id.as_str()).collect();
        assert!(layer_ids.contains(&"code"));
        assert!(layer_ids.contains(&"config"));
        assert!(layer_ids.contains(&"docs"));
        // Note: Script→File mapping means no "other" layer in test data

        // Code layer should have 3 nodes
        let code_layer = layers.iter().find(|l| l.id == "code").unwrap();
        assert_eq!(code_layer.node_ids.len(), 3);
    }

    #[test]
    fn test_build_tour() {
        let scan = make_scan_result();
        let mut nodes = Vec::new();

        for entry in &scan.files {
            nodes.push(GraphNode {
                id: node_id_from_path(&entry.path),
                node_type: file_category_to_node_type(&entry.file_category),
                name: display_name_from_path(&entry.path),
                file_path: Some(entry.path.clone()),
                line_range: None,
                summary: String::new(),
                tags: vec![],
                complexity: Complexity::Simple,
                language_notes: None,
                domain_meta: None,
                knowledge_meta: None,
            });
        }

        // Add a directory node
        nodes.push(GraphNode {
            id: node_id_from_path("src"),
            node_type: NodeType::Module,
            name: "src".to_string(),
            file_path: None,
            line_range: None,
            summary: "Directory: src".to_string(),
            tags: vec![],
            complexity: Complexity::Simple,
            language_notes: None,
            domain_meta: None,
            knowledge_meta: None,
        });

        let tour = build_tour(&nodes);
        assert!(!tour.is_empty());

        // First step should be project overview
        assert_eq!(tour[0].title, "Project Overview");
        assert_eq!(tour[0].order, 1);

        // Orders should be sequential
        for (i, step) in tour.iter().enumerate() {
            assert_eq!(step.order as usize, i + 1);
        }
    }

    #[test]
    fn test_build_graph_integration() {
        let scan = make_scan_result();
        let parsed = make_parsed_files();
        let root = std::env::current_dir().unwrap();

        let graph = build_graph(&root, &scan, &parsed);

        // Basic structure
        assert!(!graph.version.is_empty());
        assert_eq!(graph.kind, Some("codebase".to_string()));

        // Project metadata
        assert!(!graph.project.name.is_empty());
        assert!(!graph.project.languages.is_empty());
        assert!(!graph.project.description.is_empty());
        assert!(!graph.project.analyzed_at.is_empty());

        // Nodes: 6 files + directory nodes
        assert!(graph.nodes.len() >= 6);

        // Edges: directory structure + imports
        assert!(!graph.edges.is_empty());

        // Layers
        assert!(!graph.layers.is_empty());

        // Tour
        assert!(!graph.tour.is_empty());

        // JSON roundtrip
        let json = serde_json::to_string(&graph).unwrap();
        let parsed_graph: KnowledgeGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed_graph.nodes.len(), graph.nodes.len());
        assert_eq!(parsed_graph.edges.len(), graph.edges.len());
        assert_eq!(parsed_graph.layers.len(), graph.layers.len());
        assert_eq!(parsed_graph.tour.len(), graph.tour.len());
    }
}
