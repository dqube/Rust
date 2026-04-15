//! Demonstrates gRPC→REST transcoding helpers in `ddd-bff`.
//!
//! Run with:
//! ```shell
//! cargo run -p ddd-bff --example transcode_errors
//! ```

use ddd_bff::{app_error_to_problem, grpc_status_to_app_error};
use ddd_shared_kernel::AppError;

fn main() {
    // gRPC Status → AppError
    let cases = vec![
        tonic::Status::not_found("order 99 not found"),
        tonic::Status::permission_denied("access denied"),
        tonic::Status::invalid_argument("bad payload"),
        tonic::Status::internal("upstream failure"),
    ];

    println!("gRPC Status → AppError → ProblemDetail:");
    for status in cases {
        let code = status.code();
        let err = grpc_status_to_app_error(status);
        let pd = app_error_to_problem(&err);
        println!("  {code:?} → HTTP {} {}", pd.status, pd.title);
    }

    // AppError → ProblemDetail
    println!("/nAppError → ProblemDetail:");
    let errors: Vec<AppError> = vec![
        AppError::not_found("Order", "42"),
        AppError::conflict("duplicate order"),
        AppError::unauthorized("expired"),
        AppError::internal("crash"),
    ];
    for err in &errors {
        let pd = app_error_to_problem(err);
        println!("  {} → HTTP {} {}", pd.problem_type, pd.status, pd.title);
    }
}
