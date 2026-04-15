//! Structured audit logging for sensitive BFF operations.
//!
//! Audit events are emitted through `tracing` at `INFO` level with the
//! dedicated target `"audit"`.  This allows operators to route them to a
//! separate sink (file, Elasticsearch, stdout filter) via tracing subscriber
//! configuration — for example:
//!
//! ```text
//! RUST_LOG=info,audit=info
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! use ddd_bff::middleware::audit::{audit, AuditEvent};
//!
//! audit(AuditEvent {
//!     action: "create_product",
//!     resource: "product",
//!     resource_id: &product_id,
//!     actor: claims.sub.as_deref().unwrap_or("anonymous"),
//!     client_ip: &client_ip,
//!     request_id: &request_id,
//!     detail: None,
//! });
//! ```
//!
//! Feature-gated on `axum-response`.

/// Fields for a single audit event.
pub struct AuditEvent<'a> {
    /// Verb describing what happened, e.g. `"create_product"`,
    /// `"deactivate_product"`, `"cancel_order"`.
    pub action: &'a str,
    /// Resource type, e.g. `"product"`, `"order"`.
    pub resource: &'a str,
    /// Identifier of the affected resource (UUID, slug, etc.).
    pub resource_id: &'a str,
    /// Identity of the actor (from JWT `sub` claim or `"anonymous"`).
    pub actor: &'a str,
    /// Client IP extracted by the observability middleware.
    pub client_ip: &'a str,
    /// Correlation id for cross-referencing with access logs.
    pub request_id: &'a str,
    /// Optional free-form detail (e.g. `"reason: out of stock"`).
    pub detail: Option<&'a str>,
}

/// Emit a structured audit log event.
///
/// Uses the `"audit"` tracing target so operators can filter/route separately.
pub fn audit(evt: AuditEvent<'_>) {
    tracing::info!(
        target: "audit",
        action = evt.action,
        resource = evt.resource,
        resource_id = evt.resource_id,
        actor = evt.actor,
        client_ip = evt.client_ip,
        request_id = evt.request_id,
        detail = evt.detail.unwrap_or(""),
        "audit_event"
    );
}
