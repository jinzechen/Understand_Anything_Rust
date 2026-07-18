//! Code parser trait and registry.
//!
//! Extracts structural information (functions, classes, imports, etc.)
//! from source code using tree-sitter grammars.

use std::path::Path;

use crate::types::{DefinitionInfo, EndpointInfo, SectionInfo, ServiceInfo, StepInfo};

/// Result of parsing a single source file.
#[derive(Debug, Clone)]
pub struct ParsedFile {
    /// File path relative to project root.
    pub path: String,
    /// Detected language.
    pub language: String,
    /// Line count.
    pub line_count: usize,
    /// Top-level definitions (functions, classes, modules).
    pub definitions: Vec<DefinitionInfo>,
    /// Import statements (what this file imports).
    pub imports: Vec<ImportInfo>,
    /// Document sections (for markdown, etc.).
    pub sections: Vec<SectionInfo>,
    /// Infrastructure definitions (Docker services, etc.).
    pub services: Vec<ServiceInfo>,
    /// API endpoints (HTTP routes, GraphQL queries, etc.).
    pub endpoints: Vec<EndpointInfo>,
    /// Pipeline steps (CI stages, workflow steps).
    pub steps: Vec<StepInfo>,
}

/// Information about an import statement.
#[derive(Debug, Clone)]
pub struct ImportInfo {
    /// What is being imported.
    pub name: String,
    /// Where it's imported from (module path).
    pub source: String,
    /// Line range in the source file.
    pub line_range: (u32, u32),
}

/// Trait for language-specific code parsers.
pub trait CodeParser: Send + Sync {
    /// The language identifier this parser handles (e.g., "rust", "python").
    fn language(&self) -> &str;

    /// Supported file extensions for this language.
    fn extensions(&self) -> &[&str];

    /// Parse a source file and extract structural information.
    fn parse_file(&self, path: &Path, content: &str) -> anyhow::Result<ParsedFile>;
}

/// Registry of language parsers.
pub struct ParserRegistry {
    parsers: Vec<Box<dyn CodeParser>>,
}

impl ParserRegistry {
    pub fn new() -> Self {
        Self { parsers: Vec::new() }
    }

    pub fn register(&mut self, parser: Box<dyn CodeParser>) {
        self.parsers.push(parser);
    }

    /// Find the parser for a given file path.
    pub fn find_for(&self, path: &Path) -> Option<&dyn CodeParser> {
        let ext = path.extension()?.to_str()?;
        self.parsers
            .iter()
            .find(|p| p.extensions().contains(&ext))
            .map(|p| p.as_ref())
    }

    /// Parse a file with the appropriate parser.
    pub fn parse(&self, path: &Path) -> anyhow::Result<ParsedFile> {
        let parser = self
            .find_for(path)
            .ok_or_else(|| anyhow::anyhow!("No parser found for: {}", path.display()))?;

        let content = std::fs::read_to_string(path)?;
        parser.parse_file(path, &content)
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        // Register Rust parser
        registry.register(Box::new(RustParser));
        registry
    }
}

// ── Rust Parser ──────────────────────────────────────────────────────────────

/// Tree-sitter based Rust code parser.
pub struct RustParser;

impl CodeParser for RustParser {
    fn language(&self) -> &str {
        "rust"
    }

    fn extensions(&self) -> &[&str] {
        &["rs"]
    }

    fn parse_file(&self, path: &Path, content: &str) -> anyhow::Result<ParsedFile> {
        let line_count = content.lines().count();
        let mut definitions = Vec::new();
        let mut imports = Vec::new();

        // Use regex-based extraction (tree-sitter optional for now)
        use regex::Regex;

        // Extract function definitions
        let fn_re = Regex::new(
            r"(?m)^\s*(?:pub(?:\s*\(\s*(?:crate|super|self)\s*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+(?:"[^"]*"\s+)?)?fn\s+(\w+)"
        ).unwrap();
        for cap in fn_re.captures_iter(content) {
            let name = cap[1].to_string();
            if name != "main" && !name.starts_with("test_") {
                let line = content[..cap.get(0).unwrap().start()]
                    .chars()
                    .filter(|&c| c == '\n')
                    .count() as u32 + 1;
                definitions.push(DefinitionInfo {
                    name,
                    kind: "function".to_string(),
                    line_range: (line, line),
                    fields: Vec::new(),
                });
            }
        }

        // Extract struct definitions
        let struct_re = Regex::new(
            r"(?m)^\s*(?:pub(?:\s*\(\s*(?:crate|super|self)\s*\))?\s+)?struct\s+(\w+)"
        ).unwrap();
        for cap in struct_re.captures_iter(content) {
            let name = cap[1].to_string();
            let line = content[..cap.get(0).unwrap().start()]
                .chars()
                .filter(|&c| c == '\n')
                .count() as u32 + 1;
            definitions.push(DefinitionInfo {
                name,
                kind: "struct".to_string(),
                line_range: (line, line),
                fields: Vec::new(),
            });
        }

