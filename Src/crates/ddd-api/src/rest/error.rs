//! Implement [`IntoResponse`] for [`AppError`] via [`ProblemDetail`].

use axum::response::{IntoResponse, Response};
use ddd_shared_kernel::AppError;

use super::problem_details::ProblemDetailExt;

/// Newtype so we can implement `IntoResponse` without orphan rules.
///
/// Wrap an `AppError` in this to return it directly from an Axum handler:
///
/// ```ignore
/// async fn handler() -> Result<Json<MyDto>, RestErrorResponse> {
///     let result = my_use_case.execute(input).await.map_err(RestErrorResponse)?;
///     Ok(Json(result))
/// }
/// ```
pub struct RestErrorResponse(pub AppError);

impl From<AppError> for RestErrorResponse {
    fn from(err: AppError) -> Self {
        Self(err)
    }
}

impl IntoResponse for RestErrorResponse {
    fn into_response(self) -> Response {
        self.0.to_problem_detail().into_response()
    }
}
