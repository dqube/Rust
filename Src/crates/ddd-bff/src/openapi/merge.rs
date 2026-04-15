//! Downstream OpenAPI spec merging.
//!
//! Fetches a downstream service's OpenAPI spec and merges its `paths` and
//! `components.schemas` into a base spec, prefixing every imported path.
//!
//! On failure (downstream unreachable at startup) the function returns
//! `base_spec` unchanged and logs a warning — the BFF still starts.
//!
//! Feature-gated on `axum-response` (requires `reqwest`).

/// Fetch the downstream service's OpenAPI spec at `downstream_spec_url` and
/// merge its paths + components into `base_spec`.
///
/// Each imported path is prefixed with `prefix` (e.g. `"/admin"`).
/// Existing paths in `base_spec` are **not** overwritten.
///
/// Returns the merged spec, or `base_spec` unchanged if the downstream is
/// unreachable or returns invalid JSON.
pub async fn merged_openapi(
    base_spec: serde_json::Value,
    downstream_spec_url: &str,
    prefix: &str,
) -> serde_json::Value {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return base_spec,
    };

    let downstream: serde_json::Value = match client.get(downstream_spec_url).send().await {
        Ok(r) => match r.json().await {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(error = %err, "failed to parse downstream OpenAPI");
                return base_spec;
            }
        },
        Err(err) => {
            tracing::warn!(
                error = %err,
                "downstream OpenAPI unreachable; BFF serves own spec only"
            );
            return base_spec;
        }
    };

    let mut merged = base_spec;

    // Merge paths with prefix.
    if let Some(down_paths) = downstream.get("paths").and_then(|v| v.as_object()) {
        let paths = merged
            .as_object_mut()
            .and_then(|m| m.get_mut("paths"))
            .and_then(|v| v.as_object_mut());
        if let Some(paths) = paths {
            for (path, ops) in down_paths {
                let prefixed = format!("{prefix}{path}");
                paths.entry(prefixed).or_insert_with(|| ops.clone());
            }
        }
    }

    // Merge components.schemas (non-destructive — existing keys win).
    if let Some(down_schemas) = downstream
        .get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(|s| s.as_object())
    {
        let merged_schemas = merged
            .as_object_mut()
            .and_then(|m| m.get_mut("components"))
            .and_then(|c| c.as_object_mut())
            .and_then(|c| {
                if !c.contains_key("schemas") {
                    c.insert("schemas".into(), serde_json::json!({}));
                }
                c.get_mut("schemas")
            })
            .and_then(|s| s.as_object_mut());
        if let Some(schemas) = merged_schemas {
            for (name, schema) in down_schemas {
                schemas.entry(name.clone()).or_insert_with(|| schema.clone());
            }
        }
    }

    merged
}
