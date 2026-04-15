//! gRPC JWT authentication.
//!
//! Thin adapter that plugs a [`JwtValidator`] into the existing
//! [`AuthInterceptor`]. The validator is owned by the interceptor so it can
//! be cloned across tonic layers cheaply.
//!
//! # Example
//! ```ignore
//! use std::sync::Arc;
//! use ddd_api::grpc::auth::jwt_auth_interceptor;
//! use ddd_shared_kernel::jwt::{JwtValidator, StandardClaims};
//!
//! let validator: Arc<JwtValidator<StandardClaims>> =
//!     Arc::new(JwtValidator::hs256(b"secret").with_issuer(["issuer"]));
//! let interceptor = jwt_auth_interceptor(validator);
//! let svc = MyServiceServer::with_interceptor(my_impl, interceptor);
//! ```

use std::sync::Arc;

use ddd_shared_kernel::jwt::JwtValidator;
use serde::de::DeserializeOwned;
use tonic::{Request, Status};

use super::interceptor::AuthInterceptor;

/// Build a tonic interceptor closure that validates the bearer token in the
/// `authorization` metadata key using `validator` and rejects requests with
/// [`Status::unauthenticated`] on failure.
///
/// The decoded claims are inserted into the request's extensions as
/// [`JwtClaims<C>`] so downstream handlers can read them.
pub fn jwt_auth_interceptor<C>(
    validator: Arc<JwtValidator<C>>,
) -> impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone + Send + Sync + 'static
where
    C: DeserializeOwned + Clone + Send + Sync + 'static,
{
    move |mut req: Request<()>| {
        let token = req
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .ok_or_else(|| Status::unauthenticated("missing bearer token"))?;

        let data = validator
            .validate(token)
            .map_err(|e| Status::unauthenticated(e.to_string()))?;

        req.extensions_mut().insert(JwtClaims(data.claims));
        Ok(req)
    }
}

/// Build the closure-form [`AuthInterceptor`] backed by a [`JwtValidator`].
///
/// Equivalent to `AuthInterceptor::new(...)` with a JWT-backed validator.
pub fn jwt_auth_interceptor_struct<C>(
    validator: Arc<JwtValidator<C>>,
) -> AuthInterceptor<impl Fn(&str) -> Result<(), Status> + Send + Sync + 'static>
where
    C: DeserializeOwned + Send + Sync + 'static,
{
    AuthInterceptor::new(move |token: &str| {
        validator
            .validate(token)
            .map(|_| ())
            .map_err(|e| Status::unauthenticated(e.to_string()))
    })
}

/// Wrapper around decoded JWT claims, stored in a [`Request`]'s extensions.
///
/// Retrieve with `req.extensions().get::<JwtClaims<MyClaims>>()`.
#[derive(Debug, Clone)]
pub struct JwtClaims<C>(pub C);
