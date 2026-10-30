//! Schema loading and $ref resolution within _shared/schemas/.
//!
//! Mirrors the Node oracle's `loadSchema` candidate order exactly:
//! `[name, name.json, name.schema.json]`, first existing file wins,
//! error only when none exists. Project-root discovery lives in the
//! CLI crate (`validate-cli::cli::find_project_root`); this crate never
//! resolves the root itself.

use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SchemaDoc {
    pub doc: Value,
    pub dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct SchemaCache {
    cache: HashMap<String, SchemaDoc>,
    schemas_dir: PathBuf,
}

impl SchemaCache {
    pub fn new(schemas_dir: PathBuf) -> Result<Self, String> {
        let canonical = fs::canonicalize(&schemas_dir)
            .map_err(|_| format!("schemas directory not found: {}", schemas_dir.display()))?;
        if !canonical.exists() {
            return Err(format!(
                "schemas directory not found: {}",
                canonical.display()
            ));
        }
        Ok(Self {
            cache: HashMap::new(),
            schemas_dir: canonical,
        })
    }

    fn resolve_schema_path(&self, name: &str) -> Result<PathBuf, String> {
        // Deviation #2 (deterministic schema-not-found) plus the traversal
        // rejection rule: one stderr line, no panic.
        if name.contains("..") || name.starts_with('/') || name.contains("://") {
            return Err(format!("schema not found: {}", name));
        }
        // Same candidates and filter the oracle uses. A `?`-style early
        // return here would abort before trying `name.schema.json`, so we
        // probe every candidate and fail only after the last one misses.
        let candidates = [
            name.to_string(),
            format!("{}.json", name),
            format!("{}.schema.json", name),
        ];
        for candidate in candidates.iter().filter(|c| c.ends_with(".json")) {
            let path = self.schemas_dir.join(candidate);
            if path.exists() {
                if let Ok(canonical) = fs::canonicalize(&path) {
                    if canonical.starts_with(&self.schemas_dir) {
                        return Ok(canonical);
                    }
                }
                return Err(format!("schema not found: {}", name));
            }
        }
        Err(format!("schema not found: {}", name))
    }

    pub fn get(&mut self, name: &str) -> Result<&SchemaDoc, String> {
        if !self.cache.contains_key(name) {
            let path = self.resolve_schema_path(name)?;
            let doc: Value = fs::read_to_string(&path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .ok_or_else(|| format!("schema not valid JSON: {}", name))?;
            self.cache.insert(
                name.to_string(),
                SchemaDoc {
                    doc,
                    dir: self.schemas_dir.clone(),
                },
            );
        }
        Ok(self.cache.get(name).unwrap())
    }

    /// Resolve a `$ref` like "gate.schema.json#/definitions/gate".
    ///
    /// `depth` is threaded from the validator so cyclic refs abort
    /// deterministically instead of recursing until the process aborts.
    pub fn resolve_ref(&mut self, ref_str: &str, depth: usize) -> Result<Value, String> {
        const MAX_REF_DEPTH: usize = 50;
        if depth > MAX_REF_DEPTH {
            return Err(format!("maximum $ref depth exceeded: {}", ref_str));
        }

        let (file_part, pointer_part) = match ref_str.split_once('#') {
            Some((f, p)) => (f, Some(p)),
            None => (ref_str, None),
        };
        let doc = self.get(file_part)?.doc.clone();
        let Some(pointer) = pointer_part else {
            return Ok(doc);
        };
        let mut node = doc;
        // Oracle: pointer.split("/").filter(Boolean) with decodeURIComponent
        // per segment. Empty segments are skipped; ~0/~1 escapes are NOT
        // honored (the oracle does not honor them either).
        for segment in pointer.split('/').filter(|s| !s.is_empty()) {
            let decoded =
                percent_decode(segment).map_err(|_| format!("bad $ref pointer: {}", ref_str))?;
            match node.get(&decoded) {
                Some(child) => node = child.clone(),
                None => return Err(format!("bad $ref pointer: {}", ref_str)),
            }
        }
        Ok(node)
    }
}

/// Mirrors JS `decodeURIComponent` for JSON-pointer segments: decodes %XX
/// byte escapes, requires valid UTF-8 afterwards, passes everything else
/// through untouched.
fn percent_decode(s: &str) -> Result<String, ()> {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3).ok_or(())?;
            let digits: Result<Vec<u32>, _> = hex
                .iter()
                .map(|b| (*b as char).to_digit(16).ok_or(()))
                .collect();
            let digits = digits?;
            let byte = digits[0] * 16 + digits[1];
            out.push(byte as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| ())
}
