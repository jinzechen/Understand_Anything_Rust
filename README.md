<h1 align="center">
  <img src="https://raw.githubusercontent.com/tandpfun/skill-icons/main/icons/Rust.svg" width="32" alt="Rust" />
  Understand Anything Rust
  <img src="https://raw.githubusercontent.com/tandpfun/skill-icons/main/icons/Rust.svg" width="32" alt="Rust" />
</h1>

<p align="center">
  <strong>Turn any codebase into an interactive knowledge graph — pure Rust</strong>
  <br />
  <em>Library · CLI · MCP Server · Embedded in <a href="https://github.com/jinzechen/Hermes_Rust_Operit_App">Hermes</a></em>
</p>

<p align="center">
  <a href="https://github.com/jinzechen/Understand_Anything_Rust/actions"><img src="https://img.shields.io/badge/CI-passing-brightgreen" alt="CI" /></a>
  <a href="https://crates.io/crates/ua-core"><img src="https://img.shields.io/badge/crates.io-soon-orange" alt="crates.io" /></a>
  <a href="#mcp-server"><img src="https://img.shields.io/badge/MCP_Server-ready-8A2BE2" alt="MCP Server" /></a>
  <a href="https://github.com/jinzechen/Understand_Anything_Rust/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-yellow" alt="License: MIT" /></a>
  <a href="https://github.com/jinzechen/Hermes_Rust_Operit_App"><img src="https://img.shields.io/badge/Hermes-built--in-black" alt="Hermes" /></a>
</p>

<p align="center">
  <strong>A pure Rust rewrite of <a href="https://github.com/Egonex-AI/Understand-Anything">Egonex-AI/Understand-Anything</a></strong>
  <br />
  <em>No Node.js · No Claude Code required · Single binary · Embeddable</em>
</p>

---

**You just joined a new team. 50,000 lines of code. Where do you start?**

Understand Anything Rust scans your project, extracts every file, function, class, and dependency, builds a knowledge graph, and serves it as an interactive dashboard, a static report, or a JSON artifact you can feed into any AI agent.

> **The goal isn't a graph that dazzles you with complexity — it's a map that quietly shows you how every piece fits together.**

---

## ✨ Features

| | Feature | Description |
|---|---------|-------------|
| 🔍 | **Project Scanning** | 30+ language detection, file classification (code/config/docs/infra/test), complexity estimation |
| 📝 | **Code Parsing** | Extensible `CodeParser` trait — Rust parser built-in, register your own |
| 🧠 | **Knowledge Graph** | 21 node types, 35 edge types, architecture layers, guided learning tour |
| ⚡ | **Incremental Updates** | blake3 file fingerprinting — only re-analyze changed files |
| 🤖 | **LLM Multi-Agent** | AgentDispatcher trait + 4 prompt templates (file-analyzer, architecture-analyzer, tour-builder, graph-reviewer) |
| 🖥️ | **Interactive Dashboard** | D3.js force-directed graph, search, layer filter, tour navigation, dark theme |
| 📄 | **Multi-Format Reports** | Self-contained HTML (interactive), Markdown (static), JSON (machine-readable) |
| 🔌 | **MCP Server** | stdio JSON-RPC — compatible with Claude Desktop, Cursor, Hermes |
| 📦 | **Embeddable Library** | `cargo add ua-core` — use in any Rust project |
| 🚀 | **Zero Dependencies at Runtime** | Single binary, no Node.js, no Python, no external services |

---

## 🚀 Quick Start

### 1. Install

```bash
git clone https://github.com/jinzechen/Understand_Anything_Rust.git
cd Understand_Anything_Rust
cargo build --release
```

### 2. Scan a project

```bash
cargo run -p ua-cli -- scan /path/to/your/project

# Output:
#  44 files | moderate
#    rust: 23 files
#    markdown: 11 files
#    json: 5 files
#    toml: 3 files
#    yaml: 2 files
```

### 3. Build the knowledge graph

