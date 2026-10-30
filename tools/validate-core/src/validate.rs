//! Schema subset validator matching the Node oracle's supported keywords.
//!
//! Faithfulness notes (oracle `validate.mjs` lines 64–92):
//! - `validate` takes the path `at` as its seed; every message is prefixed
//!   `${at}: ` and nested paths append `.${key}` / `[${i}]`. One path model
//!   only — callers seed with the stage name (or the `file` path) and this
//!   module never adds a second prefix.
//! - Enum failures list raw values (`[ResearchPlan]`) and render `got`
//!   with `JSON.stringify` semantics.
//! - `minLength` counts UTF-16 code units (JS `String.length`).
//! - Order per node: enum, type, minLength, pattern, minItems/items,
//!   required, properties (in *schema declaration* order), unknown fields.
//!   Property traversal order requires serde_json `preserve_order`.
//! - Conditions the oracle leaves as uncaught JS throws (cyclic `$ref`,
//!   bad ref pointer, invalid pattern regex) surface as `Err` — one
//!   deterministic stderr line, exit 1 — instead of a panic.

use crate::schemas::SchemaCache;
use regex::Regex;
use serde_json::Value;

pub struct Validator {
    cache: SchemaCache,
    errors: Vec<String>,
    path: String,
    ref_depth: usize,
}

