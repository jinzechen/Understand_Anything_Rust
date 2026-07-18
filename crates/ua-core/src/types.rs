//! Understand Anything — Core Data Model
//!
//! Exact 1:1 Rust port of the original TypeScript types.
//! 21 node types, 35 edge types, 3 domain types, 5 knowledge types.

use serde::{Deserialize, Serialize};

// ── Node Types (21 total) ───────────────────────────────────────────────────

/// Canonical node types matching the original 21-type taxonomy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    // Code (5)
    File,
    Function,
    Class,
    Module,
    Concept,
    // Configuration & Infrastructure (8)
    Config,
    Document,
    Service,
    Table,
    Endpoint,
    Pipeline,
    Schema,
    Resource,
    // Domain (3)
    Domain,
    Flow,
    Step,
    // Knowledge (5)
    Article,
    Entity,
    Topic,
    Claim,
    Source,
}

impl NodeType {
    /// Resolve LLM-generated aliases to canonical types.
    pub fn from_alias(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "file" => Some(Self::File),
            "function" | "func" | "fn" | "method" => Some(Self::Function),
            "class" | "interface" | "struct" => Some(Self::Class),
            "module" | "mod" | "pkg" | "package" => Some(Self::Module),
            "concept" => Some(Self::Concept),
            "config" | "setting" | "env" | "configuration" => Some(Self::Config),
            "document" | "doc" | "readme" | "docs" => Some(Self::Document),
            "service" | "container" | "deployment" | "pod" => Some(Self::Service),
            "table" | "migration" | "database" | "db" | "view" => Some(Self::Table),
            "endpoint" | "route" | "api" | "query" | "mutation" => Some(Self::Endpoint),
            "pipeline" | "job" | "ci" => Some(Self::Pipeline),
            "schema" | "proto" | "protobuf" => Some(Self::Schema),
            "resource" | "infra" | "infrastructure" | "terraform" => Some(Self::Resource),
            "domain" => Some(Self::Domain),
            "flow" => Some(Self::Flow),
            "step" => Some(Self::Step),
            "article" => Some(Self::Article),
            "entity" => Some(Self::Entity),
            "topic" => Some(Self::Topic),
            "claim" => Some(Self::Claim),
            "source" => Some(Self::Source),
            _ => None,
        }
    }
}

// ── Edge Types (35 total, 8 categories) ──────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    // Structural (5)
    Imports,
    Exports,
    Contains,
    Inherits,
    Implements,
    // Behavioral (4)
    Calls,
    Subscribes,
    Publishes,
    Middleware,
    // Data Flow (4)
    ReadsFrom,
    WritesTo,
    Transforms,
    Validates,
    // Dependencies (3)
    DependsOn,
    TestedBy,
    Configures,
    // Semantic (2)
    Related,
    SimilarTo,
    // Infrastructure (4)
    Deploys,
    Serves,
    Provisions,
    Triggers,
    // Schema/Data (4)
    Migrates,
    Documents,
    Routes,
    DefinesSchema,
    // Domain (3)
    ContainsFlow,
    FlowStep,
    CrossDomain,
    // Knowledge (6)
    Cites,
    Contradicts,
    BuildsOn,
    Exemplifies,
    CategorizedUnder,
    AuthoredBy,
}

// ── Graph Structures ─────────────────────────────────────────────────────────

/// A node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: NodeType,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_range: Option<(u32, u32)>,
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub complexity: Complexity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_meta: Option<DomainMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knowledge_meta: Option<KnowledgeMeta>,
}

/// Complexity level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Complexity {
    #[default]
    Simple,
    Moderate,
    Complex,
}

/// An edge connecting two nodes in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    #[serde(rename = "type")]
    pub edge_type: EdgeType,
    #[serde(default = "default_direction")]
    pub direction: Direction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default = "default_weight")]
    pub weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Forward,
    Backward,
    Bidirectional,
}

fn default_direction() -> Direction { Direction::Forward }
fn default_weight() -> f32 { 0.5 }

// ── Metadata ─────────────────────────────────────────────────────────────────

