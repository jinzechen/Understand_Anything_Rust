//! Understand Anything CLI — codebase analysis & knowledge graph engine
//!
//! Commands: scan, parse, build, json, help
//!
//! The `build` command supports incremental updates via `--incremental`:
//!   ua build --incremental          Only re-analyze changed files since last run
//!   ua build --full                 Force full re-analysis (the default)

use std::path::Path;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    // Parse flags
    let flags = parse_flags(&args);

    // Find root path (first positional arg after command that isn't a flag)
    let root = args
        .iter()
        .skip(2)
        .find(|a| !a.starts_with("--"))
        .map(|p| Path::new(p).to_path_buf())
        .unwrap_or_else(|| Path::new(".").to_path_buf());

    match cmd {
        "scan" => {
            let result = ua_core::scanner::scan_project(&root)?;
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
            let scan = ua_core::scanner::scan_project(&root)?;
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

        "build" => cmd_build(&root, &flags, &args)?,

        "json" => {
            let scan = ua_core::scanner::scan_project(&root)?;
            println!("{}", serde_json::to_string_pretty(&scan)?);
        }

        _ => {
            println!("Understand Anything CLI v0.2.0");
            println!();
            println!("  ua scan [path]                       Scan project structure");
            println!("  ua parse [path]                      Parse code files");
            println!("  ua build [path] [out]                Build knowledge graph");
            println!(
                "  ua build --incremental [path]        Incremental: only re-analyze changed files"
            );
            println!("  ua build --full [path]               Full re-analysis (default)");
            println!("  ua build --format html [path]        Build HTML dashboard report");
            println!("  ua build --format md [path]          Build Markdown report");
            println!("  ua build --format json [path]        Build JSON graph (default)");
            println!("  ua json [path]                       Output scan result as JSON");
        }
    }
    Ok(())
}

// ── Build command ─────────────────────────────────────────────────────────────

fn cmd_build(root: &Path, flags: &Flags, args: &[String]) -> anyhow::Result<()> {
    let format = parse_format_flag(args);

    if flags.incremental && !flags.full {
        // ── Incremental path ────────────────────────────────────────────
        build_incremental(root, format)
    } else {
        // ── Full build path ─────────────────────────────────────────────
        build_full(root, format)
    }
}

/// Full analysis: scan everything, parse everything, build the graph, save
/// fingerprints for future incremental runs.
fn build_full(root: &Path, format: &str) -> anyhow::Result<()> {
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

    // Save fingerprints for future incremental runs
    println!("[Fingerprint] Computing file hashes ...");
    let fingerprints = ua_core::incremental::compute_fingerprints(root, &scan.files)?;
    let meta = ua_core::incremental::MetaFile {
        git_commit_hash: graph.project.git_commit_hash.clone(),
        fingerprints,
        analyzed_at: graph.project.analyzed_at.clone(),
        version: "0.2.0".to_string(),
    };
    ua_core::incremental::write_meta(root, &meta)?;
    println!(
        "  Saved fingerprints for {} files to .understand-anything/meta.json",
        scan.files.len()
    );

    write_output(root, &graph, format, args_for_output_path().as_deref());
    Ok(())
}

