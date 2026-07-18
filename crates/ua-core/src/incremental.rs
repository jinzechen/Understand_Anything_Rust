//! Incremental update support via file fingerprinting.
//!
//! Computes blake3 hashes for each scanned file and persists them alongside
//! metadata in `.understand-anything/meta.json`. On subsequent runs, only
//! files whose fingerprints have changed are re-parsed, and their nodes/edges
//! are merged into the existing graph.
//!
//! ## Usage
//! ```rust,ignore
//! use ua_core::incremental;
//!
//! // First run — full scan
//! let fps = incremental::compute_fingerprints(root, &scan.files)?;
//! let meta = incremental::MetaFile {
//!     git_commit_hash: "...".into(),
//!     fingerprints: fps,
//!     analyzed_at: "2024-01-01T00:00:00Z".into(),
//!     version: "0.2.0".into(),
//! };
//! incremental::write_meta(root, &meta)?;
//!
//! // Subsequent run — incremental
//! let old_meta = incremental::read_meta(root).unwrap();
//! let new_fps = incremental::compute_fingerprints(root, &scan.files)?;
//! let changed = incremental::find_changed_files(&old_meta.fingerprints, &new_fps);
//! ```

use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::types::ScanEntry;

// ── MetaFile ──────────────────────────────────────────────────────────────────

/// Persistent metadata stored in `.understand-anything/meta.json`.
///
/// Captures the state of the project at the time of the last analysis so that
/// subsequent runs can skip unchanged files (incremental mode).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaFile {
    /// Git commit hash from HEAD at analysis time (or "unknown").
    pub git_commit_hash: String,

    /// Map of relative file path → blake3 hex-encoded hash.
    pub fingerprints: HashMap<String, String>,

    /// ISO 8601 timestamp of the analysis (e.g., "2024-07-18T12:34:56Z").
    pub analyzed_at: String,

    /// Version of the meta file schema (bumps when format changes).
    pub version: String,
}

/// Default path where the meta file lives inside the project root.
pub const META_DIR: &str = ".understand-anything";
pub const META_FILE: &str = "meta.json";

// ── Fingerprinting ────────────────────────────────────────────────────────────

/// Compute a blake3 fingerprint for every file in the scan list.
///
/// Each fingerprint is the hex-encoded blake3 hash of the file's complete
/// contents. Paths use forward-slash separators and are relative to
/// `project_root` (matching the `ScanEntry.path` convention).
///
/// # Errors
/// Returns an error if any file cannot be read.
pub fn compute_fingerprints(
    project_root: &Path,
    files: &[ScanEntry],
) -> anyhow::Result<HashMap<String, String>> {
    let mut fingerprints = HashMap::with_capacity(files.len());

    for entry in files {
        let abs_path = project_root.join(&entry.path);
        let data = std::fs::read(&abs_path)
            .with_context(|| format!("Failed to read {} for fingerprinting", entry.path))?;
        let hash = blake3::hash(&data);
        fingerprints.insert(entry.path.clone(), hash.to_hex().to_string());
    }

    Ok(fingerprints)
}

// ── Meta I/O ───────────────────────────────────────────────────────────────────

/// Read the existing meta file from a project root.
///
/// Returns `None` if the file doesn't exist (first run).
/// Returns an error if the file exists but is malformed.
pub fn read_meta(project_root: &Path) -> anyhow::Result<Option<MetaFile>> {
    let meta_path = project_root.join(META_DIR).join(META_FILE);

    if !meta_path.exists() {
        return Ok(None);
    }

    let data = std::fs::read_to_string(&meta_path)
        .with_context(|| format!("Failed to read meta file at {}", meta_path.display()))?;

    let meta: MetaFile = serde_json::from_str(&data)
        .with_context(|| format!("Failed to parse meta file at {}", meta_path.display()))?;

    Ok(Some(meta))
}

/// Write meta data to `.understand-anything/meta.json`.
///
/// Creates the `.understand-anything` directory if it doesn't exist.
pub fn write_meta(project_root: &Path, meta: &MetaFile) -> anyhow::Result<()> {
    let dir = project_root.join(META_DIR);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create directory {}", dir.display()))?;

    let meta_path = dir.join(META_FILE);
    let json = serde_json::to_string_pretty(meta)?;
    std::fs::write(&meta_path, json)
        .with_context(|| format!("Failed to write meta file to {}", meta_path.display()))?;

    Ok(())
}

