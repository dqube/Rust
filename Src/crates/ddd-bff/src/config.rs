//! Generic BFF configuration.
//!
//! `ddd-bff` is a library: it does not assume which downstream services
//! a consumer talks to. [`BffConfig`] only carries settings that are
//! universal to any BFF (bind addresses, telemetry, resilience defaults,
//! redaction, request timeouts). Service-specific URLs belong in the
//! consumer's own config struct.

use std::time::Duration;

use ddd_shared_kernel::config_validation::{Report, Validate};

/// Top-level configuration for a BFF process.
#[derive(Debug, Clone)]
pub struct BffConfig {
    /// Public edge bind host (default: `0.0.0.0`).
    pub host: String,
    /// Public edge bind port (default: `3000`).
    pub port: u16,
    /// Loopback port for the Prometheus scrape server (default: `9090`).
    pub metrics_port: u16,
    /// Path to the YAML route table (default: `config/routes.yaml`).
    pub routes_path: String,
    /// Default resilience parameters applied to outgoing gRPC calls.
    pub resilience: ResilienceConfig,
    /// Comma-separated field names to redact from request/response logs.
    pub redact_fields: Vec<String>,
    /// OTLP exporter endpoint (e.g. `http://localhost:4317`).
    pub otlp_endpoint: Option<String>,
    /// Per-component graceful shutdown drain timeout.
    pub shutdown_timeout: Duration,
    /// Per-request timeout applied as the outermost Tower layer, before
    /// route matching.  Guards against slow or hung requests consuming
    /// server resources. When exceeded the server responds `408`.
    pub request_timeout: Duration,
}

/// Resilience layer parameters.
#[derive(Debug, Clone)]
pub struct ResilienceConfig {
    /// Timeout for each gRPC call.
    pub timeout: Duration,
    /// Maximum retry attempts (0 = no retries).
    pub retry_max_attempts: u32,
    /// Circuit breaker failure threshold before opening.
    pub cb_failure_threshold: u32,
    /// Circuit breaker success threshold to close again.
    pub cb_success_threshold: u32,
    /// Circuit breaker open-state timeout before half-open.
    pub cb_timeout: Duration,
    /// Maximum concurrent in-flight requests per service.
    pub max_concurrent: usize,
}

impl Default for ResilienceConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            retry_max_attempts: 3,
            cb_failure_threshold: 5,
            cb_success_threshold: 2,
            cb_timeout: Duration::from_secs(30),
            max_concurrent: 100,
        }
    }
}

impl BffConfig {
    /// Load the universal BFF configuration from environment variables.
    ///
    /// Honoured variables:
    ///
    /// | Variable                  | Default                | Field             |
    /// |---------------------------|------------------------|-------------------|
    /// | `BFF_HOST`                | `0.0.0.0`              | `host`            |
    /// | `BFF_PORT`                | `3000`                 | `port`            |
    /// | `BFF_METRICS_PORT`        | `9090`                 | `metrics_port`    |
    /// | `BFF_ROUTES_PATH`         | `config/routes.yaml`   | `routes_path`     |
    /// | `GRPC_TIMEOUT_MS`         | `5000`                 | `resilience.timeout` |
    /// | `GRPC_RETRY_MAX_ATTEMPTS` | `3`                    | `resilience.retry_max_attempts` |
    /// | `GRPC_CB_FAILURE_THRESHOLD` | `5`                  | `resilience.cb_failure_threshold` |
    /// | `GRPC_CB_SUCCESS_THRESHOLD` | `2`                  | `resilience.cb_success_threshold` |
    /// | `GRPC_CB_TIMEOUT_SECS`    | `30`                   | `resilience.cb_timeout` |
    /// | `GRPC_MAX_CONCURRENT`     | `100`                  | `resilience.max_concurrent` |
    /// | `REDACT_FIELDS`           | `password,secret,token,authorization` | `redact_fields` |
    /// | `OTLP_ENDPOINT`           | _(unset)_              | `otlp_endpoint`   |
    /// | `SHUTDOWN_TIMEOUT_SECS`   | `30`                   | `shutdown_timeout`|
    /// | `REQUEST_TIMEOUT_SECS`    | `30`                   | `request_timeout` |
    pub fn from_env() -> Self {
        Self {
            host: env_or("BFF_HOST", "0.0.0.0"),
            port: env_or("BFF_PORT", "3000").parse().unwrap_or(3000),
            metrics_port: env_or("BFF_METRICS_PORT", "9090").parse().unwrap_or(9090),
            routes_path: env_or("BFF_ROUTES_PATH", "config/routes.yaml"),
            resilience: ResilienceConfig {
                timeout: Duration::from_millis(
                    env_or("GRPC_TIMEOUT_MS", "5000").parse().unwrap_or(5000),
                ),
                retry_max_attempts: env_or("GRPC_RETRY_MAX_ATTEMPTS", "3")
                    .parse()
                    .unwrap_or(3),
                cb_failure_threshold: env_or("GRPC_CB_FAILURE_THRESHOLD", "5")
                    .parse()
                    .unwrap_or(5),
                cb_success_threshold: env_or("GRPC_CB_SUCCESS_THRESHOLD", "2")
                    .parse()
                    .unwrap_or(2),
                cb_timeout: Duration::from_secs(
                    env_or("GRPC_CB_TIMEOUT_SECS", "30").parse().unwrap_or(30),
                ),
                max_concurrent: env_or("GRPC_MAX_CONCURRENT", "100")
                    .parse()
                    .unwrap_or(100),
            },
            redact_fields: env_or("REDACT_FIELDS", "password,secret,token,authorization")
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect(),
            otlp_endpoint: std::env::var("OTLP_ENDPOINT").ok(),
            shutdown_timeout: Duration::from_secs(
                env_or("SHUTDOWN_TIMEOUT_SECS", "30")
                    .parse()
                    .unwrap_or(30),
            ),
            request_timeout: Duration::from_secs(
                env_or("REQUEST_TIMEOUT_SECS", "30")
                    .parse()
                    .unwrap_or(30),
            ),
        }
    }
}

// ─── Validation ──────────────────────────────────────────────────────────────

impl Validate for BffConfig {
    fn validate(&self, report: &mut Report) {
        if self.port == 0 {
            report.error("port", "port_zero", "port must be > 0");
        }

        if self.resilience.timeout.is_zero() {
            report.error(
                "resilience.timeout",
                "timeout_zero",
                "gRPC timeout must be > 0",
            );
        }
        if self.resilience.max_concurrent == 0 {
            report.error(
                "resilience.max_concurrent",
                "max_concurrent_zero",
                "max_concurrent must be > 0",
            );
        }

        if self.shutdown_timeout.is_zero() {
            report.error(
                "shutdown_timeout",
                "timeout_zero",
                "shutdown_timeout must be > 0",
            );
        }

        if self.request_timeout.is_zero() {
            report.error(
                "request_timeout",
                "timeout_zero",
                "request_timeout must be > 0",
            );
        }
    }
}

/// Read an environment variable or fall back to a default.
pub fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_owned())
}
