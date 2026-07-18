//! Understand Anything CLI — codebase analysis & knowledge graph engine
//!
//! Commands: scan, parse, build, json, help

use std::path::Path;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");
    let root_path = args.get(2).map(|s| Path::new(s));
    let output_path = args.get(3);

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

            let out = output_path
                .map(|s| s.to_string())
                .unwrap_or_else(|| ".understand-anything/knowledge-graph.json".to_string());
            let dir = Path::new(&out).parent().unwrap();
            std::fs::create_dir_all(dir)?;
            std::fs::write(&out, serde_json::to_string_pretty(&graph)?)?;
            println!("  Graph written to {}", out);
        }

        "json" => {
            let scan = ua_core::scanner::scan_project(root)?;
            println!("{}", serde_json::to_string_pretty(&scan)?);
        }

        _ => {
            println!("Understand Anything CLI v0.1.0");
            println!();
            println!("  ua scan [path]           Scan project structure");
            println!("  ua parse [path]          Parse code files");
            println!(
                "  ua build [path] [out]    Build knowledge graph (→ .ua/knowledge-graph.json)"
            );
            println!("  ua json [path]           Output scan result as JSON");
        }
    }
    Ok(())
}
