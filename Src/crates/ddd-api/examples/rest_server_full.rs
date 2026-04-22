//! Demonstrates a complete REST server using `ddd-api`'s `RestServer`,
//! the `Validated` extractor, and RFC 9457 Problem Details error mapping.

use axum::{extract::State, routing::{get, post}, Json, Router};
use ddd_api::prelude::*;
use ddd_api::rest::{RestValidator, Validated};
use ddd_api::{ProblemDetailExt, RestServer};
use ddd_shared_kernel::validation::{ValidationResult, ValidationRule};
use serde::{Deserialize, Serialize};

// 1. Define Request/Response DTOs
#[derive(Debug, Deserialize)]
struct CreateUserRequest {
    pub username: String,
    pub email: String,
}

// 2. Implement RestValidator (fluent validation, no external validator crate needed)
impl RestValidator for CreateUserRequest {
    fn validate(&self) -> ValidationResult {
        let username = ValidationRule::new(self.username.as_str(), "username")
            .min_length(3)
            .finish();
        let email = ValidationRule::new(self.email.as_str(), "email")
            .email()
            .finish();
        username.and(email)
    }
}

#[derive(Debug, Serialize)]
struct UserDto {
    pub id: uuid::Uuid,
    pub username: String,
}

// 3. Mock Application State
#[derive(Clone)]
struct AppState {
    pub db_name: String,
}

// 4. Handler with Validation
async fn create_user(
    State(state): State<AppState>,
    Validated(req): Validated<CreateUserRequest>,
) -> Result<Json<UserDto>, ProblemDetail> {
    println!("Creating user {} in database {}", req.username, state.db_name);

    if req.username == "error" {
        return Err(AppError::validation("username", "This username is banned").to_problem_detail());
    }

    Ok(Json(UserDto {
        id: uuid::Uuid::now_v7(),
        username: req.username,
    }))
}

#[tokio::main]
async fn main() -> AppResult<()> {
    // 5. Build the Router
    let state = AppState { db_name: "users_db".into() };

    let app = Router::new()
        .route("/users", post(create_user))
        .route("/health", get(|| async { "OK" }))
        .with_state(state);

    // 6. Run the Server with graceful shutdown (SIGTERM / SIGINT).
    println!("Starting REST server on :8080...");
    RestServer::new()
        .with_port(8080)
        .with_router(app)
        .run()
        .await?;

    Ok(())
}
