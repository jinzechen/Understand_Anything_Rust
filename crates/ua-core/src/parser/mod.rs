//! Code parser trait and Rust parser implementation.
//! Extracts structural information from source code using regex-based analysis.

use std::path::Path;
use regex::Regex;

use crate::types::{DefinitionInfo, EndpointInfo, ImportInfo, SectionInfo, ServiceInfo, StepInfo};

/// Result of parsing a single source file.
#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub path: String,
    pub language: String,
    pub line_count: usize,
    pub definitions: Vec<DefinitionInfo>,
    pub imports: Vec<ImportInfo>,
    pub sections: Vec<SectionInfo>,
    pub services: Vec<ServiceInfo>,
    pub endpoints: Vec<EndpointInfo>,
    pub steps: Vec<StepInfo>,
}

/// Trait for language-specific code parsers.
pub trait CodeParser: Send + Sync {
    fn language(&self) -> &str;
    fn extensions(&self) -> &[&str];
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

    pub fn find_for(&self, path: &Path) -> Option<&dyn CodeParser> {
        let ext = path.extension()?.to_str()?;
        self.parsers
            .iter()
            .find(|p| p.extensions().contains(&ext))
            .map(|p| p.as_ref())
    }

    pub fn parse(&self, path: &Path) -> anyhow::Result<ParsedFile> {
        let parser = self.find_for(path)
            .ok_or_else(|| anyhow::anyhow!("No parser for: {}", path.display()))?;
        let content = std::fs::read_to_string(path)?;
        parser.parse_file(path, &content)
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(RustParser::new()));
        registry
    }
}

// ── Rust Parser ──────────────────────────────────────────────────────────────

pub struct RustParser {
    fn_re: Regex,
    struct_re: Regex,
    trait_re: Regex,
    enum_re: Regex,
    impl_re: Regex,
    use_re: Regex,
}

impl RustParser {
    pub fn new() -> Self {
        Self {
            fn_re: Regex::new(r"(?m)^\s*(?:pub(?:\s*\(\s*[^)]*\s*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+(\w+)").unwrap(),
            struct_re: Regex::new(r"(?m)^\s*(?:pub(?:\s*\(\s*[^)]*\s*\))?\s+)?struct\s+(\w+)").unwrap(),
            trait_re: Regex::new(r"(?m)^\s*(?:pub(?:\s*\(\s*[^)]*\s*\))?\s+)?(?:unsafe\s+)?trait\s+(\w+)").unwrap(),
            enum_re: Regex::new(r"(?m)^\s*(?:pub(?:\s*\(\s*[^)]*\s*\))?\s+)?enum\s+(\w+)").unwrap(),
            impl_re: Regex::new(r"(?m)^\s*(?:pub(?:\s*\(\s*[^)]*\s*\))?\s+)?(?:unsafe\s+)?impl(?:\s*<[^>]*>\s*)?\s+(\S+)").unwrap(),
            use_re: Regex::new(r"(?m)^\s*use\s+([^;]+);").unwrap(),
        }
    }

    fn line_of(&self, content: &str, pos: usize) -> u32 {
        content[..pos].chars().filter(|&c| c == '\n').count() as u32 + 1
    }
}

impl CodeParser for RustParser {
    fn language(&self) -> &str { "rust" }
    fn extensions(&self) -> &[&str] { &["rs"] }

    fn parse_file(&self, path: &Path, content: &str) -> anyhow::Result<ParsedFile> {
        let line_count = content.lines().count();
        let mut definitions = Vec::new();
        let mut imports = Vec::new();

        for cap in self.fn_re.captures_iter(content) {
            let name = cap[1].to_string();
            if name != "main" {
                let line = self.line_of(content, cap.get(0).unwrap().start());
                definitions.push(DefinitionInfo {
                    name, kind: "function".into(),
                    line_range: (line, line), fields: vec![],
                });
            }
        }

        for cap in self.struct_re.captures_iter(content) {
            let line = self.line_of(content, cap.get(0).unwrap().start());
            definitions.push(DefinitionInfo {
                name: cap[1].to_string(), kind: "struct".into(),
                line_range: (line, line), fields: vec![],
            });
        }

        for cap in self.trait_re.captures_iter(content) {
            let line = self.line_of(content, cap.get(0).unwrap().start());
            definitions.push(DefinitionInfo {
                name: cap[1].to_string(), kind: "trait".into(),
                line_range: (line, line), fields: vec![],
            });
        }

        for cap in self.enum_re.captures_iter(content) {
            let line = self.line_of(content, cap.get(0).unwrap().start());
            definitions.push(DefinitionInfo {
                name: cap[1].to_string(), kind: "enum".into(),
                line_range: (line, line), fields: vec![],
            });
        }

        for cap in self.impl_re.captures_iter(content) {
            let line = self.line_of(content, cap.get(0).unwrap().start());
            definitions.push(DefinitionInfo {
                name: format!("impl {}", &cap[1]), kind: "implementation".into(),
                line_range: (line, line), fields: vec![],
            });
        }

        for cap in self.use_re.captures_iter(content) {
            let source = cap[1].trim().to_string();
            let name = source.split("::").last().unwrap_or(&source).to_string();
            let line = self.line_of(content, cap.get(0).unwrap().start());
            imports.push(ImportInfo { name, source, line_range: (line, line) });
        }

        Ok(ParsedFile {
            path: path.to_string_lossy().replace('\\', "/"),
            language: "rust".into(),
            line_count,
            definitions,
            imports,
            sections: vec![],
            services: vec![],
            endpoints: vec![],
            steps: vec![],
        })
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RUST: &str = "\
pub fn hello() -> String {
    \"world\".into()
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
";

    #[test]
    fn test_rust_parser_functions() {
        let parser = RustParser::new();
        let result = parser.parse_file(Path::new("test.rs"), SAMPLE_RUST).unwrap();

        assert_eq!(result.language, "rust");
        let names: Vec<_> = result.definitions.iter().map(|d| &d.name).collect();
        assert!(names.contains(&&"hello".to_string()));
        assert!(names.contains(&&"load_data".to_string()));
        assert!(names.contains(&&"Config".to_string()));
        assert!(names.contains(&&"Handler".to_string()));
        assert!(names.contains(&&"impl Config".to_string()));

        let import_srcs: Vec<_> = result.imports.iter().map(|i| &i.source).collect();
        assert!(import_srcs.iter().any(|s| s.contains("std::collections")));
        assert!(import_srcs.iter().any(|s| s.contains("serde")));
    }
}
