<h1 align="center">Understand Anything Rust</h1>

<p align="center">
  <strong>将任意代码库转化为交互式知识图谱 — 纯 Rust 实现</strong>
  <br />
  <em>支持作为 Rust 库嵌入、MCP Server 调用、CLI 命令行使用</em>
</p>

<p align="center">
  <a href="#-快速开始"><img src="https://img.shields.io/badge/快速开始-blue" alt="Quick Start" /></a>
  <a href="https://github.com/jinzechen/Understand_Anything_Rust/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-yellow" alt="License: MIT" /></a>
  <a href="#mcp-server"><img src="https://img.shields.io/badge/MCP_Server-8A2BE2" alt="MCP Server" /></a>
  <a href="https://github.com/jinzechen/Hermes_Rust_Operit_App"><img src="https://img.shields.io/badge/Hermes-内置-black" alt="Hermes" /></a>
</p>

<p align="center">
  <strong>Understand Anything 的纯 Rust 重写版</strong>
  <br />
  <em>原本是 <a href="https://github.com/Egonex-AI/Understand-Anything">Egonex-AI/Understand-Anything</a> 的 TypeScript 实现 (Claude Code Plugin)</em>
  <br />
  <em>本 Rust 版：无需 Node.js、无需 Claude Code、可嵌入任何 Rust 项目</em>
</p>

---

**你刚加入一个新团队，面对几万行代码。从哪开始？**

Understand Anything Rust 扫描你的项目，提取每个文件/函数/类的结构，构建知识图谱，然后你可以 JSON 导入、MCP 调用、或嵌入到 AI Agent 中。不再盲读代码，从全局视角理解系统。

> **目标不是图有多复杂来惊艳你 —— 而是默默告诉你每一块是怎么拼在一起的。**

---

## ✨ 核心功能

### 项目扫描
30+ 语言检测、文件分类（代码/配置/文档/基础设施）、复杂度评估。

### 代码解析
Rust 代码结构提取：函数、struct、trait、enum、impl、use 导入。可扩展的 `CodeParser` trait，支持注册更多语言解析器。

### 知识图谱构建
21 种节点类型、35 种边类型、架构分层、引导式学习路径。输出标准 `knowledge-graph.json`，与原始 Understand-Anything 格式完全兼容。

### MCP Server
通过 stdio JSON-RPC 暴露工具：`understand_scan`。可直接被 Claude Desktop、Hermes、Cursor 等 MCP 客户端调用。

### 可嵌入库
作为 Rust 依赖直接使用：
```toml
[dependencies]
ua-core = { git = "https://github.com/jinzechen/Understand_Anything_Rust" }
```

---

## 🚀 快速开始

### 1. 安装

```bash
git clone https://github.com/jinzechen/Understand_Anything_Rust.git
cd Understand_Anything_Rust
cargo build --release
```

### 2. 扫描项目

```bash
cargo run -p ua-cli -- scan /path/to/project
# 输出: 44 files | moderate
#   rust: 23 files
#   markdown: 19 files
#   ...
```

### 3. 构建知识图谱

```bash
cargo run -p ua-cli -- build /path/to/project
# 输出: [Phase 1/3] Scanning ...
#       [Phase 2/3] Parsing source files ...
#         Parsed 23 files
#       [Phase 3/3] Building knowledge graph ...
#         37 nodes, 46 edges, 6 layers, 5 tour steps
#         Graph written to .understand-anything/knowledge-graph.json
```

### 4. 作为 MCP Server 使用

```bash
cargo run -p ua-mcp
```

配置 Claude Desktop / Hermes / Cursor 的 MCP 连接指向此进程。

### 5. 作为 Rust 库使用

```rust
use ua_core::{scanner, parser, graph};

let scan = scanner::scan_project(Path::new("./my_project"))?;
let registry = parser::ParserRegistry::default();
let mut parsed = vec![];
for file in &scan.files {
    if let Ok(p) = registry.parse(&Path::new(&file.path)) {
        parsed.push(p);
    }
}
let knowledge_graph = graph::build_graph(Path::new("./my_project"), &scan, &parsed);
// knowledge_graph is fully JSON-serializable KnowledgeGraph
```

