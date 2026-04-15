//! Demonstrates gRPC error mapping in `ddd-api`.
//!
//! Run with:
//! ```shell
//! cargo run -p ddd-api --example error_mapping --all-features
//! ```

use ddd_api::common::error_mapping::{http_status_title, IDEMPOTENCY_KEY};
use ddd_shared_kernel::AppError;

fn main() {
    // Idempotency key constant shared across transports
    println!("Idempotency key: {IDEMPOTENCY_KEY}");

    // HTTP status titles
    for code in [200u16, 400, 401, 403, 404, 409, 422, 500] {
        println!("  HTTP {code} → \"{}\"", http_status_title(code));
    }

    // AppError → http status
    let errors: Vec<AppError> = vec![
        AppError::not_found("Order", "42"),
        AppError::conflict("already exists"),
        AppError::unauthorized("token expired"),
        AppError::internal("database unavailable"),
    ];

    println!("\nAppError → HTTP status:");
    for err in &errors {
        println!("  HTTP {}", err.http_status_code());
    }
}
