//! Understand Anything CLI
//!
//! Commands: scan, parse, build

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");
    let path = args.get(2).map(|s| std::path::Path::new(s));

    match cmd {
        "scan" => {
            let root = path.unwrap_or(std::path::Path::new("."));
            let result = ua_core::scanner::scan_project(root)?;
            println!("{} files scanned ({} filtered)", result.total_files, result.filtered_by_ignore);
            println!("Complexity: {:?}", result.estimated_complexity);
            println!("Languages: {:?}", result.stats.by_language);
        }
        "parse" => {
            let root = path.unwrap_or(std::path::Path::new("."));
            let registry = ua_core::parser::ParserRegistry::default();
            let result = ua_core::scanner::scan_project(root)?;
            for file in &result.files {
                if file.file_category == ua_core::types::FileCategory::Code {
                    let full_path = root.join(&file.path);
                    match registry.parse(&full_path) {
                        Ok(parsed) => {
                            println!("{}: {} defs, {} imports", file.path, parsed.definitions.len(), parsed.imports.len());
                        }
                        Err(e) => {
                            eprintln!("{}: skip ({})", file.path, e);
                        }
                    }
                }
            }
        }
        _ => {
            println!("Understand Anything CLI v0.1.0");
            println!("  ua scan [path]   — scan project structure");
            println!("  ua parse [path]  — parse code files");
        }
    }
    Ok(())
}