        // Extract trait definitions
        let trait_re = Regex::new(
            r"(?m)^\s*(?:pub(?:\s*\(\s*(?:crate|super|self)\s*\))?\s+)?(?:unsafe\s+)?trait\s+(\w+)"
        ).unwrap();
        for cap in trait_re.captures_iter(content) {
            let name = cap[1].to_string();
            let line = content[..cap.get(0).unwrap().start()]
                .chars()
                .filter(|&c| c == '\n')
                .count() as u32 + 1;
            definitions.push(DefinitionInfo {
                name,
                kind: "trait".to_string(),
                line_range: (line, line),
                fields: Vec::new(),
            });
        }

        // Extract enum definitions
        let enum_re = Regex::new(
            r"(?m)^\s*(?:pub(?:\s*\(\s*(?:crate|super|self)\s*\))?\s+)?enum\s+(\w+)"
        ).unwrap();
        for cap in enum_re.captures_iter(content) {
            let name = cap[1].to_string();
            let line = content[..cap.get(0).unwrap().start()]
                .chars()
                .filter(|&c| c == '\n')
                .count() as u32 + 1;
            definitions.push(DefinitionInfo {
                name,
                kind: "enum".to_string(),
                line_range: (line, line),
                fields: Vec::new(),
            });
        }

        // Extract impl blocks
        let impl_re = Regex::new(
            r"(?m)^\s*(?:pub(?:\s*\(\s*(?:crate|super|self)\s*\))?\s+)?(?:unsafe\s+)?impl(?:\s*<\s*\w+(?:\s*:\s*\w+(?:\s*\+\s*\w+)*)?\s*>)?\s+(\w+(?:::)?\w*)"
        ).unwrap();
        for cap in impl_re.captures_iter(content) {
            let name = format!("impl {}", cap[1].to_string());
            let line = content[..cap.get(0).unwrap().start()]
                .chars()
                .filter(|&c| c == '\n')
                .count() as u32 + 1;
            definitions.push(DefinitionInfo {
                name,
                kind: "implementation".to_string(),
                line_range: (line, line),
                fields: Vec::new(),
            });
        }

        // Extract use/import statements
        let use_re = Regex::new(r"(?m)^\s*use\s+([^;]+);").unwrap();
        for cap in use_re.captures_iter(content) {
            let source = cap[1].trim().to_string();
            let name = source.split("::").last().unwrap_or(&source).to_string();
            let line = content[..cap.get(0).unwrap().start()]
                .chars()
                .filter(|&c| c == '\n')
                .count() as u32 + 1;
            imports.push(ImportInfo {
                name,
                source,
                line_range: (line, line),
            });
        }

        Ok(ParsedFile {
            path: path.to_string_lossy().replace('\\', "/"),
            language: "rust".to_string(),
            line_count,
            definitions,
            imports,
            sections: Vec::new(),
            services: Vec::new(),
            endpoints: Vec::new(),
            steps: Vec::new(),
        })
    }
}

// ── Generic/Plaintext Parser ─────────────────────────────────────────────────

/// Fallback parser for unsupported languages.
pub struct PlaintextParser;

impl CodeParser for PlaintextParser {
    fn language(&self) -> &str { "plaintext" }
    fn extensions(&self) -> &[&str] { &["txt", "cfg", "ini", "log"] }

    fn parse_file(&self, path: &Path, content: &str) -> anyhow::Result<ParsedFile> {
        Ok(ParsedFile {
            path: path.to_string_lossy().replace('\\', "/"),
            language: "plaintext".to_string(),
            line_count: content.lines().count(),
            definitions: Vec::new(),
            imports: Vec::new(),
            sections: Vec::new(),
            services: Vec::new(),
            endpoints: Vec::new(),
            steps: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_parser_functions() {
        let code = r#"
pub fn hello() -> String {
    "world".into()
}

async fn load_data() -> Result<Vec<u8>> {
    Ok(vec![])
}

pub(crate) fn internal() {}

pub struct Config {
    pub name: String,
}

pub trait Handler {
    fn handle(&self);
}

impl Handler for Config {
    fn handle(&self) {}
}

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
"#;
        let parser = RustParser;
        let result = parser.parse_file(std::path::Path::new("test.rs"), code).unwrap();

        assert_eq!(result.language, "rust");
        assert!(result.definitions.iter().any(|d| d.name == "hello" && d.kind == "function"));
        assert!(result.definitions.iter().any(|d| d.name == "load_data" && d.kind == "function"));
        assert!(result.definitions.iter().any(|d| d.name == "Config" && d.kind == "struct"));
        assert!(result.definitions.iter().any(|d| d.name == "Handler" && d.kind == "trait"));
        assert!(result.imports.iter().any(|i| i.source.contains("std::collections")));
        assert!(result.imports.iter().any(|i| i.source.contains("serde")));
    }
}
