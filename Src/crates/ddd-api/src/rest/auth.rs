//! REST JWT authentication.
//!
//! Provides an axum extractor [`Authenticated<C>`] that reads
//! `Authorization: Bearer <token>` from the request, validates it via
//! [`JwtValidator`], and surfaces the decoded claims to handlers. Failures are
//! returned as RFC 9457 Problem Details via the crate's standard error
//! mapping.
//!
//! The validator is picked up from axum state. Mount it by implementing
//! [`ProvideJwtValidator`] on your app state (or storing a `JwtValidator<C>`
//! directly as state).
//!
//! # Example
//! ```ignore
//! use std::sync::Arc;
//! use axum::{Router, routing::get};
//! use ddd_api::rest::auth::{Authenticated, ProvideJwtValidator};
//! use ddd_shared_kernel::jwt::{JwtValidator, StandardClaims};
//!
//! #[derive(Clone)]
//! struct AppState {
//!     jwt: Arc<JwtValidator<StandardClaims>>,
//! }
//!
//! impl ProvideJwtValidator<StandardClaims> for AppState {
//!     fn jwt_validator(&self) -> &JwtValidator<StandardClaims> { &self.jwt }
//! }
//!
//! async fn me(Authenticated(claims): Authenticated<StandardClaims>) -> String {
//!     claims.sub
//! }
//!
//! let app: Router<AppState> = Router::new().route("/me", get(me));
//! ```

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use ddd_shared_kernel::jwt::JwtValidator;
use ddd_shared_kernel::AppError;
use serde::de::DeserializeOwned;

use super::problem_details::ProblemDetailExt;

/// Axum state providing a [`JwtValidator`] for claims type `C`.
pub trait ProvideJwtValidator<C>: Send + Sync {
    /// Return the validator used to decode incoming JWTs.
    fn jwt_validator(&self) -> &JwtValidator<C>;
}

impl<C> ProvideJwtValidator<C> for JwtValidator<C>
where
    C: Send + Sync,
{
    fn jwt_validator(&self) -> &JwtValidator<C> {
        self
    }
}

/// Axum extractor that validates `Authorization: Bearer <jwt>` and yields
/// the decoded claims.
///
/// Rejects with 401 Problem Details if the header is missing, malformed, the
/// signature is invalid, the token is expired, or any validation rule fails.
#[derive(Debug, Clone)]
pub struct Authenticated<C>(pub C);

impl<S, C> FromRequestParts<S> for Authenticated<C>
where
    S: ProvideJwtValidator<C> + Send + Sync,
    C: DeserializeOwned + Send + 'static,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = extract_bearer(parts).ok_or_else(|| {
            AppError::unauthorized("missing bearer token")
                .to_problem_detail()
                .into_response()
        })?;

        let data = state
            .jwt_validator()
            .validate(token)
            .map_err(|e| e.to_problem_detail().into_response())?;

        Ok(Authenticated(data.claims))
    }
}

/// Pull a bearer token out of the `Authorization` header.
///
/// Returns the raw token (without the `Bearer ` prefix) or `None` if the
/// header is absent or malformed. Exposed so callers can reuse it in custom
/// middleware.
pub fn extract_bearer(parts: &Parts) -> Option<&str> {
    parts
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|t| !t.is_empty())
}
