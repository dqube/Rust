//! Auth Service — gRPC server (gRPC + gRPC-Web on port 50054).
//!
//! Wires the Postgres repositories, JWT token service, PBKDF2 password hasher,
//! and mediator into a single tonic server. DB bootstrap follows
//! shared-service: create the `auth` schema if missing, run sqlx migrations,
//! then reconnect with `search_path=auth,public` pinned.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use ddd_application::Mediator;
use ddd_infrastructure::{create_pool, run_migrations_from_path, Pbkdf2Hasher};
use ddd_shared_kernel::Hasher;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use auth_service::api::grpc::AuthGrpcService;
use auth_service::application::deps::AppDeps;
use auth_service::domain::token_service::TokenService;
use auth_service::infrastructure::db::repositories::{
    PgPasswordResetTokenRepository, PgRefreshTokenRepository, PgRolePermissionRepository,
    PgRoleRepository, PgUserRepository, PgUserRoleRepository,
};
use auth_service::infrastructure::db::seeder;
use auth_service::infrastructure::jwt_token_service::{JwtTokenService, JwtTokenServiceConfig};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Auth Service");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/modernstores".into());

    // Ensure the `auth` schema exists before sqlx migrations run.
    let init_pool = create_pool(&database_url)
        .await
        .expect("failed to connect to database for schema bootstrap");
    init_pool
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            "CREATE SCHEMA IF NOT EXISTS auth".to_owned(),
        ))
        .await
        .expect("failed to create `auth` schema");
    drop(init_pool);

    run_migrations_from_path(&database_url, "./migrations")
        .await
        .expect("failed to run auth-service migrations");

    // Connect with search_path pinned to `auth,public`.
    let url_with_path = if database_url.contains('?') {
        format!("{database_url}&options=-c%20search_path%3Dauth,public")
    } else {
        format!("{database_url}?options=-c%20search_path%3Dauth,public")
    };
    let db = Arc::new(
        create_pool(&url_with_path)
            .await
            .expect("failed to connect to database"),
    );

    seeder::run_seeder(&db).await;

    // ── Token service (JWT) ─────────────────────────────────────────────
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        tracing::warn!(
            "JWT_SECRET unset — using an insecure development default. DO NOT use in production."
        );
        "development-secret-replace-me-at-least-32-bytes-long".to_owned()
    });
    let token_service: Arc<dyn TokenService> = Arc::new(
        JwtTokenService::new(JwtTokenServiceConfig {
            secret: jwt_secret,
            issuer: std::env::var("JWT_ISSUER").unwrap_or_else(|_| "auth-service".into()),
            audience: std::env::var("JWT_AUDIENCE").unwrap_or_else(|_| "admin-bff".into()),
            access_ttl: Duration::from_secs(parse_env("ACCESS_TOKEN_TTL_SECS", 900)),
            refresh_ttl: Duration::from_secs(parse_env(
                "REFRESH_TOKEN_TTL_SECS",
                60 * 60 * 24 * 30, // 30 days
            )),
        })
        .expect("invalid JWT token service configuration"),
    );

    let password_hasher: Arc<dyn Hasher> = Arc::new(Pbkdf2Hasher::new());

    let deps = AppDeps {
        user_repo: Arc::new(PgUserRepository(db.clone())),
        role_repo: Arc::new(PgRoleRepository(db.clone())),
        user_role_repo: Arc::new(PgUserRoleRepository(db.clone())),
        role_permission_repo: Arc::new(PgRolePermissionRepository(db.clone())),
        refresh_token_repo: Arc::new(PgRefreshTokenRepository(db.clone())),
        password_reset_repo: Arc::new(PgPasswordResetTokenRepository(db.clone())),
        password_hasher,
        token_service,
        password_reset_ttl_secs: parse_env("PASSWORD_RESET_TTL_SECS", 3600),
    };
    let mediator = Arc::new(Mediator::from_inventory(&deps));

    let grpc_service = AuthGrpcService::new(mediator);

    let addr: SocketAddr = ([0, 0, 0, 0], 50054).into();
    tracing::info!("gRPC + gRPC-Web on 0.0.0.0:50054");

    if let Err(e) = Server::builder()
        .accept_http1(true)
        .layer(GrpcWebLayer::new())
        .add_service(grpc_service.into_server())
        .serve_with_shutdown(addr, shutdown_signal())
        .await
    {
        tracing::error!(error = %e, "gRPC server exited with error");
    }
}

fn parse_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => tracing::info!("received SIGINT"),
        () = terminate => tracing::info!("received SIGTERM"),
    }
}
