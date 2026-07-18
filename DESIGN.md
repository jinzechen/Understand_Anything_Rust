# Understand_Anything_Rust — Architecture Design v1.0

## Goal

Pure Rust port of [Understand-Anything](https://github.com/jinzechen/Understand-Anything), usable as:
- **Library** (`understand_anything_core`) — direct dependency of Hermes_Rust_Operit_App
- **MCP Server** — standalone binary exposing tools via stdio JSON-RPC
- **CLI** — command-line scanning and graph generation

## Architecture

```
understand_anything_rust/
├── Cargo.toml              # workspace
├── crates/
│   ├── ua-core/            # lib: data model + scanner + parser + graph + search
│   ├── ua-mcp/             # bin: MCP server (stdio JSON-RPC)
│   └── ua-cli/             # bin: CLI (scan, build, serve)
└── README.md
```

## Module Design (ua-core)

```
src/
├── lib.rs                  # re-exports
├── types.rs                # GraphNode, GraphEdge, KnowledgeGraph, etc.
├── scanner.rs              # File enumeration, language detection, .understandignore
├── parser/
│   ├── mod.rs              # Parser trait + registry
│   ├── rust.rs             # tree-sitter-rust extractor
│   ├── typescript.rs       # tree-sitter-typescript
│   ├── python.rs           # tree-sitter-python
│   └── ...                 # 10+ language parsers (start with top 10)
├── graph.rs                # GraphBuilder: nodes, edges, layers, tours
├── search.rs               # Fuzzy search over nodes (tantivy or custom)
├── fingerprint.rs          # File hash for incremental analysis
└── ignore.rs               # .understandignore parsing
```

## Data Model (exact 1:1 port from types.ts)

### Node Types (21)
```
Code:    file, function, class, module, concept
Config:  config, document, service, table, endpoint, pipeline, schema, resource
Domain:  domain, flow, step
Knowledge: article, entity, topic, claim, source
```

### Edge Types (35, 8 categories)
```
Structural:     imports, exports, contains, inherits, implements
Behavioral:     calls, subscribes, publishes, middleware
Data Flow:      reads_from, writes_to, transforms, validates
Dependencies:   depends_on, tested_by, configures
Semantic:       related, similar_to
Infra:          deploys, serves, provisions, triggers
Schema:         migrates, documents, routes, defines_schema
Domain:         contains_flow, flow_step, cross_domain
Knowledge:      cites, contradicts, builds_on, exemplifies, categorized_under, authored_by
```

## API Surface

### Library API (ua-core)
```rust
// scan a project, return file inventory
pub fn scan_project(root: &Path) -> Result<ScanResult>;

// parse code files with tree-sitter, return structural analysis
pub fn parse_files(files: &[ScanEntry], language: &str) -> Result<Vec<ParsedFile>>;

// build knowledge graph from parsed files
pub fn build_graph(project: ProjectMeta, parsed: Vec<ParsedFile>) -> Result<KnowledgeGraph>;

// fuzzy search over graph nodes
pub fn search_graph(graph: &KnowledgeGraph, query: &str) -> Vec<SearchResult>;
```

### MCP Server API (ua-mcp)
```
Tools exposed:
  - understand_scan  → scan-project.mjs equivalent
  - understand_parse → parse specific files
  - understand_graph → build knowledge graph
  - understand_search → search the graph
```

## Dependencies

```toml
[dependencies]
# Core
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
anyhow = "1"

# File scanning
walkdir = "2"
ignore = "0.4"          # .gitignore / .understandignore support

# Code parsing (tree-sitter)
tree-sitter = "0.24"
tree-sitter-rust = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-python = "0.23"
# Add more languages as needed

# Search
tantivy = "0.22"        # full-text search (optional, fuse.js equivalent)

# Fingerprinting
sha2 = "0.10"

# MCP Server (optional)
# no external MCP SDK — use our stdio JSON-RPC pattern from Hermes_Rust_Operit_App
tokio = { version = "1", features = ["full"] }
```

## Integration with Hermes_Rust_Operit_App

```toml
# In Hermes_Rust_Operit_App's Cargo.toml:
[dependencies]
ua-core = { path = "../Understand_Anything_Rust/crates/ua-core" }
```

Or as Git dependency:
```toml
ua-core = { git = "https://github.com/jinzechen/Understand_Anything_Rust" }
```

## Phase Plan

| Phase | What | Files |
|-------|------|-------|
| 1 | Project scaffold + types.rs | Cargo.toml, types.rs |
| 2 | Scanner (walkdir + language detection) | scanner.rs, ignore.rs |
| 3 | Parser trait + Rust parser (tree-sitter) | parser/mod.rs, parser/rust.rs |
| 4 | +5 more language parsers | parser/*.rs |
| 5 | Graph builder | graph.rs |
| 6 | Search engine | search.rs |
| 7 | Fingerprint + incremental | fingerprint.rs |
| 8 | MCP server binary | ua-mcp/ |
| 9 | CLI binary | ua-cli/ |
| 10 | CI/CD + crates.io publish | .github/workflows/ |
