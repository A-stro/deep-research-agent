//! Schema subset validator matching the Node oracle's supported keywords.

use serde_json::Value;
use crate::schemas::SchemaCache;
use regex::Regex;

#[derive(Debug)]
pub struct Validator {
    cache: SchemaCache,
    errors: Vec<String>,
    notes: Vec<String>,
    path_prefix: String,
}

impl Validator {
    pub fn new(cache: SchemaCache) -> Self {
        Self {
            cache,
            errors: Vec::new(),
            notes: Vec::new(),
            path_prefix: String::new(),
        }
    }

    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn take_errors_and_notes(self) -> (Vec<String>, Vec<String>) {
        (self.errors, self.notes)
    }

    fn add_error(&mut self, msg: &str) {
        self.errors.push(format!("{}{}", self.path_prefix, msg));
    }

    fn add_note(&mut self, msg: &str) {
        self.notes.push(format!("{}{}", self.path_prefix, msg));
    }

    fn with_path<F>(&mut self, prefix: &str, f: F)
    where
        F: FnOnce(&mut Validator),
    {
        let old = std::mem::take(&mut self.path_prefix);
        self.path_prefix = format!("{}{}.", old, prefix);
        f(self);
        self.path_prefix = old;
    }

    fn with_index<F>(&mut self, index: usize, f: F)
    where
        F: FnOnce(&mut Validator),
    {
        let old = std::mem::take(&mut self.path_prefix);
        self.path_prefix = format!("{}[{}].", old, index);
        f(self);
        self.path_prefix = old;
    }

    /// Main validation entry point
    pub fn validate(&mut self, value: &Value, schema: &Value) -> bool {
        self.validate_inner(value, schema);
        self.errors.is_empty()
    }

    fn validate_inner(&mut self, value: &Value, schema: &Value) {
        // Handle $ref first
        if let Some(ref_str) = schema.get("$ref").and_then(|v| v.as_str()) {
            match self.cache.resolve_ref(ref_str) {
                Ok(resolved) => self.validate_inner(value, &resolved),
                Err(e) => self.add_error(&format!("failed to resolve $ref '{}': {}", ref_str, e)),
            }
            return;
        }

        // type
        if let Some(t) = schema.get("type").and_then(|v| v.as_str()) {
            self.check_type(value, t);
        }

        // enum
        if let Some(enum_vals) = schema.get("enum").and_then(|v| v.as_array()) {
            self.check_enum(value, enum_vals);
        }

        // For strings
        if value.is_string() {
            if let Some(min_len) = schema.get("minLength").and_then(|v| v.as_u64()) {
                if (value.as_str().unwrap().len() as u64) < min_len {
                    self.add_error(&format!("shorter than minLength {}", min_len));
                }
            }
            if let Some(pattern) = schema.get("pattern").and_then(|v| v.as_str()) {
                let re = Regex::new(pattern).unwrap_or_else(|_| Regex::new("").unwrap());
                if !re.is_match(value.as_str().unwrap()) {
                    self.add_error(&format!("does not match pattern {} (got {})", pattern, serde_json::to_string(value.as_str().unwrap()).unwrap()));
                }
            }
        }

        // For arrays
        if let Some(arr) = value.as_array() {
            if let Some(min_items) = schema.get("minItems").and_then(|v| v.as_u64()) {
                if (arr.len() as u64) < min_items {
                    self.add_error(&format!("fewer than {} item(s)", min_items));
                }
            }
            if let Some(items_schema) = schema.get("items") {
                for (i, item) in arr.iter().enumerate() {
                    self.with_index(i, |v| v.validate_inner(item, items_schema));
                }
            }
        }

        // For objects
        if let Some(obj) = value.as_object() {
            // required
            if let Some(req) = schema.get("required").and_then(|v| v.as_array()) {
                for field in req {
                    if let Some(field_name) = field.as_str() {
                        if !obj.contains_key(field_name) {
                            self.add_error(&format!("missing required field \"{}\"", field_name));
                        }
                    }
                }
            }

            // properties
            if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
                for (key, val) in obj {
                    if let Some(prop_schema) = props.get(key) {
                        self.with_path(key, |v| v.validate_inner(val, prop_schema));
                    } else if schema.get("additionalProperties").and_then(|v| v.as_bool()) == Some(false) {
                        self.add_error(&format!("unknown field \"{}\"", key));
                    }
                }
            }
        }
    }

    fn check_type(&mut self, value: &Value, expected: &str) {
        let actual = match value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(n) => {
                if n.is_i64() || n.is_u64() {
                    "integer"
                } else {
                    "number"
                }
            }
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        };

        let type_ok = match expected {
            "integer" => matches!(value, Value::Number(n) if n.is_i64() || n.is_u64()),
            "number" => matches!(value, Value::Number(_)),
            _ => actual == expected,
        };

        if !type_ok {
            self.add_error(&format!("expected {}, got {}", expected, actual));
        }
    }

    fn check_enum(&mut self, value: &Value, enum_vals: &[Value]) {
        if !enum_vals.iter().any(|v| v == value) {
            let expected: Vec<String> = enum_vals.iter().map(|v| serde_json::to_string(v).unwrap()).collect();
            self.add_error(&format!("expected one of [{}], got {}", expected.join(", "), serde_json::to_string(value).unwrap()));
        }
    }
}