/// Domain metadata for domain/flow/step nodes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DomainMeta {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub business_rules: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cross_domain_interactions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_point: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_type: Option<String>,
}

/// Knowledge metadata for article/entity/topic/claim/source nodes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KnowledgeMeta {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wikilinks: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backlinks: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

// ── Project & Root Structures ────────────────────────────────────────────────

/// Logical grouping of nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub node_ids: Vec<String>,
}

/// A step in the guided tour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TourStep {
    pub order: u32,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub node_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_lesson: Option<String>,
}

/// Project-level metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub frameworks: Vec<String>,
    pub description: String,
    pub analyzed_at: String,
    pub git_commit_hash: String,
}

/// The root knowledge graph structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub project: ProjectMeta,
    #[serde(default)]
    pub nodes: Vec<GraphNode>,
    #[serde(default)]
    pub edges: Vec<GraphEdge>,
    #[serde(default)]
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub tour: Vec<TourStep>,
}

// ── Scan Result ──────────────────────────────────────────────────────────────

/// Entry from the file scanner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanEntry {
    pub path: String,
    pub language: String,
    pub size_lines: usize,
    pub file_category: FileCategory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileCategory {
    Code,
    Config,
    Docs,
    Infra,
    Script,
    Data,
    Test,
    Unknown,
}

/// Result of scanning a project directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub files: Vec<ScanEntry>,
    pub total_files: usize,
    pub filtered_by_ignore: usize,
    pub estimated_complexity: Complexity,
    pub stats: ScanStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStats {
    pub files_scanned: usize,
    pub by_category: std::collections::HashMap<String, usize>,
    pub by_language: std::collections::HashMap<String, usize>,
}

// ── Search ───────────────────────────────────────────────────────────────────

/// A search result from the fuzzy search engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub node_id: String,
    pub score: f32, // 0 = perfect match, 1 = worst match
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_type_aliases() {
        assert_eq!(NodeType::from_alias("func"), Some(NodeType::Function));
        assert_eq!(NodeType::from_alias("fn"), Some(NodeType::Function));
        assert_eq!(NodeType::from_alias("mod"), Some(NodeType::Module));
        assert_eq!(NodeType::from_alias("doc"), Some(NodeType::Document));
        assert_eq!(NodeType::from_alias("route"), Some(NodeType::Endpoint));
    }

    #[test]
    fn test_knowledge_graph_serialization() {
        let graph = KnowledgeGraph {
            version: "1.0.0".into(),
            kind: Some("codebase".into()),
            project: ProjectMeta {
                name: "test".into(),
                languages: vec!["rust".into()],
                frameworks: vec![],
                description: "test project".into(),
                analyzed_at: "2024-01-01".into(),
                git_commit_hash: "abc123".into(),
            },
            nodes: vec![],
            edges: vec![],
            layers: vec![],
            tour: vec![],
        };

        let json = serde_json::to_string(&graph).unwrap();
        let parsed: KnowledgeGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.project.name, "test");
        assert_eq!(parsed.project.languages, vec!["rust"]);
    }
}

// ── Parser Result Types ──────────────────────────────────────────────────────

/// A code definition extracted by the parser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionInfo {
    pub name: String,
    pub kind: String,
    pub line_range: (u32, u32),
    #[serde(default)]
    pub fields: Vec<String>,
}

/// An import statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    pub name: String,
    pub source: String,
    pub line_range: (u32, u32),
}

/// A document section (markdown headers, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SectionInfo {
    pub title: String,
    pub level: u32,
    pub line_range: (u32, u32),
}

/// An infrastructure service definition.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceInfo {
    pub name: String,
    pub image: Option<String>,
    pub ports: Vec<String>,
}

/// An API endpoint definition.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EndpointInfo {
    pub method: String,
    pub path: String,
    pub line_range: (u32, u32),
}

/// A pipeline/CI step definition.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StepInfo {
    pub name: String,
    pub command: Option<String>,
    pub line_range: (u32, u32),
}