impl Validator {
    pub fn new(cache: SchemaCache) -> Self {
        Self {
            cache,
            errors: Vec::new(),
            path: String::new(),
            ref_depth: 0,
        }
    }

    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    pub fn into_errors(self) -> Vec<String> {
        self.errors
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Validate `value` against `schema`, seeding all message paths with
    /// `at`. Fatal conditions (schema refs that can't resolve) short-circuit
    /// the whole command like an uncaught throw in the oracle would.
    pub fn validate(&mut self, value: &Value, schema: &Value, at: &str) -> Result<bool, String> {
        self.path = at.to_string();
        self.ref_depth = 0;
        self.validate_inner(value, schema)?;
        Ok(self.errors.is_empty())
    }

    fn add_error(&mut self, msg: &str) {
        self.errors.push(format!("{}: {}", self.path, msg));
    }

    fn with_path<F>(&mut self, segment: &str, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut Validator) -> Result<(), String>,
    {
        let old = std::mem::take(&mut self.path);
        self.path = format!("{}.{}", old, segment);
        let result = f(self);
        self.path = old;
        result
    }

    fn with_index<F>(&mut self, index: usize, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut Validator) -> Result<(), String>,
    {
        let old = std::mem::take(&mut self.path);
        self.path = format!("{}[{}]", old, index);
        let result = f(self);
        self.path = old;
        result
    }

    fn validate_inner(&mut self, value: &Value, schema: &Value) -> Result<(), String> {
        if let Some(ref_str) = schema.get("$ref").and_then(|v| v.as_str()) {
            self.ref_depth += 1;
            let resolved = self.cache.resolve_ref(ref_str, self.ref_depth)?;
            self.validate_inner(value, &resolved)?;
            self.ref_depth -= 1;
            return Ok(());
        }

        // enum — before type (oracle order)
        if let Some(enum_vals) = schema.get("enum").and_then(|v| v.as_array()) {
            if !enum_vals.iter().any(|v| v == value) {
                let expected: Vec<String> = enum_vals
                    .iter()
                    .map(|v| match v {
                        // Oracle joins the raw values: strings go unquoted.
                        Value::String(s) => s.clone(),
                        other => serde_json::to_string(other).unwrap_or_default(),
                    })
                    .collect();
                self.add_error(&format!(
                    "expected one of [{}], got {}",
                    expected.join(", "),
                    serde_json::to_string(value).unwrap_or_default()
                ));
            }
        }

        if let Some(t) = schema.get("type").and_then(|v| v.as_str()) {
            self.check_type(value, t);
        }

        if let Some(s) = value.as_str() {
            if let Some(min_len) = schema.get("minLength").and_then(|v| v.as_u64()) {
                if js_utf16_len(s) < min_len {
                    self.add_error(&format!("shorter than minLength {}", min_len));
                }
            }
            if let Some(pattern) = schema.get("pattern").and_then(|v| v.as_str()) {
                match Regex::new(pattern) {
                    // Invalid regex would be an uncaught SyntaxError in the
                    // oracle; here it is one deterministic fatal line.
                    Err(_) => return Err(format!("invalid schema pattern: {}", pattern)),
                    Ok(re) if !re.is_match(s) => {
                        self.add_error(&format!(
                            "does not match pattern {} (got {})",
                            pattern,
                            serde_json::to_string(value).unwrap_or_default()
                        ));
                    }
                    Ok(_) => {}
                }
            }
        }

        if let Some(arr) = value.as_array() {
            if let Some(min_items) = schema.get("minItems").and_then(|v| v.as_u64()) {
                if (arr.len() as u64) < min_items {
                    self.add_error(&format!("fewer than {} item(s)", min_items));
                }
            }
            if let Some(items_schema) = schema.get("items") {
                for (i, item) in arr.iter().enumerate() {
                    self.with_index(i, |v| v.validate_inner(item, items_schema))?;
                }
            }
        }

        if let Some(obj) = value.as_object() {
            if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
                for field in required {
                    if let Some(name) = field.as_str() {
                        if !obj.contains_key(name) {
                            self.add_error(&format!("missing required field \"{}\"", name));
                        }
                    }
                }
            }
            if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
                // Oracle iterates schema.properties (declaration order),
                // validating only keys present in the value.
                for (key, prop_schema) in props.iter() {
                    if let Some(val) = obj.get(key) {
                        self.with_path(key, |v| v.validate_inner(val, prop_schema))?;
                    }
                }
                if schema.get("additionalProperties").and_then(|v| v.as_bool()) == Some(false) {
                    for key in obj.keys() {
                        if !props.contains_key(key) {
                            self.add_error(&format!("unknown field \"{}\"", key));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn check_type(&mut self, value: &Value, expected: &str) {
        // Wording mirrors the oracle's `typeof`-based rendering (validate.mjs:70)
        // exactly: a JS number prints "number" unless the expected type is
        // "integer" and the value is a JS integer, and `typeof null` is
        // "object". Acceptance stays NORMAL JSON semantics (ADR §2): null is
        // never accepted as an object, so wording and acceptance are computed
        // separately.
        let word = match value {
            Value::Array(_) => "array",
            Value::Number(n) => {
                if js_is_integer(n) && expected == "integer" {
                    "integer"
                } else {
                    "number"
                }
            }
            Value::Null => "object",
            Value::Bool(_) => "boolean",
            Value::String(_) => "string",
            Value::Object(_) => "object",
        };

        let type_ok = match expected {
            // Number.isInteger: 1.0 is an integer; serde must agree.
            "integer" => matches!(value, Value::Number(n) if js_is_integer(n)),
            "number" => value.is_number(),
            "string" => matches!(value, Value::String(_)),
            "boolean" => matches!(value, Value::Bool(_)),
            "array" => matches!(value, Value::Array(_)),
            "object" => matches!(value, Value::Object(_)),
            _ => word == expected,
        };

        if !type_ok {
            self.add_error(&format!("expected {}, got {}", expected, word));
        }
    }
}

/// JS `Number.isInteger` semantics on a serde number.
fn js_is_integer(n: &serde_json::Number) -> bool {
    n.as_f64().map(|f| f.fract() == 0.0).unwrap_or(false)
}

/// JS `String.length`: UTF-16 code units, so astral chars count as 2.
fn js_utf16_len(s: &str) -> u64 {
    s.chars().map(char::len_utf16).sum::<usize>() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn cache(dir: &TempDir) -> SchemaCache {
        SchemaCache::new(dir.path().to_path_buf()).unwrap()
    }

    fn validate_json(cache: &SchemaCache, value: Value, schema: Value) -> Vec<String> {
        let mut v = Validator::new(cache.clone());
        v.validate(&value, &schema, "doc").unwrap();
        v.errors().to_vec()
    }

    #[test]
    fn integer_accepts_integral_float() {
        let dir = TempDir::new().unwrap();
        let c = cache(&dir);
        let errs = validate_json(&c, json!(1.0), json!({"type": "integer"}));
        assert!(errs.is_empty(), "{errs:?}");
        let errs = validate_json(&c, json!(2), json!({"type": "integer"}));
        assert!(errs.is_empty(), "{errs:?}");
        let errs = validate_json(&c, json!(1.5), json!({"type": "integer"}));
        assert_eq!(errs, vec!["doc: expected integer, got number"]);
    }

    #[test]
    fn type_wording_matches_js_typeof() {
        let dir = TempDir::new().unwrap();
        let c = cache(&dir);
        // A JS number renders "number" against any other expected type.
        let errs = validate_json(&c, json!(1), json!({"type": "string"}));
        assert_eq!(errs, vec!["doc: expected string, got number"]);
        // typeof null is "object" in JS — wording only; null is still rejected.
        let errs = validate_json(&c, json!(null), json!({"type": "string"}));
        assert_eq!(errs, vec!["doc: expected string, got object"]);
        // null is NOT accepted as an object (normal JSON semantics, ADR §2).
        let errs = validate_json(&c, json!(null), json!({"type": "object"}));
        assert_eq!(errs, vec!["doc: expected object, got object"]);
        let errs = validate_json(&c, json!(true), json!({"type": "integer"}));
        assert_eq!(errs, vec!["doc: expected integer, got boolean"]);
    }

    #[test]
    fn min_length_counts_utf16_units() {
        let dir = TempDir::new().unwrap();
        let c = cache(&dir);
        let schema = json!({"minLength": 6});
        // 5 chars, each BMP → 5 UTF-16 units → fails.
        let errs = validate_json(&c, json!("héllo"), schema.clone());
        assert_eq!(errs, vec!["doc: shorter than minLength 6"]);
        // 3 chars + one astral char = 4 chars but 6 UTF-16 units → passes.
        let errs = validate_json(&c, json!("a𝕏cd\u{1F9CA}"), schema);
        assert!(errs.is_empty(), "{errs:?}");
    }

    #[test]
    fn enum_list_is_raw_joined() {
        let dir = TempDir::new().unwrap();
        let c = cache(&dir);
        let schema = json!({"enum": ["ResearchPlan"]});
        let errs = validate_json(&c, json!("Nope"), schema.clone());
        assert_eq!(
            errs,
            vec!["doc: expected one of [ResearchPlan], got \"Nope\""]
        );
        let errs = validate_json(&c, json!(3), schema);
        assert_eq!(errs, vec!["doc: expected one of [ResearchPlan], got 3"]);
    }

    #[test]
    fn property_traversal_is_in_declaration_order() {
        let dir = TempDir::new().unwrap();
        let c = cache(&dir);
        let schema = json!({
            "properties": {"run_id": {"type": "string"}, "gate": {"type": "string"}},
            "additionalProperties": false
        });
        let errs = validate_json(&c, json!({"gate": 1, "run_id": 2, "zz": 1}), schema);
        assert_eq!(
            errs,
            vec![
                "doc.run_id: expected string, got number",
                "doc.gate: expected string, got number",
                "doc: unknown field \"zz\"",
            ]
        );
    }

    #[test]
    fn cyclic_ref_errors_instead_of_aborting() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("cyclic.schema.json"),
            r#"{"$ref": "cyclic.schema.json#/definitions/loop", "definitions": {"loop": {"$ref": "cyclic.schema.json#/definitions/loop"}}}"#,
        )
        .unwrap();
        let mut c = cache(&dir);
        let schema = c.get("cyclic.schema.json").unwrap().doc.clone();
        let mut v = Validator::new(c.clone());
        let result = v.validate(&json!({}), &schema, "doc");
        match result {
            Ok(_) => panic!("cyclic ref must not validate"),
            Err(e) => assert!(
                e.starts_with("maximum $ref depth exceeded:"),
                "unexpected: {e}"
            ),
        }
    }

    #[test]
    fn bad_ref_pointer_is_fatal() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("gate.schema.json"),
            r#"{"definitions": {"gate": {"type": "object"}}}"#,
        )
        .unwrap();
        let c = cache(&dir);
        let schema = json!({"$ref": "gate.schema.json#/definitions/missing"});
        let mut v = Validator::new(c.clone());
        let result = v.validate(&json!({}), &schema, "doc");
        assert_eq!(
            result.err(),
            Some("bad $ref pointer: gate.schema.json#/definitions/missing".to_string())
        );
    }

    #[test]
    fn invalid_pattern_is_fatal_not_silent() {
        let dir = TempDir::new().unwrap();
        let c = cache(&dir);
        // `[` is invalid in the regex crate's syntax as written.
        let schema = json!({"pattern": "["});
        let mut v = Validator::new(c.clone());
        let result = v.validate(&json!("a"), &schema, "doc");
        assert_eq!(result.err(), Some("invalid schema pattern: [".to_string()));
    }

    #[test]
    fn short_schema_names_resolve_all_three_forms() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("plan.schema.json"), r#"{"type": "object"}"#).unwrap();
        let mut c = cache(&dir);
        assert!(c.get("plan.schema.json").is_ok());
        assert!(c.get("plan").is_ok());
        assert!(c.get("plan.schema").is_ok());
        assert_eq!(
            c.get("nope").err(),
            Some("schema not found: nope".to_string())
        );
    }
}
