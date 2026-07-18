# Understand_Anything_Rust

Pure Rust port of [Understand-Anything](https://github.com/jinzechen/Understand-Anything) — codebase analysis & interactive knowledge graph engine.

## Features

- **Project Scanner** — file enumeration, language detection, complexity estimation
- **Code Parser** — regex-based structural extraction (fn/struct/trait/enum/impl/imports)
- **Knowledge Graph** — 21 node types, 35 edge types, layers, guided tours
- **MCP Server** — stdio JSON-RPC, tool: `understand_scan`
- **CLI** — `ua scan`, `ua parse`, `ua build`

## Usage

### Library
```toml
[dependencies]
ua-core = { git = "https://github.com/jinzechen/Understand_Anything_Rust" }
```

### MCP Server
```bash
cargo run -p ua-mcp
```

### CLI
```bash
cargo run -p ua-cli -- scan /path/to/project
cargo run -p ua-cli -- parse /path/to/project
```

## Integration

Designed to be embedded in [Hermes_Rust_Operit_App](https://github.com/jinzechen/Hermes_Rust_Operit_App) as a code analysis tool.

## License

MIT
