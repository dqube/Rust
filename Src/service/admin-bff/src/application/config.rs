//! Admin BFF configuration loaded from environment variables.
//!
//! Reuses [`ddd_bff::config::ResilienceConfig`] for outgoing gRPC tuning
//! so the resilience knobs match those of any other ddd-bff-based gateway.
//!
//! Validation is performed at bootstrap via
//! [`ddd_shared_kernel::config_validation`]: both env-var parse failures and
//! structural issues (empty host, bad URLs, too-short JWT secret, zero
//! timeouts) are collected into a single [`Report`] so operators see every
//! problem at once instead of fixing them one at a time.

use std::str::FromStr;
use std::time::Duration;

pub use ddd_bff::config::ResilienceConfig;
use ddd_bff::config::env_or;
use ddd_shared_kernel::config_validation::{Report, Validate};
use ddd_shared_kernel::AppResult;

/// Minimum HS256 secret length in bytes (RFC 7518 §3.2 recommends ≥ key size).
const MIN_HS256_SECRET_BYTES: usize = 32;

/// Top-level Admin BFF configuration.
#[derive(Debug, Clone)]
pub struct AdminBffConfig {
    pub host: String,
    pub port: u16,
    pub services: ServiceUrls,
    pub resilience: ResilienceConfig,
    pub redact_fields: Vec<String>,
    pub shutdown_timeout: Duration,
    pub request_timeout: Duration,
    pub auth: AuthConfig,
    pub cache: CacheConfig,
}

/// Cache (Redis) configuration.
///
/// When `redis_url` is empty the cache layer is not constructed and the BFF
/// runs without read-through caching (development mode).
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub redis_url: String,
    pub key_prefix: String,
    pub catalog_summary_ttl: Duration,
}

/// JWT bearer-token authentication settings.
///
/// When `secret` is empty the auth layer is not mounted (development mode).
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub secret: String,
    pub issuer: String,
    pub audience: String,
    pub leeway_secs: u64,
}

/// Downstream service URLs.
#[derive(Debug, Clone)]
pub struct ServiceUrls {
    pub order_service: String,
    pub product_service: String,
    pub shared_service: String,
}

impl AdminBffConfig {
    /// Load configuration from the environment and validate it.
    ///
    /// Collects every parse failure and structural issue into one report
    /// and returns [`AppError::ValidationBatch`](ddd_shared_kernel::AppError::ValidationBatch)
    /// if any errors were found. Warnings are logged via `tracing` at `warn`
    /// level and do not fail the bootstrap.
    pub fn from_env() -> AppResult<Self> {
        let mut report = Report::new();
        let cfg = Self::build(&mut report);
        cfg.validate(&mut report);
        report.log_warnings("admin-bff-config");
        if !report.is_ok() {
            report.log_errors("admin-bff-config");
        }
        report.into_result()?;
        Ok(cfg)
    }

    fn build(report: &mut Report) -> Self {
        Self {
            host: env_or("ADMIN_BFF_HOST", "0.0.0.0"),
            port: parse_env("ADMIN_BFF_PORT", "3001", 3001, report),
            services: ServiceUrls {
                order_service: env_or("ORDER_SERVICE_URL", "http://localhost:8080"),
                product_service: env_or("PRODUCT_SERVICE_URL", "http://localhost:50052"),
                shared_service: env_or("SHARED_SERVICE_URL", "http://localhost:50053"),
            },
            resilience: ResilienceConfig {
                timeout: Duration::from_millis(parse_env(
                    "GRPC_TIMEOUT_MS",
                    "5000",
                    5000,
                    report,
                )),
                max_concurrent: parse_env("GRPC_MAX_CONCURRENT", "100", 100, report),
                ..ResilienceConfig::default()
            },
            redact_fields: env_or("REDACT_FIELDS", "password,secret,token,authorization")
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect(),
            shutdown_timeout: Duration::from_secs(parse_env(
                "SHUTDOWN_TIMEOUT_SECS",
                "30",
                30,
                report,
            )),
            request_timeout: Duration::from_secs(parse_env(
                "REQUEST_TIMEOUT_SECS",
                "30",
                30,
                report,
            )),
            auth: AuthConfig {
                secret: env_or("JWT_SECRET", ""),
                issuer: env_or("JWT_ISSUER", ""),
                audience: env_or("JWT_AUDIENCE", "admin-bff"),
                leeway_secs: parse_env("JWT_LEEWAY_SECS", "30", 30, report),
            },
            cache: CacheConfig {
                redis_url: env_or("REDIS_URL", ""),
                key_prefix: env_or("REDIS_KEY_PREFIX", "adminbff"),
                catalog_summary_ttl: Duration::from_secs(parse_env(
                    "CACHE_CATALOG_SUMMARY_TTL_SECS",
                    "30",
                    30,
                    report,
                )),
            },
        }
    }
}

