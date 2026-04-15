//! Redaction utilities for sensitive fields in JSON payloads.

use serde_json::Value;

/// Redact sensitive fields in a JSON [`Value`] in place.
///
/// Any key whose lowercase form appears in `redact_fields` will have its value
/// replaced with `"[REDACTED]"`.  Nested objects and arrays are traversed
/// recursively.
pub fn redact_json(value: &mut Value, redact_fields: &[String]) {
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if redact_fields.contains(&key.to_lowercase()) {
                    *val = Value::String("[REDACTED]".to_owned());
                } else {
                    redact_json(val, redact_fields);
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                redact_json(item, redact_fields);
            }
        }
        _ => {}
    }
}

/// Redact a JSON string, returning the redacted version.
///
/// If the input is not valid JSON the original string is returned unchanged.
pub fn redact_json_string(body: &str, redact_fields: &[String]) -> String {
    match serde_json::from_str::<Value>(body) {
        Ok(mut val) => {
            redact_json(&mut val, redact_fields);
            serde_json::to_string(&val).unwrap_or_else(|_| body.to_owned())
        }
        Err(_) => body.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_top_level_fields() {
        let mut val = json!({
            "username": "alice",
            "password": "secret123",
            "token": "abc"
        });
        let fields = vec!["password".to_owned(), "token".to_owned()];
        redact_json(&mut val, &fields);

        assert_eq!(val["username"], "alice");
        assert_eq!(val["password"], "[REDACTED]");
        assert_eq!(val["token"], "[REDACTED]");
    }

    #[test]
    fn redacts_nested_fields() {
        let mut val = json!({
            "user": {
                "name": "bob",
                "secret": "hidden"
            }
        });
        let fields = vec!["secret".to_owned()];
        redact_json(&mut val, &fields);

        assert_eq!(val["user"]["name"], "bob");
        assert_eq!(val["user"]["secret"], "[REDACTED]");
    }

    #[test]
    fn case_insensitive() {
        let mut val = json!({ "Password": "val" });
        let fields = vec!["password".to_owned()];
        redact_json(&mut val, &fields);
        assert_eq!(val["Password"], "[REDACTED]");
    }

    #[test]
    fn non_json_passthrough() {
        let result = redact_json_string("not json", &["password".to_owned()]);
        assert_eq!(result, "not json");
    }
}
