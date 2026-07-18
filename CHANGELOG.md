# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-07-18

### Added
- Incremental update support: only re-analyze files that changed since last run (`ua build --incremental`)
- File fingerprinting via blake3 for fast change detection
- `--full` flag to force a complete re-analysis
- `.understand-anything/meta.json` stores fingerprints and analysis metadata between runs
- `incremental` module in `ua-core` with `compute_fingerprints`, `read_meta`, `write_meta`, and `find_changed_files`
- Interactive dashboard improvements: dark theme, responsive design, search and filter
- Community files: CHANGELOG.md, CODE_OF_CONDUCT.md, CONTRIBUTING.md

### Changed
- CLI help text updated to include `--incremental` and `--full` flags
- `build` command refactored into dedicated `build_full` and `build_incremental` functions
- Bumped version to 0.2.0

## [0.1.0] - 2025-07-14

### Added
- Initial release of Understand Anything Rust
- Project scanner with language detection and file classification
- Regex-based Rust code parser (functions, structs, traits, enums, impls, imports)
- Knowledge graph builder with 21 node types and 35 edge types
- HTML report generation (static and interactive dashboard)
- Markdown report generation
- CLI (`ua`) with `scan`, `parse`, `build`, and `json` commands
- D3.js force-directed graph visualization in the HTML dashboard
- Layer-based architecture grouping
- Guided tour navigation
- Workspace structure with `ua-core`, `ua-cli`, and `ua-mcp` crates
- MIT license