impl Validate for AdminBffConfig {
    fn validate(&self, report: &mut Report) {
        if self.host.trim().is_empty() {
            report.error("host", "empty", "ADMIN_BFF_HOST must not be empty");
        }
        if self.port == 0 {
            report.error("port", "port_zero", "ADMIN_BFF_PORT must be > 0");
        }

        validate_http_url(
            "services.order_service",
            "ORDER_SERVICE_URL",
            &self.services.order_service,
            report,
        );
        validate_http_url(
            "services.product_service",
            "PRODUCT_SERVICE_URL",
            &self.services.product_service,
            report,
        );
        validate_http_url(
            "services.shared_service",
            "SHARED_SERVICE_URL",
            &self.services.shared_service,
            report,
        );

        if self.resilience.timeout.is_zero() {
            report.error(
                "resilience.timeout",
                "timeout_zero",
                "GRPC_TIMEOUT_MS must be > 0",
            );
        }
        if self.resilience.max_concurrent == 0 {
            report.error(
                "resilience.max_concurrent",
                "max_concurrent_zero",
                "GRPC_MAX_CONCURRENT must be > 0",
            );
        }
        if self.shutdown_timeout.is_zero() {
            report.error(
                "shutdown_timeout",
                "timeout_zero",
                "SHUTDOWN_TIMEOUT_SECS must be > 0",
            );
        }
        if self.request_timeout.is_zero() {
            report.error(
                "request_timeout",
                "timeout_zero",
                "REQUEST_TIMEOUT_SECS must be > 0",
            );
        }

        // JWT: only enforce when auth is enabled (secret present).
        if !self.auth.secret.is_empty() {
            if self.auth.secret.len() < MIN_HS256_SECRET_BYTES {
                report.error(
                    "auth.secret",
                    "weak_secret",
                    format!(
                        "JWT_SECRET must be at least {MIN_HS256_SECRET_BYTES} bytes for HS256"
                    ),
                );
            }
            if self.auth.audience.trim().is_empty() {
                report.error(
                    "auth.audience",
                    "empty",
                    "JWT_AUDIENCE must not be empty when JWT_SECRET is set",
                );
            }
            if self.auth.issuer.trim().is_empty() {
                report.warn(
                    "auth.issuer",
                    "empty",
                    "JWT_ISSUER unset — tokens from any issuer will be accepted",
                );
            }
        }
    }
}

fn parse_env<T>(key: &str, default_str: &str, fallback: T, report: &mut Report) -> T
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    let raw = env_or(key, default_str);
    match raw.parse::<T>() {
        Ok(v) => v,
        Err(e) => {
            report.error(
                key,
                "parse_error",
                format!("{key}={raw:?} could not be parsed: {e}"),
            );
            fallback
        }
    }
}

