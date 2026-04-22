use std::sync::Arc;
use axum::{routing::{get, post}, Json, Router, extract::State};
use ddd_api::prelude::*;
use ddd_api::{RestServer, Validated, FieldViolation, ProblemDetailExt};
use serde::{Deserialize, Serialize};
use validator::Validate;

// 1. Define Request/Response DTOs
#[derive(Debug, Deserialize, Validate)]
struct CreateUserRequest {
    #[validate(length(min = 3, message = "Username must be at least 3 characters"))]
    pub username: String,
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
}

#[derive(Debug, Serialize)]
struct UserDto {
    pub id: uuid::Uuid,
    pub username: String,
}

// 2. Mock Application State
#[derive(Clone)]
struct AppState {
    pub db_name: String,
}

// 3. Handler with Validation
async fn create_user(
    State(state): State<AppState>,
    Validated(req): Validated<CreateUserRequest>,
) -> Result<Json<UserDto>, ProblemDetail> {
    println!("Creating user {} in database {}", req.username, state.db_name);
    
    // Simulate domain logic
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
    // 4. Build the Router
    let state = AppState { db_name: "users_db".into() };
    
    let app = Router::new()
        .route("/users", post(create_user))
        .route("/health", get(|| async { "OK" }))
        .with_state(state);

    // 5. Run the Server
    // This will listen on 0.0.0.0:8080 by default and handle SIGTERM/SIGINT.
    println!("Starting REST server on :8080...");
    RestServer::new()
        .with_port(8080)
        .with_router(app)
        .run()
        .await?;

    Ok(())
}