---

## 📊 数据模型

### 21 种节点类型
```
Code:    file, function, class, module, concept
Config:  config, document, service, table, endpoint, pipeline, schema, resource
Domain:  domain, flow, step
Knowledge: article, entity, topic, claim, source
```

### 35 种边类型
```
Structural:     imports, exports, contains, inherits, implements
Behavioral:     calls, subscribes, publishes, middleware
Data Flow:      reads_from, writes_to, transforms, validates
Dependencies:   depends_on, tested_by, configures
Semantic:       related, similar_to
Infrastructure: deploys, serves, provisions, triggers
Schema:         migrates, documents, routes, defines_schema
Domain:         contains_flow, flow_step, cross_domain
Knowledge:      cites, contradicts, builds_on, exemplifies, categorized_under, authored_by
```

---

## 🏗️ 架构

```
understand_anything_rust/
├── crates/
│   ├── ua-core/           ← 核心库 (类型 + 扫描器 + 解析器 + 图构建器)
│   │   └── src/
│   │       ├── types.rs   → 数据模型 (21 node + 35 edge types)
│   │       ├── scanner.rs → 文件扫描 + 语言检测
│   │       ├── parser/    → CodeParser trait + Rust 解析器
│   │       └── graph.rs   → 知识图谱构建器
│   ├── ua-mcp/            ← MCP stdio JSON-RPC Server
│   └── ua-cli/            ← CLI (scan / parse / build / json)
└── .github/workflows/     ← CI (cargo check + test + fmt)
```

---

## 🔧 与原始 Understand-Anything 的对比

| 特性 | TypeScript 原版 | Rust 版 |
|------|:---:|:---:|
| 项目扫描 | ✅ tree-sitter | ✅ walkdir + 扩展名检测 |
| 多语言解析器 | ✅ 30+ 语言 (tree-sitter) | 🚧 Rust (regex), 更多语言开发中 |
| LLM 多 Agent 分析 | ✅ Claude Code 子代理 | 🔮 计划中 (无需外部 LLM) |
| 知识图谱构建 | ✅ | ✅ 完全兼容格式 |
| Dashboard 可视化 | ✅ React SPA | 🔮 计划中 |
| 增量更新 | ✅ Git diff + fingerprint | 🔮 计划中 |
| MCP Server | ❌ | ✅ stdio JSON-RPC |
| 可嵌入库 | ❌ (需 Node.js) | ✅ 纯 Rust crate |
| 零外部依赖运行 | ❌ (需 Node.js + pnpm) | ✅ 单一二进制 |

---

## 📦 与团队共享知识图谱

图谱就是一份 JSON 文件 — 提交一次，团队成员就可以直接使用。

```bash
git add .understand-anything/knowledge-graph.json
git commit -m "Add project knowledge graph"
```

无需 Rust、无需 MCP、无需 LLM — 任何人拿到 JSON 文件就能导入到自己工具中使用。

---

## 🤝 整合到 Hermes_Rust_Operit_App

本 crate 设计为 [Hermes_Rust_Operit_App](https://github.com/jinzechen/Hermes_Rust_Operit_App) 的内置代码分析工具：

```toml
[dependencies]
ua-core = { path = "../Understand_Anything_Rust/crates/ua-core" }
```

通过 `ToolHandler` trait 暴露为 Agent 可调用的工具：

```rust
struct CodebaseAnalyzerTool;
impl ToolHandler for CodebaseAnalyzerTool {
    fn execute(&self, args: &Value) -> Result<String> {
        let scan = scanner::scan_project(Path::new(&args["path"]))?;
        let graph = graph::build_graph(/* ... */);
        Ok(serde_json::to_string(&graph)?)
    }
}
```

---

## 📝 许可证

MIT License

---

<p align="center">
  <strong>不再盲读代码，理解整个系统</strong>
</p>

<p align="center">
  <em>Rust 重写版由 <a href="https://github.com/jinzechen">jinzechen</a> 开发 — 基于 <a href="https://github.com/Egonex-AI/Understand-Anything">Egonex-AI/Understand-Anything</a> (MIT License)</em>
</p>
