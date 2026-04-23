//! Admin BFF configuration.
//!
//! Loads settings in three layers, each overriding the previous:
//!
//! 1. **Built-in defaults** — see [`AdminBffConfig::defaults`].
//! 2. **YAML file** (optional) — path from `ADMIN_BFF_CONFIG`, else
//!    `config/admin-bff.yaml`. Missing file is not an error.
//! 3. **Environment variables** — every field that was previously
//!    configurable via env is still honoured; env wins over YAML so
//!    deploy-time overrides remain possible without editing the file.
//!
//! Shared BFF knobs (bind host/port, resilience, redact fields, timeouts)
//! are carried inside [`ddd_bff::config::BffConfig`] so admin-bff inherits
//! any validation rule added upstream. Admin-specific settings (service
//! URLs, JWT, cache) live alongside.
//!
//! Validation is performed after all layers have been merged via
//! [`ddd_shared_kernel::config_validation`]: parse failures and structural
//! issues are collected into a single [`Report`] so operators see every
//! problem at once instead of fixing them one at a time.

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

pub use ddd_bff::config::{BffConfig, ResilienceConfig};
use ddd_shared_kernel::config_validation::{Report, Validate};
use ddd_shared_kernel::AppResult;
use serde::Deserialize;

/// Minimum HS256 secret length in bytes (RFC 7518 §3.2 recommends ≥ key size).
const MIN_HS256_SECRET_BYTES: usize = 32;

/// Default YAML path when `ADMIN_BFF_CONFIG` is unset.
const DEFAULT_YAML_PATH: &str = "config/admin-bff.yaml";

/// Top-level Admin BFF configuration.
///
/// Shared BFF knobs are delegated to [`BffConfig`]; admin-specific
/// configuration sits alongside.
#[derive(Debug, Clone)]
pub struct AdminBffConfig {
    /// Generic BFF knobs: host, port, resilience, redact fields, shutdown /
    /// request timeouts. Sourced from `ddd-bff` so validation stays in sync.
    pub bff: BffConfig,
    pub services: ServiceUrls,
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
    pub auth_service: String,
    pub customer_service: String,
    pub employee_service: String,
    pub supplier_service: String,
    pub catalog_service:  String,
    pub sales_service:    String,
}

impl AdminBffConfig {
    /// Load configuration (defaults → YAML → env) and validate it.
    ///
    /// Returns [`AppError::ValidationBatch`](ddd_shared_kernel::AppError::ValidationBatch)
    /// if any errors were found. Warnings are logged at `warn` level and do
    /// not fail the bootstrap.
    pub fn from_env() -> AppResult<Self> {
        let mut report = Report::new();
        let mut cfg = Self::defaults();

        apply_yaml_layer(&mut cfg, &mut report);
        apply_env_layer(&mut cfg, &mut report);

        cfg.validate(&mut report);
        report.log_warnings("admin-bff-config");
        if !report.is_ok() {
            report.log_errors("admin-bff-config");
        }
        report.into_result()?;
        Ok(cfg)
    }

