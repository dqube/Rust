//! Axum JWT authentication middleware for BFF gateways.
//!
//! Built on [`ddd_shared_kernel::jwt::JwtValidator`]. Two shapes are exposed:
//!
//! - [`jwt_auth_layer`] — an [`axum::middleware::from_fn_with_state`]
//!   middleware that validates `Authorization: Bearer <jwt>` and inserts the
//!   decoded claims as a request extension so handlers can pick them up with
//!   `Extension(claims): Extension<C>`.
//! - [`Authenticated<C>`] — an axum extractor that re-reads those same claims.
//!   Handlers use *either* pattern; both are equivalent.
//!
//! Failures are returned as RFC 9457 Problem Details.
//!
//! Feature-gated on `jwt` (which enables `axum-response`).
//!
//! # Example
//! ```ignore
//! use std::sync::Arc;
//! use axum::{middleware as axum_mw, routing::get, Extension, Router};
//! use ddd_bff::middleware::axum_auth::jwt_auth_layer;
//! use ddd_shared_kernel::jwt::{JwtValidator, StandardClaims};
//!
//! let validator: Arc<JwtValidator<StandardClaims>> =
//!     Arc::new(JwtValidator::hs256(b"secret").with_issuer(["issuer"]));
//!
//! async fn me(Extension(claims): Extension<StandardClaims>) -> String {
//!     claims.sub
//! }
//!
//! let protected = Router::new()
//!     .route("/me", get(me))
//!     .layer(axum_mw::from_fn_with_state(validator, jwt_auth_layer::<StandardClaims>));
//! ```

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{FromRequestParts, State};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use ddd_shared_kernel::jwt::JwtValidator;
use serde::de::DeserializeOwned;
use serde_json::json;

/// Axum middleware that validates `Authorization: Bearer <jwt>` against the
/// supplied [`JwtValidator`] and stores the decoded claims as an extension on
/// the request.
///
/// Reject with `401 application/problem+json` on any failure.
///
/// `C` is the claims type stored in the extension. Downstream handlers read it
/// with `Extension(claims): Extension<C>` or via [`Authenticated<C>`].
pub async fn jwt_auth_layer<C>(
    State(validator): State<Arc<JwtValidator<C>>>,
    mut req: Request<Body>,
    next: Next,
) -> Response
where
    C: DeserializeOwned + Clone + Send + Sync + 'static,
{
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|t| !t.is_empty());

    let Some(token) = token else {
        return problem(401, "Unauthorized", "missing bearer token");
    };

    match validator.validate(token) {
        Ok(data) => {
            req.extensions_mut().insert(data.claims);
            next.run(req).await
        }
        Err(e) => problem(401, "Unauthorized", &e.to_string()),
    }
}

/// Axum extractor that returns the JWT claims inserted by [`jwt_auth_layer`].
///
/// Use this in handlers that want the claims inline rather than via
/// `Extension<C>`. Missing claims (layer not mounted) returns a 500 so that
/// configuration mistakes surface immediately rather than silently denying.
#[derive(Debug, Clone)]
pub struct Authenticated<C>(pub C);

impl<S, C> FromRequestParts<S> for Authenticated<C>
where
    S: Send + Sync,
    C: Clone + Send + Sync + 'static,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<C>()
            .cloned()
            .map(Authenticated)
            .ok_or_else(|| {
                problem(
                    500,
                    "Internal Server Error",
                    "authentication layer not mounted",
                )
            })
    }
}

fn problem(status: u16, title: &str, detail: &str) -> Response {
    let body = json!({
        "type": "about:blank",
        "title": title,
        "status": status,
        "detail": detail,
    });
    let status = StatusCode::from_u16(status).unwrap_or(StatusCode::UNAUTHORIZED);
    let mut resp = (status, Json(body)).into_response();
    if let Ok(ct) = "application/problem+json".parse() {
        resp.headers_mut().insert(axum::http::header::CONTENT_TYPE, ct);
    }
    resp
}
