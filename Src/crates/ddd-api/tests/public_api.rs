//! Integration smoke-test: verifies the public API of `ddd-api`
//! is reachable from an external crate boundary.

use ddd_api::common::error_mapping::{http_status_title, IDEMPOTENCY_KEY};

#[test]
fn idempotency_key_constant_value() {
    assert_eq!(IDEMPOTENCY_KEY, "idempotency-key");
}

#[test]
fn http_status_title_known_codes() {
    assert_eq!(http_status_title(200), "Error"); // 200 not explicitly mapped → "Error"
    assert_eq!(http_status_title(400), "Bad Request");
    assert_eq!(http_status_title(401), "Unauthorized");
    assert_eq!(http_status_title(403), "Forbidden");
    assert_eq!(http_status_title(404), "Not Found");
    assert_eq!(http_status_title(409), "Conflict");
    assert_eq!(http_status_title(422), "Unprocessable Entity");
    assert_eq!(http_status_title(500), "Internal Server Error");
}