```bash
cargo run -p ua-cli -- build /path/to/your/project

# Output:
#  [Phase 1/3] Scanning ...
#  [Phase 2/3] Parsing source files ...
#    Parsed 23 files
#  [Phase 3/3] Building knowledge graph ...
#    37 nodes, 46 edges, 6 layers, 5 tour steps
#  [Fingerprint] Computing file hashes ...
#    Saved fingerprints for 44 files to .understand-anything/meta.json
#    Graph written to .understand-anything/knowledge-graph.json
```

---

## 📖 Usage

### CLI

```bash
# Scan project structure
ua scan ./my-project

# Parse code files (extract functions, structs, imports)
ua parse ./my-project

# Build knowledge graph (JSON output)
ua build ./my-project

# Incremental — only re-analyze changed files
ua build --incremental ./my-project

# Force full re-analysis
ua build --full ./my-project

# Generate interactive HTML dashboard
ua build --format html ./my-project

# Generate static Markdown report
ua build --format md ./my-project

# Output scan result as JSON
ua json ./my-project
```

### Library

Add to your `Cargo.toml`:

```toml
[dependencies]
ua-core = { git = "https://github.com/jinzechen/Understand_Anything_Rust" }
```

Then:

```rust
use ua_core::{scanner, parser, graph, report};
use std::path::Path;

// Phase 1: Scan
let scan = scanner::scan_project(Path::new("./my_project"))?;

// Phase 2: Parse
let registry = parser::ParserRegistry::default();
let parsed: Vec<_> = scan.files.iter()
    .filter(|f| f.file_category == ua_core::FileCategory::Code)
    .filter_map(|f| registry.parse(&Path::new(&f.path)).ok())
    .collect();

// Phase 3: Build graph
let kg = graph::build_graph(Path::new("./my_project"), &scan, &parsed);

// Phase 4: Export
let html = report::to_html(&kg);           // Interactive dashboard
let md   = report::to_markdown(&kg);       // Static documentation
let json = serde_json::to_string_pretty(&kg)?; // Machine-readable
```

### MCP Server

Start the MCP server:

```bash
cargo run -p ua-mcp
```

