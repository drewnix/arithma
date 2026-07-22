//! Strict argument validation against the published tool schemas.
//!
//! Every tool call is checked against the tool's `inputSchema` — the same
//! object served by `tools/list` — before dispatch. The schema is the
//! contract we publish to clients; this module enforces exactly that
//! contract, so the two cannot drift apart.
//!
//! Why this exists: `args.get(key).and_then(as_type)` returns `None` both
//! when a key is absent and when it is present with the wrong type, and a
//! downstream `unwrap_or(default)` then silently substitutes the default.
//! A caller who mistypes a value (or a field name) gets an answer to a
//! question they did not ask, with no indication anything was ignored.
//! Rejecting at the boundary, naming the offending key, is the only shape
//! of this check that cannot be silently bypassed by a new tool forgetting
//! to validate: validation happens before any tool code runs.
//!
//! Supported schema subset: `type` (string or array of strings),
//! `properties` / `required` / `additionalProperties`, `items`, `enum`,
//! `minimum`, and `maximum`. Objects that declare `properties` are closed:
//! a key that is neither declared nor covered by an `additionalProperties`
//! schema is an error, because an unknown field is almost always a typo
//! whose effect would otherwise be "your option silently did nothing".
//! NOTE for schema authors: `additionalProperties: false` is NOT
//! interpreted (only object-valued `additionalProperties` schemas are) —
//! declaring `properties` already closes the object, which covers the
//! same intent. Any keyword outside this subset is silently unenforced;
//! extend the subset before relying on one.
//!
//! An explicit `null` for a non-required field means "not provided" —
//! the pre-existing contract of the assumptions plumbing.

use serde_json::Value;

/// Validate `args` against a tool's `inputSchema`. On failure the message
/// names every offending key, its expected shape, and what was received.
pub fn validate_tool_args(tool: &str, schema: &Value, args: &Value) -> Result<(), String> {
    let mut errors = Vec::new();
    validate(schema, args, "", &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Invalid arguments for tool '{}': {}",
            tool,
            errors.join("; ")
        ))
    }
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn matches_type(v: &Value, t: &str) -> bool {
    match t {
        "string" => v.is_string(),
        "number" => v.is_number(),
        // JSON Schema semantics: integral floats (7.0) are integers.
        "integer" => {
            v.is_i64()
                || v.is_u64()
                || v.as_f64()
                    .is_some_and(|f| f.is_finite() && f.fract() == 0.0)
        }
        "object" => v.is_object(),
        "array" => v.is_array(),
        "boolean" => v.is_boolean(),
        _ => true, // an unrecognized type name in our own schema never rejects
    }
}

fn join_path(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{}.{}", path, key)
    }
}

fn display_path(path: &str) -> &str {
    if path.is_empty() {
        "arguments"
    } else {
        path
    }
}

fn validate(schema: &Value, value: &Value, path: &str, errors: &mut Vec<String>) {
    if let Some(t) = schema.get("type") {
        let allowed: Vec<&str> = match t {
            Value::String(s) => vec![s.as_str()],
            Value::Array(arr) => arr.iter().filter_map(|v| v.as_str()).collect(),
            _ => vec![],
        };
        if !allowed.is_empty() && !allowed.iter().any(|t| matches_type(value, t)) {
            errors.push(format!(
                "{}: expected {}, got {}",
                display_path(path),
                allowed.join(" or "),
                type_name(value)
            ));
            // A wrong-typed value has no meaningful interior to descend into.
            return;
        }
    }

    if let Some(allowed) = schema.get("enum").and_then(|e| e.as_array()) {
        if !allowed.contains(value) {
            let list: Vec<String> = allowed
                .iter()
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .collect();
            errors.push(format!(
                "{}: must be one of [{}], got {}",
                display_path(path),
                list.join(", "),
                value
            ));
            return;
        }
    }

    if let (Some(min), Some(v)) = (
        schema.get("minimum").and_then(|m| m.as_f64()),
        value.as_f64(),
    ) {
        if v < min {
            errors.push(format!(
                "{}: must be >= {}, got {}",
                display_path(path),
                min,
                v
            ));
            return;
        }
    }

    if let (Some(max), Some(v)) = (
        schema.get("maximum").and_then(|m| m.as_f64()),
        value.as_f64(),
    ) {
        if v > max {
            errors.push(format!(
                "{}: must be <= {}, got {}",
                display_path(path),
                max,
                v
            ));
            return;
        }
    }

    if let Some(obj) = value.as_object() {
        let props = schema.get("properties").and_then(|p| p.as_object());
        let additional = schema.get("additionalProperties").filter(|a| a.is_object());

        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            for req in required.iter().filter_map(|r| r.as_str()) {
                if obj.get(req).is_none_or(Value::is_null) {
                    errors.push(format!(
                        "{}: missing required field '{}'",
                        display_path(path),
                        req
                    ));
                }
            }
        }

        for (k, v) in obj {
            if v.is_null() {
                // Explicit null on a non-required field means "not provided";
                // a null on a required field was reported above.
                continue;
            }
            let child_path = join_path(path, k);
            match (props.and_then(|p| p.get(k)), additional) {
                (Some(sub), _) => validate(sub, v, &child_path, errors),
                (None, Some(add)) => validate(add, v, &child_path, errors),
                (None, None) => {
                    if let Some(props) = props {
                        let mut allowed: Vec<&str> = props.keys().map(String::as_str).collect();
                        allowed.sort_unstable();
                        errors.push(format!(
                            "unknown field '{}' (allowed: {})",
                            child_path,
                            allowed.join(", ")
                        ));
                    }
                    // No properties and no additionalProperties declared:
                    // an open object; accept anything.
                }
            }
        }
    }

    if let (Some(arr), Some(items)) = (value.as_array(), schema.get("items")) {
        for (i, item) in arr.iter().enumerate() {
            validate(items, item, &format!("{}[{}]", path, i), errors);
        }
    }
}
