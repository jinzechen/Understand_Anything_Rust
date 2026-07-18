//! Project scanner — file enumeration, language detection, and complexity estimation.
//!
//! Equivalent to Understand-Anything's `scan-project.mjs`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::types::{Complexity, FileCategory, ScanEntry, ScanResult, ScanStats};

/// Language detection by file extension.
fn detect_language(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") | Some("mjs") | Some("cjs") => "javascript",
        Some("py") | Some("pyi") | Some("pyx") => "python",
        Some("go") => "go",
        Some("java") => "java",
        Some("kt") | Some("kts") => "kotlin",
        Some("swift") => "swift",
        Some("c") | Some("h") => "c",
        Some("cpp") | Some("cc") | Some("cxx") | Some("hpp") | Some("hxx") => "cpp",
        Some("cs") => "csharp",
        Some("rb") => "ruby",
        Some("php") => "php",
        Some("lua") => "lua",
        Some("sql") => "sql",
        Some("sh") | Some("bash") | Some("zsh") => "shell",
        Some("ps1") | Some("psm1") | Some("psd1") => "powershell",
        Some("md") | Some("mdx") | Some("rst") => "markdown",
        Some("json") => "json",
        Some("yaml") | Some("yml") => "yaml",
        Some("toml") => "toml",
        Some("xml") | Some("svg") | Some("html") | Some("htm") => "xml",
        Some("css") | Some("scss") | Some("sass") | Some("less") => "css",
        Some("dockerfile") | Some("dockerignore") => "dockerfile",
        Some("tf") | Some("tfvars") => "terraform",
        Some("proto") => "protobuf",
        Some("graphql") | Some("gql") => "graphql",
        Some("env") | Some("envrc") => "env",
        Some("makefile") | Some("mk") => "makefile",
        _ => "unknown",
    }
}

/// Classify a file by its category (code, config, docs, infra, etc.).
fn classify_file(path: &Path, language: &str) -> FileCategory {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Infrastructure files
    if name == "dockerfile"
        || name == "docker-compose.yml"
        || name == "docker-compose.yaml"
        || path
            .to_str()
            .map(|p| p.contains(".github/workflows"))
            .unwrap_or(false)
        || path
            .to_str()
            .map(|p| p.contains("Jenkinsfile"))
            .unwrap_or(false)
    {
        return FileCategory::Infra;
    }

    // Config files
    if matches!(language, "json" | "yaml" | "toml" | "xml" | "env")
        || name == "makefile"
        || name == ".env"
        || name == ".envrc"
        || name.starts_with(".env.")
        || name.ends_with("rc")
        || name.ends_with(".conf")
    {
        return FileCategory::Config;
    }

    // Documentation
    if matches!(language, "markdown")
        || name == "readme.md"
        || name == "changelog.md"
        || name == "contributing.md"
        || name == "license"
        || name == "licence"
    {
        return FileCategory::Docs;
    }

    // Test files
    if name.contains("test")
        || name.contains("spec")
        || path
            .to_str()
            .map(|p| p.contains("/tests/") || p.contains("/__tests__/") || p.contains("/test/"))
            .unwrap_or(false)
    {
        return FileCategory::Test;
    }

    // Data files
    if matches!(language, "csv")
        || path
            .extension()
            .map(|e| e == "csv" || e == "tsv" || e == "jsonl")
            .unwrap_or(false)
    {
        return FileCategory::Data;
    }

    // Shell scripts
    if matches!(language, "shell") {
        return FileCategory::Script;
    }

    // Code (default for recognized programming languages)
    let code_languages = [
        "rust",
        "typescript",
        "javascript",
        "python",
        "go",
        "java",
        "kotlin",
        "swift",
        "c",
        "cpp",
        "csharp",
        "ruby",
        "php",
        "lua",
        "sql",
        "powershell",
        "terraform",
        "protobuf",
        "graphql",
        "dockerfile",
        "css",
        "makefile",
    ];
    if code_languages.contains(&language) {
        return FileCategory::Code;
    }

    FileCategory::Unknown
}

/// Estimate project complexity based on file count and total lines.
fn estimate_complexity(total_files: usize, total_lines: usize) -> Complexity {
    if total_files < 50 && total_lines < 10_000 {
        Complexity::Simple
    } else if total_files < 200 && total_lines < 100_000 {
        Complexity::Moderate
    } else {
        Complexity::Complex
    }
}

/// Count lines in a file.
fn count_lines(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

/// Scan a project directory and return a structured inventory.
pub fn scan_project(root: &Path) -> anyhow::Result<ScanResult> {
    let mut files: Vec<ScanEntry> = Vec::new();
    let mut total_lines = 0usize;
    let mut by_category: HashMap<String, usize> = HashMap::new();
    let mut by_language: HashMap<String, usize> = HashMap::new();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();

        // Skip hidden files and .git directories
        if path
            .to_str()
            .map(|p| p.contains("/.git/") || p.contains("\\.git\\"))
            .unwrap_or(false)
        {
            continue;
        }
        if path
            .file_name()
            .map(|n| n.to_str().map(|s| s.starts_with('.')).unwrap_or(false))
            .unwrap_or(false)
        {
            continue;
        }
        // Skip target/ and node_modules/
        if path
            .to_str()
            .map(|p| p.contains("/target/") || p.contains("/node_modules/"))
            .unwrap_or(false)
        {
            continue;
        }

        let rel_path = path.strip_prefix(root).unwrap_or(path);
        let language = detect_language(path);
        let size_lines = count_lines(path);
        let category = classify_file(path, language);

        total_lines += size_lines;

        *by_category
            .entry(format!("{:?}", category).to_lowercase())
            .or_default() += 1;
        *by_language.entry(language.to_string()).or_default() += 1;

        files.push(ScanEntry {
            path: rel_path.to_string_lossy().replace('\\', "/"),
            language: language.to_string(),
            size_lines,
            file_category: category,
        });
    }

    let total_files = files.len();
    let complexity = estimate_complexity(total_files, total_lines);

    Ok(ScanResult {
        files,
        total_files,
        filtered_by_ignore: 0,
        estimated_complexity: complexity,
        stats: ScanStats {
            files_scanned: total_files,
            by_category,
            by_language,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        assert_eq!(detect_language(Path::new("main.rs")), "rust");
        assert_eq!(detect_language(Path::new("app.ts")), "typescript");
        assert_eq!(detect_language(Path::new("script.py")), "python");
        assert_eq!(detect_language(Path::new("README.md")), "markdown");
        assert_eq!(detect_language(Path::new("config.json")), "json");
        assert_eq!(detect_language(Path::new("unknown.xyz")), "unknown");
    }

    #[test]
    fn test_file_classification() {
        assert_eq!(
            classify_file(Path::new("src/main.rs"), "rust"),
            FileCategory::Code
        );
        assert_eq!(
            classify_file(Path::new("README.md"), "markdown"),
            FileCategory::Docs
        );
        assert_eq!(
            classify_file(Path::new("Cargo.toml"), "toml"),
            FileCategory::Config
        );
        assert_eq!(
            classify_file(Path::new("Dockerfile"), "dockerfile"),
            FileCategory::Infra
        );
        assert_eq!(
            classify_file(Path::new("deploy.sh"), "shell"),
            FileCategory::Script
        );
    }
}