Configure in Claude Desktop (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "understand-anything": {
      "command": "cargo",
      "args": ["run", "-p", "ua-mcp"],
      "cwd": "/path/to/Understand_Anything_Rust"
    }
  }
}
```

Available tool: `understand_scan` — scan any project directory from your AI assistant.

### Hermes Integration

Built into [Hermes_Rust_Operit_App](https://github.com/jinzechen/Hermes_Rust_Operit_App) via the `analyze_codebase` ToolHandler:

```toml
[dependencies]
ua-core = { path = "../Understand_Anything_Rust/crates/ua-core" }
```

```rust
impl ToolHandler for CodebaseAnalyzerTool {
    fn execute(&self, args: &Value) -> Result<String> {
        let scan = scanner::scan_project(Path::new(&args["path"]))?;
        let graph = graph::build_graph(/* ... */);
        Ok(serde_json::to_string(&graph)?)
    }
}
```

---

## 📊 Comparison: Rust vs Upstream Understand-Anything

| Capability | TypeScript Upstream | Rust Rewrite |
|---|---|---|
| Project Scanning | ✅ tree-sitter | ✅ walkdir + extension-based |
| Multi-language Parsers | ✅ 30+ via tree-sitter | 🚧 Rust (regex), extensible `CodeParser` trait |
| **LLM Multi-Agent Analysis** | ✅ Claude Code sub-agents | ✅ AgentDispatcher trait + 4 prompt templates |
| Knowledge Graph | ✅ 21 nodes / 35 edges | ✅ Full 1:1 format compatibility |
| **Incremental Updates** | ✅ Git diff + fingerprint | ✅ blake3 fingerprinting + `--incremental` flag |
| **Interactive Dashboard** | ✅ React SPA | ✅ D3.js force-directed graph (self-contained HTML) |
| **MCP Server** | ❌ | ✅ stdio JSON-RPC |
| **Embeddable Library** | ❌ (Node.js only) | ✅ Pure Rust crate (`cargo add ua-core`) |
| **Zero Runtime Dependencies** | ❌ (Node.js + pnpm) | ✅ Single binary |
| **Markdown Reports** | ❌ | ✅ Human-readable static docs |
| CI/CD | ✅ | ✅ cargo check + test + fmt |
| Language | TypeScript | Rust 🦀 |

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Understand Anything Rust                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐                    │
│  │  ua-cli  │   │  ua-mcp  │   │  Hermes  │   ← Consumers     │
│  │  (bin)   │   │  (bin)   │   │ (embedded)│                   │
│  └────┬─────┘   └────┬─────┘   └────┬─────┘                    │
│       │               │              │                          │
│       └───────────────┼──────────────┘                          │
│                       │                                         │
│               ┌───────▼────────┐                                │
│               │    ua-core     │   ← Core Library               │
│               │    (lib)       │                                │
│               ├────────────────┤                                │
│               │  types.rs      │   21 node + 35 edge types      │
│               │  scanner.rs    │   File enumeration + lang det  │
│               │  parser/mod.rs │   CodeParser trait + registry  │
│               │  graph.rs      │   Graph builder (SHA-256 IDs)  │
│               │  incremental.rs│   blake3 fingerprint + cache   │
│               │  agent.rs      │   LLM multi-agent adapter      │
│               │  dashboard.rs  │   D3.js interactive HTML       │
│               │  report.rs     │   HTML + Markdown export       │
│               └────────────────┘                                │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘

Output Formats:
  ┌──────────────────────┐  ┌──────────────────┐  ┌──────────────┐
  │ knowledge-graph.json │  │   report.html     │  │  report.md   │
  │   CI / API / sharing │  │ Interactive dash  │  │  Static docs │
  └──────────────────────┘  └──────────────────┘  └──────────────┘
```

### Crate Structure

```
understand_anything_rust/
├── crates/
│   ├── ua-core/              ← Core library
│   │   └── src/
│   │       ├── types.rs      → Data model (21 node + 35 edge types)
│   │       ├── scanner.rs    → File scanner (30+ languages)
│   │       ├── parser/mod.rs → CodeParser trait + Rust parser
│   │       ├── graph.rs      → Knowledge graph builder
│   │       ├── incremental.rs→ blake3 fingerprinting + change detection
│   │       ├── agent.rs      → LLM multi-agent adapter (feature-gated)
│   │       ├── dashboard.rs  → Self-contained D3.js dashboard
│   │       └── report.rs     → HTML + Markdown report generator
│   ├── ua-cli/               ← CLI binary (`ua`)
│   └── ua-mcp/               ← MCP stdio JSON-RPC server
└── .github/workflows/        ← CI
```

---

## 📊 Generated Outputs Showcase

### Interactive Dashboard (`--format html`)

The self-contained HTML dashboard provides:

- **Force-Directed Graph** — D3.js simulation with draggable nodes, zoom/pan
- **Search Bar** — Real-time node filtering by name, path, or tags
- **Layer Toggle** — Show/hide nodes by architecture layer (code, config, docs, infra, api, data)
- **Guided Tour** — Step-by-step walkthrough of the codebase with prev/next navigation
- **Node Detail Panel** — Click any node to see type, summary, complexity, tags, language
- **Stats Bar** — Node count, edge count, layer count, language count, aggregate complexity
- **Dark Theme** — GitHub-dark inspired, responsive (mobile-friendly)

### Markdown Report (`--format md`)

Clean, human-readable documentation including:
- Project overview table (name, languages, complexity, analyzed timestamp, git commit)
- Architecture layers with file listings
- Complete file inventory (path, language, line count, category)
- Import relationship map
- Directory tree visualization (ASCII art)
- Guided tour steps with language lessons

