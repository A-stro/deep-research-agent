//! Schema loading and $ref resolution within _shared/schemas/.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use serde_json::Value;

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
        if !schemas_dir.exists() {
            return Err(format!("schemas directory not found: {}", schemas_dir.display()));
        }
        Ok(Self {
            cache: HashMap::new(),
            schemas_dir,
        })
    }

    fn resolve_schema_path(&self, name: &str) -> Result<PathBuf, String> {
        let candidates = [
            name.to_string(),
            format!("{}.json", name),
            format!("{}.schema.json", name),
        ];
        for c in &candidates {
            if c.ends_with(".json") {
                let p = self.schemas_dir.join(c);
                if p.exists() {
                    return Ok(p);
                }
            }
        }
        Err(format!("schema not found: {}", candidates.join(", ")))
    }

    pub fn get(&mut self, name: &str) -> Result<&SchemaDoc, String> {
        if self.cache.contains_key(name) {
            return Ok(self.cache.get(name).unwrap());
        }
        let path = self.resolve_schema_path(name)?;
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("failed to read schema {}: {}", path.display(), e))?;
        let doc: Value = serde_json::from_str(&content)
            .map_err(|e| format!("schema {} is not valid JSON: {}", path.display(), e))?;
        let schema_doc = SchemaDoc {
            doc,
            dir: self.schemas_dir.clone(),
        };
        self.cache.insert(name.to_string(), schema_doc);
        Ok(self.cache.get(name).unwrap())
    }

    /// Resolve a $ref like "gate.schema.json#/definitions/gate"
    pub fn resolve_ref(&mut self, ref_str: &str) -> Result<Value, String> {
        let (file_part, pointer_part) = match ref_str.split_once('#') {
            Some((f, p)) => (f, Some(p)),
            None => (ref_str, None),
        };
        let schema_doc = self.get(file_part)?;
        let mut node = &schema_doc.doc;
        if let Some(pointer) = pointer_part {
            for segment in pointer.split('/').skip(1) {
                let decoded = percent_decode(segment);
                node = node.get(&decoded).ok_or_else(|| {
                    format!("bad $ref pointer: {} (segment '{}' not found)", ref_str, segment)
                })?;
            }
        }
        Ok(node.clone())
    }
}

fn percent_decode(s: &str) -> String {
    s.replace("~1", "/").replace("~0", "~")
}

/// Find the project root by walking up from cwd.
/// Looks for a directory containing both _shared/schemas/ and stages/.
pub fn find_project_root() -> Result<PathBuf, String> {
    if let Ok(dra_root) = std::env::var("DRA_ROOT") {
        let p = PathBuf::from(dra_root);
        if p.join("_shared/schemas").exists() && p.join("stages").exists() {
            return Ok(p);
        }
        return Err(format!("DRA_ROOT set but invalid: {}", p.display()));
    }

    let cwd = std::env::current_dir().map_err(|e| format!("failed to get cwd: {}", e))?;
    let mut current = Some(cwd.as_path());
    while let Some(dir) = current {
        if dir.join("_shared/schemas").exists() && dir.join("stages").exists() {
            return Ok(dir.to_path_buf());
        }
        current = dir.parent();
    }
    Err("project root not found: run from the repository root or set DRA_ROOT".to_string())
}