//! Understand Anything CLI — codebase analysis & knowledge graph engine
//!
//! Commands: scan, parse, build, json, help

use std::path::Path;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");
    let root_path = args.get(2).map(|s| Path::new(s));

    // Parse --format flag from any position
    let format = parse_format_flag(&args);

    let root = root_path.unwrap_or(Path::new("."));

    match cmd {
        "scan" => {
            let result = ua_core::scanner::scan_project(root)?;
            println!(
                "{} files | {:?}",
                result.total_files, result.estimated_complexity
            );
            for (lang, count) in &result.stats.by_language {
                println!("  {}: {} files", lang, count);
            }
        }

        "parse" => {
            let registry = ua_core::parser::ParserRegistry::default();
            let scan = ua_core::scanner::scan_project(root)?;
            for file in &scan.files {
                if file.file_category == ua_core::types::FileCategory::Code {
                    match registry.parse(&root.join(&file.path)) {
                        Ok(parsed) => println!(
                            "{}: {} defs, {} imports",
                            file.path,
                            parsed.definitions.len(),
                            parsed.imports.len()
                        ),
                        Err(_) => {}
                    }
                }
            }
        }

        "build" => {
            println!("[Phase 1/3] Scanning {} ...", root.display());
            let scan = ua_core::scanner::scan_project(root)?;

            println!("[Phase 2/3] Parsing source files ...");
            let registry = ua_core::parser::ParserRegistry::default();
            let mut parsed = Vec::new();
            for file in &scan.files {
                if file.file_category == ua_core::types::FileCategory::Code {
                    if let Ok(p) = registry.parse(&root.join(&file.path)) {
                        parsed.push(p);
                    }
                }
            }
            println!("  Parsed {} files", parsed.len());

            println!("[Phase 3/3] Building knowledge graph ...");
            let graph = ua_core::graph::build_graph(root, &scan, &parsed);
            println!(
                "  {} nodes, {} edges, {} layers, {} tour steps",
                graph.nodes.len(),
                graph.edges.len(),
                graph.layers.len(),
                graph.tour.len()
            );

            match format {
                "html" => {
                    let out = output_path_or_default(&args, ".understand-anything/report.html");
                    let html = ua_core::report::to_html(&graph);
                    let dir = Path::new(&out).parent().unwrap();
                    std::fs::create_dir_all(dir)?;
                    std::fs::write(&out, html)?;
                    println!("  HTML report written to {}", out);
                }
                "md" | "markdown" => {
                    let out = output_path_or_default(&args, ".understand-anything/report.md");
                    let md = ua_core::report::to_markdown(&graph);
                    let dir = Path::new(&out).parent().unwrap();
                    std::fs::create_dir_all(dir)?;
                    std::fs::write(&out, md)?;
                    println!("  Markdown report written to {}", out);
                }
                _ => {
                    // Default: JSON
                    let out =
                        output_path_or_default(&args, ".understand-anything/knowledge-graph.json");
                    let dir = Path::new(&out).parent().unwrap();
                    std::fs::create_dir_all(dir)?;
                    std::fs::write(&out, serde_json::to_string_pretty(&graph)?)?;
                    println!("  Graph written to {}", out);
                }
            }
        }

        "json" => {
            let scan = ua_core::scanner::scan_project(root)?;
            println!("{}", serde_json::to_string_pretty(&scan)?);
        }

        _ => {
            println!("Understand Anything CLI v0.1.0");
            println!();
            println!("  ua scan [path]                  Scan project structure");
            println!("  ua parse [path]                 Parse code files");
            println!("  ua build [path] [out]           Build knowledge graph");
            println!("  ua build --format html [path]   Build HTML report");
            println!("  ua build --format md [path]     Build Markdown report");
            println!("  ua build --format json [path]   Build JSON graph (default)");
            println!("  ua json [path]                  Output scan result as JSON");
        }
    }
    Ok(())
}

/// Find the --format flag value in args.
/// Returns "json" if not specified.
fn parse_format_flag(args: &[String]) -> &str {
    for i in 0..args.len() {
        if args[i] == "--format" {
            if let Some(val) = args.get(i + 1) {
                return val.as_str();
            }
        }
        // Also support --format=value
        if let Some(val) = args[i].strip_prefix("--format=") {
            return val;
        }
    }
    "json"
}

/// Get output path: use the first positional arg after root (that isn't a flag),
/// or fall back to the default.
fn output_path_or_default(args: &[String], default: &str) -> String {
    // args[1] = command, args[2] = root (optional), args[3+] = output or flags
    let mut found_root = false;
    for i in 1..args.len() {
        if args[i].starts_with("--") {
            continue;
        }
        if !found_root {
            found_root = true; // skip the root path
            continue;
        }
        // This is the output path
        return args[i].clone();
    }
    default.to_string()
}