### JSON Knowledge Graph (`--format json`)

Machine-readable artifact:

```json
{
  "version": "0.2.0",
  "kind": "codebase",
  "project": {
    "name": "my-project",
    "languages": ["rust", "markdown", "toml"],
    "description": "...",
    "analyzed_at": "2025-07-18T12:00:00Z",
    "git_commit_hash": "abc1234"
  },
  "nodes": [
    {
      "id": "a1b2c3d4e5f6g7h8",
      "type": "file",
      "name": "main",
      "file_path": "src/main.rs",
      "summary": "rust (150 lines, 5 definitions, 3 imports)",
      "tags": ["rust"],
      "complexity": "moderate"
    }
  ],
  "edges": [
    {
      "source": "a1b2c3d4...",
      "target": "b2c3d4e5...",
      "type": "imports",
      "direction": "forward",
      "description": "main imports types",
      "weight": 0.5
    }
  ],
  "layers": [
    {
      "id": "code",
      "name": "Core Code",
      "description": "Source code files and modules",
      "node_ids": ["a1b2...", "b2c3..."]
    }
  ],
  "tour": [
    {
      "order": 1,
      "title": "Project Overview",
      "description": "This project contains 37 files...",
      "node_ids": ["a1b2..."],
      "language_lesson": "Optional teaching note about a pattern"
    }
  ]
}
```

---

## ⚡ Incremental Update Workflow

After the first full build, subsequent runs use blake3 fingerprints to skip unchanged files:

```
First run:
  ua build ./project
  → Full scan + parse + build
  → Saves .understand-anything/meta.json (fingerprints)

Subsequent runs:
  ua build --incremental ./project
  → Scan → Compare fingerprints → Only re-parse changed files
  → Merge into existing graph → Update meta.json

Changed 3 of 44 files → ~10x faster than full rebuild
```

```text
[Incremental] Previous analysis: 2025-07-17T10:00:00Z (44 files fingerprinted)
[Phase 1/3] Scanning ...
  Found 44 files
[Phase 1b] Computing current fingerprints ...
  3 files changed since last analysis
    - src/incremental.rs
    - src/agent.rs
    - Cargo.toml
[Phase 2/3] Re-parsing all files ...
  39 nodes, 48 edges, 6 layers, 5 tour steps
[Fingerprint] Updating meta ...
  Saved fingerprints to .understand-anything/meta.json
```

---

## 🤖 LLM Multi-Agent Pipeline

The `llm-analysis` feature flag enables semantic analysis via LLM sub-agents, matching the original Understand-Anything's Claude Code pipeline:

