//! gRPC interceptors: authentication, tracing, and redaction.

use tonic::{Request, Status};

// ─── Auth Interceptor ────────────────────────────────────────────────────────

/// Placeholder auth interceptor.
///
/// Apply as a tonic interceptor function. Checks for a `Bearer` token in the
/// `authorization` metadata key; the actual JWT validation is delegated to a
/// user-supplied closure.
pub struct AuthInterceptor<F> {
    validator: F,
}

impl<F> AuthInterceptor<F>
where
    F: Fn(&str) -> Result<(), Status> + Send + Sync + 'static,
{
    /// Create a new auth interceptor with the given token validator.
    ///
    /// The validator receives the raw bearer token (without the `Bearer `
    /// prefix) and should return `Ok(())` on success or a [`Status`] on
    /// failure.
    pub fn new(validator: F) -> Self {
        Self { validator }
    }

    /// Intercept a request.
    #[allow(clippy::result_large_err)]
    pub fn intercept(&self, req: Request<()>) -> Result<Request<()>, Status> {
        let token = req
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));

        match token {
            Some(t) => {
                (self.validator)(t)?;
                Ok(req)
            }
            None => Err(Status::unauthenticated("missing bearer token")),
        }
    }
}

// ─── Tracing Interceptor ─────────────────────────────────────────────────────

/// Interceptor that logs gRPC call metadata via `tracing`.
///
/// Extracts `x-request-id` and `x-tenant-id` from request metadata and adds
/// them to the current tracing span.
#[allow(clippy::result_large_err)]
pub fn tracing_interceptor(req: Request<()>) -> Result<Request<()>, Status> {
    let request_id = req
        .metadata()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_owned();

    let tenant_id = req
        .metadata()
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();

    tracing::info!(request_id = %request_id, tenant_id = %tenant_id, "gRPC request");
    Ok(req)
}

// ─── Redaction Interceptor ───────────────────────────────────────────────────

/// Fields whose values should be redacted before logging.
pub const DEFAULT_REDACTED_FIELDS: &[&str] = &[
    "password",
    "secret",
    "token",
    "authorization",
    "credit_card",
    "ssn",
];

/// Redact sensitive keys from a JSON value in-place.
pub fn redact_json(value: &mut serde_json::Value, fields: &[&str]) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                let lower = key.to_lowercase();
                if fields.iter().any(|f| lower.contains(f)) {
                    *val = serde_json::Value::String("***REDACTED***".to_owned());
                } else {
                    redact_json(val, fields);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                redact_json(item, fields);
            }
        }
        _ => {}
    }
}
