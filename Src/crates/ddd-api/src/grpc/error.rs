//! Convert [`AppError`] to [`tonic::Status`] with RFC 9457 Problem Details.
//!
//! Every [`AppError`] variant is mapped to the appropriate gRPC status code
//! **and** carries a Problem Details JSON body in the `problem-details-bin`
//! binary metadata header, giving clients the same structured error shape as
//! the REST API.

use ddd_shared_kernel::AppError;
use tonic::Status;

/// Extension trait for converting application errors into gRPC statuses.
pub trait GrpcErrorExt {
    /// Map this error to the appropriate [`tonic::Status`] with a Problem
    /// Details JSON body attached in the `problem-details-bin` metadata
    /// header.
    fn to_grpc_status(&self) -> Status;
}

impl GrpcErrorExt for AppError {
    fn to_grpc_status(&self) -> Status {
        crate::grpc::validation::app_error_to_status(self)
    }
}

/// Evaluate an expression returning `AppResult<T>` and map errors to
/// [`tonic::Status`].
///
/// # Example
/// ```ignore
/// use ddd_api::grpc_result;
///
/// async fn my_handler(req: Request<MyMsg>) -> Result<Response<MyResp>, Status> {
///     let result = grpc_result!(my_use_case.execute(req.into_inner()).await);
///     Ok(Response::new(result))
/// }
/// ```
#[macro_export]
macro_rules! grpc_result {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(err) => {
                return Err($crate::grpc::validation::app_error_to_status(&err))
            }
        }
    };
}