```
┌─────────────────────────────────────────────────────────┐
│              LLM Multi-Agent Pipeline                    │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌──────────────┐                                        │
│  │  Phase 1     │  scanner::scan_project()               │
│  │  Scan files  │  file inventory + language detection   │
│  └──────┬───────┘                                        │
│         │                                                │
│  ┌──────▼───────┐                                        │
│  │  Phase 2     │  build_graph_with_llm()                │
│  │  Analyze     │                                         │
│  │              │  ┌──────────────────────────────┐     │
│  │              │  │  Agent: file-analyzer         │     │
│  │              │  │  → Extract functions/classes  │     │
│  │              │  │  → Write summaries + tags     │     │
│  │              │  │  → Assign complexity          │     │
│  │              │  └──────────────┬───────────────┘     │
│  │              │                 │                      │
│  │              │  ┌──────────────▼───────────────┐     │
│  │              │  │  Agent: architecture-analyzer │     │
│  │              │  │  → Identify layers            │     │
│  │              │  │  → Group nodes logically      │     │
│  │              │  └──────────────┬───────────────┘     │
│  │              │                 │                      │
│  │              │  ┌──────────────▼───────────────┐     │
│  │              │  │  Agent: tour-builder          │     │
│  │              │  │  → Create guided walkthrough  │     │
│  │              │  │  → 5-10 sequential steps      │     │
│  │              │  └──────────────┬───────────────┘     │
│  │              │                 │                      │
│  │              │  ┌──────────────▼───────────────┐     │
│  │              │  │  Agent: graph-reviewer (opt)  │     │
│  │              │  │  → Validate dangling edges    │     │
│  │              │  │  → Check coverage + layers    │     │
│  │              │  └──────────────────────────────┘     │
│  └──────┬───────┘                                        │
│         │                                                │
│  ┌──────▼───────┐                                        │
│  │  Phase 3     │  report::export_report()               │
│  │  Export      │  JSON / HTML / Markdown                │
│  └──────────────┘                                        │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

The `AgentDispatcher` trait keeps the core library runtime-agnostic — implement it once and plug in any LLM backend.

---

## 📊 Data Model

### 21 Node Types

```
Code:       file, function, class, module, concept
Config:     config, document, service, table, endpoint, pipeline, schema, resource
Domain:     domain, flow, step
Knowledge:  article, entity, topic, claim, source
```

### 35 Edge Types (8 categories)

| Category | Edge Types |
|---|---|
| **Structural** | imports, exports, contains, inherits, implements |
| **Behavioral** | calls, subscribes, publishes, middleware |
| **Data Flow** | reads_from, writes_to, transforms, validates |
| **Dependencies** | depends_on, tested_by, configures |
| **Semantic** | related, similar_to |
| **Infrastructure** | deploys, serves, provisions, triggers |
| **Schema** | migrates, documents, routes, defines_schema |
| **Domain** | contains_flow, flow_step, cross_domain |
| **Knowledge** | cites, contradicts, builds_on, exemplifies, categorized_under, authored_by |

---

## 📦 Version History

See [CHANGELOG.md](CHANGELOG.md) for full details.

| Version | Date | Highlights |
|---|---|---|
| **v0.2.0** | 2025-07-18 | Incremental updates (blake3), LLM multi-agent adapter, interactive dashboard, `--format` flag |
| v0.1.0 | 2025-07-14 | Initial release: scanner, parser, graph builder, HTML/MD reports, MCP server |

---

## 🗺️ Roadmap

- [x] Project scanner with 30+ language detection
- [x] Rust code parser (regex-based)
- [x] Knowledge graph builder (21 node / 35 edge types)
- [x] HTML interactive dashboard (D3.js)
- [x] Markdown static reports
- [x] MCP stdio JSON-RPC server
- [x] Incremental analysis (blake3 fingerprinting)
- [x] LLM multi-agent adapter (AgentDispatcher trait)
- [ ] crates.io publication (`ua-core`, `ua-cli`, `ua-mcp`)
- [ ] Additional language parsers (TypeScript, Python, Go, Java)
- [ ] tree-sitter integration for accurate AST parsing
- [ ] WebSocket/HTTP MCP transport
- [ ] VSCode extension for in-editor graph visualization
- [ ] CI/CD integration (GitHub Actions annotation on PRs)
- [ ] Package managers (Homebrew, cargo-binstall, npm wrapper)

---

## 👥 Community

- **Contributing**: See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines
- **Code of Conduct**: See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) (Contributor Covenant 2.1)
- **Issues**: Use [GitHub Issues](https://github.com/jinzechen/Understand_Anything_Rust/issues)
- **Discussions**: Use [GitHub Discussions](https://github.com/jinzechen/Understand_Anything_Rust/discussions)

---

## 📝 License

MIT License — see [LICENSE](https://github.com/jinzechen/Understand_Anything_Rust/blob/main/LICENSE) for details.

---

<p align="center">
  <strong>Stop reading code blind. Understand the whole system.</strong>
</p>

<p align="center">
  <em>Rust rewrite by <a href="https://github.com/jinzechen">jinzechen</a> — based on <a href="https://github.com/Egonex-AI/Understand-Anything">Egonex-AI/Understand-Anything</a> (MIT License)</em>
</p>