    /// Built-in defaults — used when neither YAML nor env provides a value.
    pub fn defaults() -> Self {
        Self {
            bff: BffConfig {
                host: "0.0.0.0".into(),
                port: 3001,
                resilience: ResilienceConfig {
                    timeout: Duration::from_millis(5000),
                    max_concurrent: 100,
                    ..ResilienceConfig::default()
                },
                redact_fields: vec![
                    "password".into(),
                    "secret".into(),
                    "token".into(),
                    "authorization".into(),
                ],
                otlp_endpoint: None,
                shutdown_timeout: Duration::from_secs(30),
                request_timeout: Duration::from_secs(30),
            },
            services: ServiceUrls {
                order_service: "http://localhost:8080".into(),
                product_service: "http://localhost:50052".into(),
                shared_service: "http://localhost:50053".into(),
                auth_service: "http://localhost:50054".into(),
                customer_service: "http://localhost:50055".into(),
                employee_service: "http://localhost:50056".into(),
                supplier_service: "http://localhost:50057".into(),
                catalog_service:  "http://localhost:50058".into(),
                sales_service:    "http://localhost:50060".into(),
            },
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
}

impl Validate for AdminBffConfig {
    fn validate(&self, report: &mut Report) {
        // Shared BFF knobs — delegate to BffConfig so rules stay in sync.
        self.bff.validate(report);

        // Admin-bff also requires a non-empty host (BffConfig allows empty).
        if self.bff.host.trim().is_empty() {
            report.error("host", "empty", "host must not be empty");
        }

        validate_http_url("services.order_service", &self.services.order_service, report);
        validate_http_url(
            "services.product_service",
            &self.services.product_service,
            report,
        );
        validate_http_url("services.shared_service", &self.services.shared_service, report);
        validate_http_url("services.auth_service", &self.services.auth_service, report);
        validate_http_url(
            "services.customer_service",
            &self.services.customer_service,
            report,
        );
        validate_http_url(
            "services.employee_service",
            &self.services.employee_service,
            report,
        );
        validate_http_url(
            "services.supplier_service",
            &self.services.supplier_service,
            report,
        );
        validate_http_url(
            "services.catalog_service",
            &self.services.catalog_service,
            report,
        );
        validate_http_url(
            "services.sales_service",
            &self.services.sales_service,
            report,
        );

        // JWT: only enforce when auth is enabled (secret present).
        if !self.auth.secret.is_empty() {
            if self.auth.secret.len() < MIN_HS256_SECRET_BYTES {
                report.error(
                    "auth.secret",
                    "weak_secret",
                    format!(
                        "auth.secret must be at least {MIN_HS256_SECRET_BYTES} bytes for HS256"
                    ),
                );
            }
            if self.auth.audience.trim().is_empty() {
                report.error(
                    "auth.audience",
                    "empty",
                    "auth.audience must not be empty when auth.secret is set",
                );
            }
            if self.auth.issuer.trim().is_empty() {
                report.warn(
                    "auth.issuer",
                    "empty",
                    "auth.issuer unset — tokens from any issuer will be accepted",
                );
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// YAML layer
// ────────────────────────────────────────────────────────────────────────────

/// Intermediate, fully-optional mirror of [`AdminBffConfig`] used for YAML
/// ingestion. Any field left `None` falls through to the preceding layer
/// (defaults).
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ConfigFile {
    host: Option<String>,
    port: Option<u16>,
    services: ServicesFile,
    resilience: ResilienceFile,
    redact_fields: Option<Vec<String>>,
    shutdown_timeout_secs: Option<u64>,
    request_timeout_secs: Option<u64>,
    otlp_endpoint: Option<String>,
    auth: AuthFile,
    cache: CacheFile,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ServicesFile {
    order_service: Option<String>,
    product_service: Option<String>,
    shared_service: Option<String>,
    auth_service: Option<String>,
    customer_service: Option<String>,
    employee_service: Option<String>,
    supplier_service: Option<String>,
    catalog_service:  Option<String>,
    sales_service:    Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ResilienceFile {
    timeout_ms: Option<u64>,
    max_concurrent: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct AuthFile {
    secret: Option<String>,
    issuer: Option<String>,
    audience: Option<String>,
    leeway_secs: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct CacheFile {
    redis_url: Option<String>,
    key_prefix: Option<String>,
    catalog_summary_ttl_secs: Option<u64>,
}

fn apply_yaml_layer(cfg: &mut AdminBffConfig, report: &mut Report) {
    let (path, explicit) = resolve_yaml_path();
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if explicit {
                report.error(
                    "config_file",
                    "not_found",
                    format!("ADMIN_BFF_CONFIG={} does not exist", path.display()),
                );
            }
            return;
        }
        Err(e) => {
            report.error(
                "config_file",
                "io_error",
                format!("failed to read {}: {e}", path.display()),
            );
            return;
        }
    };

    let file: ConfigFile = match serde_yaml::from_str(&contents) {
        Ok(f) => f,
        Err(e) => {
            report.error(
                "config_file",
                "parse_error",
                format!("{} is not valid YAML: {e}", path.display()),
            );
            return;
        }
    };

    overlay_file(cfg, file);
    tracing::info!(path = %path.display(), "loaded YAML config");
}

fn resolve_yaml_path() -> (PathBuf, bool) {
    match std::env::var("ADMIN_BFF_CONFIG") {
        Ok(p) if !p.trim().is_empty() => (PathBuf::from(p), true),
        _ => (PathBuf::from(DEFAULT_YAML_PATH), false),
    }
}

fn overlay_file(cfg: &mut AdminBffConfig, f: ConfigFile) {
    if let Some(v) = f.host {
        cfg.bff.host = v;
    }
    if let Some(v) = f.port {
        cfg.bff.port = v;
    }
    if let Some(v) = f.services.order_service {
        cfg.services.order_service = v;
    }
    if let Some(v) = f.services.product_service {
        cfg.services.product_service = v;
    }
    if let Some(v) = f.services.shared_service {
        cfg.services.shared_service = v;
    }
    if let Some(v) = f.services.auth_service {
        cfg.services.auth_service = v;
    }
    if let Some(v) = f.services.customer_service {
        cfg.services.customer_service = v;
    }
    if let Some(v) = f.services.employee_service {
        cfg.services.employee_service = v;
    }
    if let Some(v) = f.services.supplier_service {
        cfg.services.supplier_service = v;
    }
    if let Some(v) = f.services.catalog_service {
        cfg.services.catalog_service = v;
    }
    if let Some(v) = f.services.sales_service {
        cfg.services.sales_service = v;
    }
    if let Some(v) = f.resilience.timeout_ms {
        cfg.bff.resilience.timeout = Duration::from_millis(v);
    }
    if let Some(v) = f.resilience.max_concurrent {
        cfg.bff.resilience.max_concurrent = v;
    }
    if let Some(v) = f.redact_fields {
        cfg.bff.redact_fields = v
            .into_iter()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
    }
    if let Some(v) = f.shutdown_timeout_secs {
        cfg.bff.shutdown_timeout = Duration::from_secs(v);
    }
    if let Some(v) = f.request_timeout_secs {
        cfg.bff.request_timeout = Duration::from_secs(v);
    }
    if let Some(v) = f.otlp_endpoint {
        cfg.bff.otlp_endpoint = Some(v);
    }
    if let Some(v) = f.auth.secret {
        cfg.auth.secret = v;
    }
    if let Some(v) = f.auth.issuer {
        cfg.auth.issuer = v;
    }
    if let Some(v) = f.auth.audience {
        cfg.auth.audience = v;
    }
    if let Some(v) = f.auth.leeway_secs {
        cfg.auth.leeway_secs = v;
    }
    if let Some(v) = f.cache.redis_url {
        cfg.cache.redis_url = v;
    }
    if let Some(v) = f.cache.key_prefix {
        cfg.cache.key_prefix = v;
    }
    if let Some(v) = f.cache.catalog_summary_ttl_secs {
        cfg.cache.catalog_summary_ttl = Duration::from_secs(v);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Env-var layer
// ────────────────────────────────────────────────────────────────────────────

/// Apply env-var overrides. Only variables that are explicitly set overwrite
/// the current value — unset variables keep the YAML / default value.
fn apply_env_layer(cfg: &mut AdminBffConfig, report: &mut Report) {
    apply_str("ADMIN_BFF_HOST", |v| cfg.bff.host = v);
    apply_parse::<u16>("ADMIN_BFF_PORT", report, |v| cfg.bff.port = v);

    apply_str("ORDER_SERVICE_URL", |v| cfg.services.order_service = v);
    apply_str("PRODUCT_SERVICE_URL", |v| cfg.services.product_service = v);
    apply_str("SHARED_SERVICE_URL", |v| cfg.services.shared_service = v);
    apply_str("AUTH_SERVICE_URL", |v| cfg.services.auth_service = v);
    apply_str("CUSTOMER_SERVICE_URL", |v| cfg.services.customer_service = v);
    apply_str("EMPLOYEE_SERVICE_URL", |v| cfg.services.employee_service = v);
    apply_str("SUPPLIER_SERVICE_URL", |v| cfg.services.supplier_service = v);
    apply_str("CATALOG_SERVICE_URL",  |v| cfg.services.catalog_service = v);
    apply_str("SALES_SERVICE_URL",    |v| cfg.services.sales_service = v);

    apply_parse::<u64>("GRPC_TIMEOUT_MS", report, |v| {
        cfg.bff.resilience.timeout = Duration::from_millis(v)
    });
    apply_parse::<usize>("GRPC_MAX_CONCURRENT", report, |v| {
        cfg.bff.resilience.max_concurrent = v
    });

    if let Ok(raw) = std::env::var("REDACT_FIELDS") {
        cfg.bff.redact_fields = raw
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
    }

    apply_parse::<u64>("SHUTDOWN_TIMEOUT_SECS", report, |v| {
        cfg.bff.shutdown_timeout = Duration::from_secs(v)
    });
    apply_parse::<u64>("REQUEST_TIMEOUT_SECS", report, |v| {
        cfg.bff.request_timeout = Duration::from_secs(v)
    });
    apply_str("OTLP_ENDPOINT", |v| cfg.bff.otlp_endpoint = Some(v));

    apply_str("JWT_SECRET", |v| cfg.auth.secret = v);
    apply_str("JWT_ISSUER", |v| cfg.auth.issuer = v);
    apply_str("JWT_AUDIENCE", |v| cfg.auth.audience = v);
    apply_parse::<u64>("JWT_LEEWAY_SECS", report, |v| cfg.auth.leeway_secs = v);

    apply_str("REDIS_URL", |v| cfg.cache.redis_url = v);
    apply_str("REDIS_KEY_PREFIX", |v| cfg.cache.key_prefix = v);
    apply_parse::<u64>("CACHE_CATALOG_SUMMARY_TTL_SECS", report, |v| {
        cfg.cache.catalog_summary_ttl = Duration::from_secs(v)
    });
}

fn apply_str(key: &str, mut set: impl FnMut(String)) {
    if let Ok(v) = std::env::var(key) {
        set(v);
    }
}

fn apply_parse<T>(key: &str, report: &mut Report, mut set: impl FnMut(T))
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    let Ok(raw) = std::env::var(key) else { return };
    match raw.parse::<T>() {
        Ok(v) => set(v),
        Err(e) => report.error(
            key,
            "parse_error",
            format!("{key}={raw:?} could not be parsed: {e}"),
        ),
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Shared helpers
// ────────────────────────────────────────────────────────────────────────────

fn validate_http_url(field: &str, value: &str, report: &mut Report) {
    if value.trim().is_empty() {
        report.error(field, "empty", format!("{field} must not be empty"));
        return;
    }
    match reqwest::Url::parse(value) {
        Ok(url) => {
            let scheme = url.scheme();
            if scheme != "http" && scheme != "https" {
                report.error(
                    field,
                    "invalid_scheme",
                    format!("{field}={value:?} must use http or https (got {scheme:?})"),
                );
            }
            if url.host_str().is_none_or(str::is_empty) {
                report.error(
                    field,
                    "missing_host",
                    format!("{field}={value:?} must include a host"),
                );
            }
        }
        Err(e) => {
            report.error(
                field,
                "invalid_url",
                format!("{field}={value:?} is not a valid URL: {e}"),
            );
        }
    }
}

/// Convenience — useful for tests and for callers that want to check a
/// candidate YAML file without going through the env-var layer.
#[allow(dead_code)]
pub(crate) fn load_yaml_from_path(path: &Path) -> AppResult<AdminBffConfig> {
    let contents = std::fs::read_to_string(path).map_err(|e| {
        ddd_shared_kernel::AppError::internal(format!("failed to read {}: {e}", path.display()))
    })?;
    let file: ConfigFile = serde_yaml::from_str(&contents).map_err(|e| {
        ddd_shared_kernel::AppError::internal(format!("{} is not valid YAML: {e}", path.display()))
    })?;
    let mut cfg = AdminBffConfig::defaults();
    overlay_file(&mut cfg, file);
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ddd_shared_kernel::AppError;

    fn valid_config() -> AdminBffConfig {
        AdminBffConfig::defaults()
    }

    fn codes(report: &Report) -> Vec<&str> {
        report.errors().iter().map(|e| e.code.as_str()).collect()
    }

    #[test]
    fn defaults_pass_validation() {
        assert!(valid_config().validate_to_result().is_ok());
    }

    #[test]
    fn empty_host_rejected() {
        let mut cfg = valid_config();
        cfg.bff.host = String::new();
        let r = cfg.build_report();
        assert!(codes(&r).contains(&"empty"));
    }

    #[test]
    fn zero_port_rejected() {
        let mut cfg = valid_config();
        cfg.bff.port = 0;
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
        cfg.bff.resilience.timeout = Duration::from_secs(0);
        assert!(codes(&cfg.build_report()).contains(&"timeout_zero"));
    }

    #[test]
    fn zero_max_concurrent_rejected() {
        let mut cfg = valid_config();
        cfg.bff.resilience.max_concurrent = 0;
        assert!(codes(&cfg.build_report()).contains(&"max_concurrent_zero"));
    }

    #[test]
    fn jwt_disabled_skips_secret_checks() {
        assert!(valid_config().validate_to_result().is_ok());
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
    fn yaml_overlays_defaults() {
        let yaml = r#"
host: 127.0.0.1
port: 4001
services:
  order_service: http://orders:8080
resilience:
  timeout_ms: 7500
cache:
  redis_url: redis://localhost:6379
"#;
        let file: ConfigFile = serde_yaml::from_str(yaml).expect("yaml");
        let mut cfg = AdminBffConfig::defaults();
        overlay_file(&mut cfg, file);
        assert_eq!(cfg.bff.host, "127.0.0.1");
        assert_eq!(cfg.bff.port, 4001);
        assert_eq!(cfg.services.order_service, "http://orders:8080");
        assert_eq!(cfg.bff.resilience.timeout, Duration::from_millis(7500));
        assert_eq!(cfg.cache.redis_url, "redis://localhost:6379");
        // Unset fields keep defaults.
        assert_eq!(cfg.services.product_service, "http://localhost:50052");
    }

    #[test]
    fn yaml_rejects_unknown_fields() {
        let yaml = "host: 0.0.0.0\nnonsense: 42\n";
        let err = serde_yaml::from_str::<ConfigFile>(yaml).unwrap_err();
        assert!(err.to_string().contains("nonsense"), "{err}");
    }

    #[test]
    fn validation_aggregates_errors() {
        let mut cfg = valid_config();
        cfg.bff.port = 0;
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
