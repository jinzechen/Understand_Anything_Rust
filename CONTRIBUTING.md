# Contributing to Understand Anything Rust

Thank you for your interest in contributing! This document outlines the
process for contributing to the project.

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable, 1.70+)
- Git

### Setup

```bash
git clone https://github.com/jinzechen/Understand_Anything_Rust.git
cd Understand_Anything_Rust
cargo build
```

### Project Structure

```
Understand_Anything_Rust/
├── crates/
│   ├── ua-core/       # Core library: types, scanner, parser, graph builder
│   ├── ua-cli/        # CLI binary (ua) with scan/parse/build commands
│   └── ua-mcp/        # MCP server (Model Context Protocol)
├── Cargo.toml         # Workspace root
├── CHANGELOG.md
├── CODE_OF_CONDUCT.md
├── CONTRIBUTING.md
└── LICENSE
```

## Development Workflow

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Make your changes
4. Run tests: `cargo test --all`
5. Run formatting: `cargo fmt --all`
6. Run clippy: `cargo clippy --all -- -D warnings`
7. Commit your changes
8. Push to your fork
9. Open a Pull Request

## Code Style

- Follow standard Rust conventions (`rustfmt` default settings)
- Write doc comments (`///`) for all public items
- Keep functions small and focused
- Use `anyhow::Result` for fallible functions
- Add tests for new functionality

## Commit Messages

Use conventional commit format:

```
type(scope): description

feat: add incremental update support
fix: handle empty file list in scanner
docs: update README with new CLI flags
test: add fingerprint determinism test
```

Types: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`, `perf`

## Testing

```bash
# Run all tests
cargo test --all

# Run with output
cargo test --all -- --nocapture

# Run specific test
cargo test -p ua-core test_fingerprint_determinism
```

## Adding a New Language Parser

1. Implement `CodeParser` trait in `crates/ua-core/src/parser/`
2. Register it in `ParserRegistry::default()`
3. Add tests with sample code
4. Update the README with the new supported language

## Reporting Issues

- Use the GitHub issue tracker
- Include steps to reproduce, expected behavior, and actual behavior
- Include Rust version (`rustc --version`)
- For bugs, include a minimal reproducible example if possible

## License

By contributing, you agree that your contributions will be licensed under the
MIT License (same as the project).
