//! Request-body validation extractors for Axum.
//!
//! Two extractors are provided:
//!
//! - [`Validated<T>`] — `T` self-validates via the [`RestValidator`] trait.
//! - [`ValidatedByRegistry<T>`] — runs the [`ValidatorRegistry`] validator
//!   registered for `T` (see [`ddd_application::register_validator!`]). This
//!   is the recommended extractor: validators live separately from DTOs and
//!   the same registry is shared with gRPC handlers.
//!
//! Both extractors emit RFC 9457 Problem Details responses on failure.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRef, FromRequest, FromRequestParts};
use axum::http::request::Parts;
use axum::http::Request;
use axum::response::{IntoResponse, Response};
use ddd_application::ValidatorRegistry;
use ddd_shared_kernel::validation::ValidationResult;

use super::problem_details::{ProblemDetail, ProblemDetailExt};

/// An Axum extractor that deserialises JSON and runs validation.
///
/// Implement the [`RestValidator`] trait on your request DTO, then use
/// `Validated<MyDto>` as the handler parameter instead of `Json<MyDto>`.
///
/// # Example
/// ```ignore
/// use ddd_api::rest::Validated;
///
/// async fn create(Validated(dto): Validated<CreateUserDto>) -> impl IntoResponse {
///     // dto has already been validated
/// }
/// ```
pub struct Validated<T>(pub T);

/// Trait that DTOs implement to self-validate.
pub trait RestValidator: Sized {
    /// Validate and return a [`ValidationResult`]. An empty (valid) result
    /// means the request passed validation.
    fn validate(&self) -> ValidationResult;
}

impl<S, T> FromRequest<S> for Validated<T>
where
    T: serde::de::DeserializeOwned + RestValidator + Send,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request<axum::body::Body>, state: &S) -> Result<Self, Self::Rejection> {
        let json_result: Result<axum::Json<T>, JsonRejection> =
            axum::Json::from_request(req, state).await;

        match json_result {
            Err(rejection) => {
                let pd = ProblemDetail::new(
                    422,
                    "Unprocessable Entity",
                    format!("Invalid JSON: {rejection}"),
                );
                Err(pd.into_response())
            }
            Ok(axum::Json(value)) => {
                let result = value.validate();
                if result.is_valid() {
                    Ok(Validated(value))
                } else {
                    let app_err = result.into_app_error().unwrap_err();
                    Err(app_err.to_problem_detail().into_response())
                }
            }
        }
    }
}

// ─── ValidatedByRegistry ────────────────────────────────────────────────────

/// Axum extractor that runs the validator registered for `T` in a shared
/// [`ValidatorRegistry`]. The registry is pulled from axum state via
/// [`FromRef`], so your application state must implement
/// `FromRef<State, Arc<ValidatorRegistry>>` (or hold `Arc<ValidatorRegistry>`
/// directly as its state type).
///
/// On failure the extractor returns a Problem Details response built by
/// [`ProblemDetailExt::to_problem_detail`] on [`AppError::ValidationBatch`],
/// so REST and gRPC emit structurally identical errors.
///
/// # Example
/// ```ignore
/// #[derive(Clone, FromRef)]
/// struct AppState {
///     mediator: Arc<Mediator>,
///     validators: Arc<ValidatorRegistry>,
/// }
///
/// async fn create_order(
///     ValidatedByRegistry(cmd): ValidatedByRegistry<CreateOrder>,
///     State(state): State<AppState>,
/// ) -> Result<Json<OrderDto>, ProblemDetail> {
///     let id = state.mediator.send(cmd).await.map_err(|e| e.to_problem_detail())?;
///     Ok(Json(OrderDto { id }))
/// }
/// ```
pub struct ValidatedByRegistry<T>(pub T);

impl<S, T> FromRequest<S> for ValidatedByRegistry<T>
where
    T: serde::de::DeserializeOwned + Send + 'static,
    S: Send + Sync,
    Arc<ValidatorRegistry>: FromRef<S>,
{
    type Rejection = Response;

    async fn from_request(req: Request<axum::body::Body>, state: &S) -> Result<Self, Self::Rejection> {
        let json: axum::Json<T> = axum::Json::from_request(req, state).await.map_err(
            |rejection: JsonRejection| {
                ProblemDetail::new(
                    422,
                    "Unprocessable Entity",
                    format!("Invalid JSON: {rejection}"),
                )
                .into_response()
            },
        )?;

        let registry = Arc::<ValidatorRegistry>::from_ref(state);
        match registry.validate(&json.0) {
            Ok(()) => Ok(ValidatedByRegistry(json.0)),
            Err(app_err) => Err(app_err.to_problem_detail().into_response()),
        }
    }
}

/// Marker extractor that surfaces the [`ValidatorRegistry`] without consuming
/// the request body — useful when you need to validate after partial
/// decoding.
pub struct ValidatorRegistryExt(pub Arc<ValidatorRegistry>);

impl<S> FromRequestParts<S> for ValidatorRegistryExt
where
    S: Send + Sync,
    Arc<ValidatorRegistry>: FromRef<S>,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(_: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(ValidatorRegistryExt(Arc::<ValidatorRegistry>::from_ref(state)))
    }
}

/// Validate an expression and return a [`ProblemDetail`] on failure.
///
/// # Example
/// ```ignore
/// use ddd_api::rest_validate;
///
/// rest_validate!(name, "name", |r| r.not_empty().min_length(2))?;
/// ```
#[macro_export]
macro_rules! rest_validate {
    ($value:expr, $field:expr, $rules:expr) => {{
        let __result: ddd_shared_kernel::validation::ValidationResult =
            $rules(ddd_shared_kernel::validation::ValidationRule::new($value, $field)).into();
        match __result.into_app_error() {
            Ok(()) => Ok::<(), $crate::rest::ProblemDetail>(()),
            Err(__err) => Err(
                <ddd_shared_kernel::AppError as $crate::rest::ProblemDetailExt>::to_problem_detail(
                    &__err,
                ),
            ),
        }
    }};
}