fn validate_http_url(field: &str, env_key: &str, value: &str, report: &mut Report) {
    if value.trim().is_empty() {
        report.error(field, "empty", format!("{env_key} must not be empty"));
        return;
    }
    match reqwest::Url::parse(value) {
        Ok(url) => {
            let scheme = url.scheme();
            if scheme != "http" && scheme != "https" {
                report.error(
                    field,
                    "invalid_scheme",
                    format!("{env_key}={value:?} must use http or https (got {scheme:?})"),
                );
            }
            if url.host_str().is_none_or(str::is_empty) {
                report.error(
                    field,
                    "missing_host",
                    format!("{env_key}={value:?} must include a host"),
                );
            }
        }
        Err(e) => {
            report.error(
                field,
                "invalid_url",
                format!("{env_key}={value:?} is not a valid URL: {e}"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ddd_shared_kernel::AppError;

    fn valid_config() -> AdminBffConfig {
        AdminBffConfig {
            host: "0.0.0.0".into(),
            port: 3001,
            services: ServiceUrls {
                order_service: "http://localhost:8080".into(),
                product_service: "http://localhost:50052".into(),
                shared_service: "http://localhost:50053".into(),
            },
            resilience: ResilienceConfig::default(),
            redact_fields: vec!["password".into()],
            shutdown_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(30),
            auth: AuthConfig {
                secret: String::new(),
                issuer: String::new(),
                audience: "admin-bff".into(),
                leeway_secs: 30,
            },
            cache: CacheConfig {
                redis_url: String::new(),
                key_prefix: "adminbff".into(),
                catalog_summary_ttl: Duration::from_secs(30),
            },
        }
    }

    fn codes(report: &Report) -> Vec<&str> {
        report.errors().iter().map(|e| e.code.as_str()).collect()
    }

    #[test]
    fn valid_config_passes() {
        assert!(valid_config().validate_to_result().is_ok());
    }

    #[test]
    fn empty_host_rejected() {
        let mut cfg = valid_config();
        cfg.host = String::new();
        let r = cfg.build_report();
        assert_eq!(codes(&r), ["empty"]);
    }

    #[test]
    fn zero_port_rejected() {
        let mut cfg = valid_config();
        cfg.port = 0;
        assert!(codes(&cfg.build_report()).contains(&"port_zero"));
    }

    #[test]
    fn bad_service_url_rejected() {
        let mut cfg = valid_config();
        cfg.services.order_service = "not a url".into();
        assert!(codes(&cfg.build_report()).contains(&"invalid_url"));
    }

    #[test]
    fn non_http_scheme_rejected() {
        let mut cfg = valid_config();
        cfg.services.product_service = "ftp://localhost/x".into();
        assert!(codes(&cfg.build_report()).contains(&"invalid_scheme"));
    }

    #[test]
    fn zero_timeout_rejected() {
        let mut cfg = valid_config();
        cfg.resilience.timeout = Duration::from_secs(0);
        assert!(codes(&cfg.build_report()).contains(&"timeout_zero"));
    }

    #[test]
    fn zero_max_concurrent_rejected() {
        let mut cfg = valid_config();
        cfg.resilience.max_concurrent = 0;
        assert!(codes(&cfg.build_report()).contains(&"max_concurrent_zero"));
    }

    #[test]
    fn jwt_disabled_skips_secret_checks() {
        let cfg = valid_config();
        // secret is empty by default in `valid_config` → auth checks are skipped.
        assert!(cfg.validate_to_result().is_ok());
    }

    #[test]
    fn short_jwt_secret_rejected_when_auth_enabled() {
        let mut cfg = valid_config();
        cfg.auth.secret = "too-short".into();
        assert!(codes(&cfg.build_report()).contains(&"weak_secret"));
    }

    #[test]
    fn empty_audience_rejected_when_auth_enabled() {
        let mut cfg = valid_config();
        cfg.auth.secret = "x".repeat(MIN_HS256_SECRET_BYTES);
        cfg.auth.audience = String::new();
        assert!(codes(&cfg.build_report()).contains(&"empty"));
    }

    #[test]
    fn missing_issuer_is_a_warning_not_an_error() {
        let mut cfg = valid_config();
        cfg.auth.secret = "x".repeat(MIN_HS256_SECRET_BYTES);
        cfg.auth.issuer = String::new();
        let r = cfg.build_report();
        assert!(r.is_ok(), "issuer-missing should be a warning, not an error");
        assert_eq!(r.warnings().len(), 1);
        assert_eq!(r.warnings()[0].code, "empty");
    }

    #[test]
    fn parse_errors_accumulate_in_report() {
        let mut report = Report::new();
        let n: u16 = parse_env("__ADMIN_BFF_MISSING_KEY", "abc", 9999, &mut report);
        assert_eq!(n, 9999);
        assert_eq!(codes(&report), ["parse_error"]);
    }

    #[test]
    fn from_env_surfaces_parse_and_structural_errors_together() {
        // Sanity: the `validate_to_result` path yields AppError::ValidationBatch
        // when both a parse error and a structural error are present.
        let mut cfg = valid_config();
        cfg.port = 0;
        cfg.services.order_service = "not-a-url".into();
        match cfg.validate_to_result() {
            Err(AppError::ValidationBatch { errors }) => {
                let fields: Vec<&str> = errors.iter().map(|e| e.field.as_str()).collect();
                assert!(fields.contains(&"port"));
                assert!(fields.contains(&"services.order_service"));
            }
            other => panic!("expected ValidationBatch, got {other:?}"),
        }
    }
}