/// Incremental build: read old fingerprints, find changed files, only re-parse
/// those, merge with existing graph.
fn build_incremental(root: &Path, format: &str) -> anyhow::Result<()> {
    // Read previous meta
    let old_meta = match ua_core::incremental::read_meta(root)? {
        Some(m) => m,
        None => {
            println!("[Incremental] No previous analysis found. Falling back to full build.");
            return build_full(root, format);
        }
    };

    println!(
        "[Incremental] Previous analysis: {} ({} files fingerprinted)",
        old_meta.analyzed_at,
        old_meta.fingerprints.len()
    );

    // Phase 1: Scan
    println!("[Phase 1/3] Scanning {} ...", root.display());
    let scan = ua_core::scanner::scan_project(root)?;
    println!("  Found {} files", scan.total_files);

    // Phase 1b: Compare fingerprints
    println!("[Phase 1b] Computing current fingerprints ...");
    let new_fingerprints = ua_core::incremental::compute_fingerprints(root, &scan.files)?;

    let changed_files =
        ua_core::incremental::find_changed_files(&old_meta.fingerprints, &new_fingerprints);

    if changed_files.is_empty() {
        println!("  No files changed since last analysis. Everything is up to date.");
        // Still write output from any cached graph, but we don't have a cache
        // for the full graph yet. For now, just report and exit.
        println!("  (Full graph caching not yet implemented — re-run with --full to rebuild.)");
        return Ok(());
    }

    println!(
        "  {} files changed since last analysis",
        changed_files.len()
    );
    for f in &changed_files {
        println!("    - {}", f);
    }

    // Phase 2+3: Rebuild graph (parse all files for correct topology).
    // The fingerprint check above confirmed what changed, and for correctness
    // we still need to parse all files to build the complete graph.
    // Future optimization: only parse changed files and merge nodes/edges
    // into the existing graph from a cached result.
    println!("[Phase 2/3] Re-parsing all files ...");
    let registry = ua_core::parser::ParserRegistry::default();
    let mut all_parsed = Vec::new();
    for file in &scan.files {
        if file.file_category == ua_core::types::FileCategory::Code {
            if let Ok(p) = registry.parse(&root.join(&file.path)) {
                all_parsed.push(p);
            }
        }
    }
    let graph = ua_core::graph::build_graph(root, &scan, &all_parsed);
    println!(
        "  {} nodes, {} edges, {} layers, {} tour steps",
        graph.nodes.len(),
        graph.edges.len(),
        graph.layers.len(),
        graph.tour.len()
    );

    // Save updated fingerprints
    println!("[Fingerprint] Updating meta ...");
    let meta = ua_core::incremental::MetaFile {
        git_commit_hash: graph.project.git_commit_hash.clone(),
        fingerprints: new_fingerprints,
        analyzed_at: graph.project.analyzed_at.clone(),
        version: "0.2.0".to_string(),
    };
    ua_core::incremental::write_meta(root, &meta)?;
    println!("  Saved fingerprints to .understand-anything/meta.json");

    write_output(root, &graph, format, args_for_output_path().as_deref());
    Ok(())
}

fn write_output(
    _root: &Path,
    graph: &ua_core::types::KnowledgeGraph,
    format: &str,
    output_arg: Option<&str>,
) -> anyhow::Result<()> {
    match format {
        "html" => {
            let out = output_arg.unwrap_or(".understand-anything/report.html");
            let html = ua_core::report::to_html(graph);
            let dir = Path::new(out).parent().unwrap();
            std::fs::create_dir_all(dir)?;
            std::fs::write(out, html)?;
            println!("  HTML report written to {}", out);
        }
        "md" | "markdown" => {
            let out = output_arg.unwrap_or(".understand-anything/report.md");
            let md = ua_core::report::to_markdown(graph);
            let dir = Path::new(out).parent().unwrap();
            std::fs::create_dir_all(dir)?;
            std::fs::write(out, md)?;
            println!("  Markdown report written to {}", out);
        }
        _ => {
            // Default: JSON
            let out = output_arg.unwrap_or(".understand-anything/knowledge-graph.json");
            let dir = Path::new(out).parent().unwrap();
            std::fs::create_dir_all(dir)?;
            std::fs::write(out, serde_json::to_string_pretty(graph)?)?;
            println!("  Graph written to {}", out);
        }
    }
    Ok(())
}

// ── Flag parsing ──────────────────────────────────────────────────────────────

struct Flags {
    incremental: bool,
    full: bool,
}

fn parse_flags(args: &[String]) -> Flags {
    let mut incremental = false;
    let mut full = false;

    for arg in args {
        if arg == "--incremental" {
            incremental = true;
        }
        if arg == "--full" {
            full = true;
        }
    }

    Flags { incremental, full }
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

/// Extract the output path from args (positional after root, not a flag or flag value).
fn args_for_output_path() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let mut found_root = false;
    let mut skip_next = false;
    for i in 1..args.len() {
        if args[i].starts_with("--") {
            if !args[i].contains('=') {
                skip_next = true; // --format html → skip "html"
            }
            continue;
        }
        if skip_next {
            skip_next = false;
            continue;
        }
        if !found_root {
            found_root = true;
            continue;
        }
        return Some(args[i].clone());
    }
    None
}