// ── Change Detection ──────────────────────────────────────────────────────────

/// Compare two fingerprint maps and return the list of files that have changed.
///
/// A file is considered "changed" if:
/// - It exists in `new_fingerprints` but not in `old_fingerprints` (new file).
/// - It exists in both but has a different hash (modified file).
///
/// Files present only in `old_fingerprints` are **not** included as "changed"
/// (they were deleted; the caller can detect them by absence from `new_fingerprints`).
pub fn find_changed_files(
    old_fingerprints: &HashMap<String, String>,
    new_fingerprints: &HashMap<String, String>,
) -> Vec<String> {
    let mut changed = Vec::new();

    for (path, new_hash) in new_fingerprints {
        match old_fingerprints.get(path) {
            Some(old_hash) if old_hash == new_hash => {
                // Unchanged — skip
            }
            _ => {
                // New file or hash differs
                changed.push(path.clone());
            }
        }
    }

    changed
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FileCategory, ScanEntry};

    fn make_entry(path: &str) -> ScanEntry {
        ScanEntry {
            path: path.to_string(),
            language: "rust".to_string(),
            size_lines: 10,
            file_category: FileCategory::Code,
        }
    }

    #[test]
    fn test_fingerprint_computation() {
        // Use a temp dir with known content
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("hello.rs");
        std::fs::write(&file_path, b"fn main() {}").unwrap();

        let entries = vec![ScanEntry {
            path: "hello.rs".to_string(),
            language: "rust".to_string(),
            size_lines: 1,
            file_category: FileCategory::Code,
        }];

        let fps = compute_fingerprints(tmp.path(), &entries).unwrap();
        assert!(fps.contains_key("hello.rs"));

        // Same content → same hash
        let fps2 = compute_fingerprints(tmp.path(), &entries).unwrap();
        assert_eq!(fps.get("hello.rs"), fps2.get("hello.rs"));
    }

    #[test]
    fn test_find_changed_files() {
        let mut old: HashMap<String, String> = HashMap::new();
        old.insert("a.rs".to_string(), "hash_a".to_string());
        old.insert("b.rs".to_string(), "hash_b".to_string());

        let mut new: HashMap<String, String> = HashMap::new();
        new.insert("a.rs".to_string(), "hash_a".to_string()); // unchanged
        new.insert("b.rs".to_string(), "hash_b_modified".to_string()); // changed
        new.insert("c.rs".to_string(), "hash_c".to_string()); // new file

        let changed = find_changed_files(&old, &new);
        assert_eq!(changed.len(), 2);
        assert!(changed.contains(&"b.rs".to_string()));
        assert!(changed.contains(&"c.rs".to_string()));
        assert!(!changed.contains(&"a.rs".to_string()));
    }

    #[test]
    fn test_find_changed_files_empty_old() {
        let old: HashMap<String, String> = HashMap::new();
        let mut new: HashMap<String, String> = HashMap::new();
        new.insert("a.rs".to_string(), "hash_a".to_string());

        let changed = find_changed_files(&old, &new);
        assert_eq!(changed.len(), 1);
        assert!(changed.contains(&"a.rs".to_string()));
    }

    #[test]
    fn test_meta_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Should return None on first read
        assert!(read_meta(root).unwrap().is_none());

        // Write
        let meta = MetaFile {
            git_commit_hash: "abc123".to_string(),
            fingerprints: {
                let mut m = HashMap::new();
                m.insert("main.rs".to_string(), "deadbeef".to_string());
                m
            },
            analyzed_at: "2024-07-18T12:00:00Z".to_string(),
            version: "0.2.0".to_string(),
        };
        write_meta(root, &meta).unwrap();

        // Read back
        let read = read_meta(root).unwrap().unwrap();
        assert_eq!(read.git_commit_hash, "abc123");
        assert_eq!(read.version, "0.2.0");
        assert_eq!(read.fingerprints.get("main.rs").unwrap(), "deadbeef");
    }

    #[test]
    fn test_fingerprint_determinism() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("data.txt");
        std::fs::write(&file_path, b"hello world").unwrap();

        let entries = vec![make_entry("data.txt")];

        let fp1 = compute_fingerprints(tmp.path(), &entries).unwrap();
        let fp2 = compute_fingerprints(tmp.path(), &entries).unwrap();

        // Same content should always produce the same hash
        assert_eq!(fp1.get("data.txt"), fp2.get("data.txt"));
    }
}
