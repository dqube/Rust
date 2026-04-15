//! Generic configuration validation.
//!
//! Provides a reusable bootstrap-time validator for any deserialized config
//! struct. The shape is deliberately close to a linter: callers collect
//! issues into a [`Report`], then convert the report to an [`AppError`] (hard
//! failure) or log warnings and keep running.
//!
//! ## Why not `validator`?
//!
//! The `validator` crate targets field-level rules on DTOs. Config files need
//! cross-field, cross-collection, graph-ish checks — e.g., *"every route
//! references an upstream that exists, no route pair claims the same
//! (path, method), and unreferenced upstreams emit a warning"*. That flavour
//! fits poorly into per-field attributes, so this module provides a free-form
//! report-building API instead.
//!
//! ## Typical usage
//!
//! (requires the `config-validation` feature)
//!
//! ```no_run
//! # #[cfg(feature = "config-validation")]
//! # fn demo() -> Result<(), ddd_shared_kernel::AppError> {
//! use ddd_shared_kernel::config_validation::{Report, Validate, load_yaml_validated};
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct MyConfig { port: u16, hosts: Vec<String> }
//!
//! impl Validate for MyConfig {
//!     fn validate(&self, report: &mut Report) {
//!         if self.port == 0 {
//!             report.error("port", "port_zero", "port must be > 0");
//!         }
//!         if self.hosts.is_empty() {
//!             report.error("hosts", "empty", "at least one host required");
//!         }
//!     }
//! }
//!
//! let cfg: MyConfig = load_yaml_validated("config.yaml")?;
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "config-validation")]
use std::path::Path;

use crate::error::{AppError, AppResult, ValidationFieldError};

// ─── Types ───────────────────────────────────────────────────────────────────

/// A single configuration issue (error or warning).
///
/// `path` points at the offending field using dotted / indexed notation
/// (`routes[2].target.upstream`). `code` is a machine-readable slug
/// (`unknown_upstream`, `duplicate_id`, `invalid_url`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigIssue {
    /// Dotted/indexed path to the offending field.
    pub path: String,
    /// Machine-readable issue code.
    pub code: String,
    /// Human-readable description.
    pub message: String,
}

/// Severity of a [`ConfigIssue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Hard failure — bootstrap should abort.
    Error,
    /// Advisory — bootstrap should continue but log.
    Warning,
}

/// Accumulated report of issues found while validating a config.
///
/// `Validate` implementations call `report.error(...)` / `report.warn(...)`
/// freely; the caller drains the report with [`Report::into_result`] or
/// inspects it directly.
#[derive(Debug, Clone, Default)]
pub struct Report {
    errors: Vec<ConfigIssue>,
    warnings: Vec<ConfigIssue>,
}

impl Report {
    /// Create an empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an error.
    pub fn error(
        &mut self,
        path: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.errors.push(ConfigIssue {
            path: path.into(),
            code: code.into(),
            message: message.into(),
        });
    }

    /// Record a warning.
    pub fn warn(
        &mut self,
        path: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.warnings.push(ConfigIssue {
            path: path.into(),
            code: code.into(),
            message: message.into(),
        });
    }

    /// All errors in insertion order.
    pub fn errors(&self) -> &[ConfigIssue] {
        &self.errors
    }

    /// All warnings in insertion order.
    pub fn warnings(&self) -> &[ConfigIssue] {
        &self.warnings
    }

    /// `true` if no errors were recorded (warnings don't count).
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Total issue count across errors + warnings.
    pub fn total(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }

    /// Emit every warning to `tracing` at `warn` level, leaving errors
    /// untouched. Useful when callers want warnings surfaced even though
    /// bootstrap continues.
    pub fn log_warnings(&self, context: &str) {
        for w in &self.warnings {
            tracing::warn!(
                target: "config_validation",
                context,
                path = %w.path,
                code = %w.code,
                "{}",
                w.message
            );
        }
    }

    /// Log all errors at `error` level.
    pub fn log_errors(&self, context: &str) {
        for e in &self.errors {
            tracing::error!(
                target: "config_validation",
                context,
                path = %e.path,
                code = %e.code,
                "{}",
                e.message
            );
        }
    }

    /// Consume the report and convert to an [`AppResult`]. Returns
    /// [`AppError::ValidationBatch`] if any errors were recorded. Warnings
    /// are dropped — call [`log_warnings`](Self::log_warnings) first if you
    /// care about them.
    pub fn into_result(self) -> AppResult<()> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            let fields = self
                .errors
                .into_iter()
                .map(|e| ValidationFieldError::with_code(e.path, e.message, e.code))
                .collect();
            Err(AppError::validation_batch(fields))
        }
    }
}

// ─── Trait ───────────────────────────────────────────────────────────────────

/// Something that can check itself for configuration issues.
///
/// Implementations should record every issue they find instead of bailing on
/// the first one — bootstrap-time validation is most useful when it surfaces
/// all problems in a single pass so operators can fix them together.
pub trait Validate {
    /// Append every detected issue to `report`.
    fn validate(&self, report: &mut Report);

    /// Convenience: build a fresh report, run [`validate`], and return it.
    fn build_report(&self) -> Report {
        let mut r = Report::new();
        self.validate(&mut r);
        r
    }

    /// Convenience: run validation and convert to [`AppResult`], discarding
    /// warnings. Equivalent to `self.build_report().into_result()`.
    fn validate_to_result(&self) -> AppResult<()> {
        self.build_report().into_result()
    }
}

// ─── YAML loader ─────────────────────────────────────────────────────────────

/// Load a YAML file from disk, deserialize it into `T`, then validate it.
///
/// On success returns the parsed value. On failure returns:
/// - [`AppError::Internal`] when the file can't be read,
/// - [`AppError::Serialization`] when YAML parsing fails,
/// - [`AppError::ValidationBatch`] when one or more [`Validate`] errors were
///   reported (warnings are logged via `tracing` and discarded).
///
/// The `context` used for warning logs is the file path.
#[cfg(feature = "config-validation")]
pub fn load_yaml_validated<T>(path: impl AsRef<Path>) -> AppResult<T>
where
    T: serde::de::DeserializeOwned + Validate,
{
    let path = path.as_ref();
    let src = std::fs::read_to_string(path).map_err(|e| AppError::Internal {
        message: format!("failed to read config `{}`: {}", path.display(), e),
    })?;
    from_yaml_validated(&src, path.display().to_string())
}

/// Parse YAML from a string and validate it. Same semantics as
/// [`load_yaml_validated`] but without touching the filesystem. `context` is
/// any label (file path, test name) that appears in warning/error logs.
#[cfg(feature = "config-validation")]
pub fn from_yaml_validated<T>(src: &str, context: impl Into<String>) -> AppResult<T>
where
    T: serde::de::DeserializeOwned + Validate,
{
    let context = context.into();
    let value: T = serde_yaml::from_str(src).map_err(|e| AppError::Serialization {
        message: format!("failed to parse `{context}`: {e}"),
    })?;
    let report = value.build_report();
    report.log_warnings(&context);
    if !report.is_ok() {
        report.log_errors(&context);
    }
    report.into_result()?;
    Ok(value)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct Demo {
        port: u16,
        hosts: Vec<String>,
    }

    impl Validate for Demo {
        fn validate(&self, report: &mut Report) {
            if self.port == 0 {
                report.error("port", "port_zero", "must be > 0");
            }
            if self.hosts.is_empty() {
                report.error("hosts", "empty", "at least one host required");
            }
            if self.hosts.len() > 100 {
                report.warn("hosts", "too_many", "unusually long host list");
            }
        }
    }

    #[test]
    fn empty_report_is_ok() {
        let r = Report::new();
        assert!(r.is_ok());
        assert!(r.into_result().is_ok());
    }

    #[test]
    fn report_collects_errors_in_order() {
        let d = Demo {
            port: 0,
            hosts: vec![],
        };
        let r = d.build_report();
        assert_eq!(r.errors().len(), 2);
        assert_eq!(r.errors()[0].code, "port_zero");
        assert_eq!(r.errors()[1].code, "empty");
        assert!(!r.is_ok());
    }

    #[test]
    fn into_result_builds_validation_batch() {
        let d = Demo {
            port: 0,
            hosts: vec!["a".into()],
        };
        match d.validate_to_result() {
            Err(AppError::ValidationBatch { errors }) => {
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].field, "port");
                assert_eq!(errors[0].code, "port_zero");
            }
            other => panic!("expected ValidationBatch, got {other:?}"),
        }
    }

    #[test]
    fn warnings_do_not_fail_result() {
        let d = Demo {
            port: 8080,
            hosts: (0..150).map(|i| format!("h{i}")).collect(),
        };
        let r = d.build_report();
        assert!(r.is_ok());
        assert_eq!(r.warnings().len(), 1);
        assert!(d.validate_to_result().is_ok());
    }

    #[cfg(feature = "config-validation")]
    #[test]
    fn from_yaml_validated_roundtrips() {
        #[derive(Debug, serde::Deserialize)]
        struct Y {
            n: u32,
        }
        impl Validate for Y {
            fn validate(&self, r: &mut Report) {
                if self.n == 0 {
                    r.error("n", "zero", "n must be > 0");
                }
            }
        }
        let ok: Y = from_yaml_validated("n: 5", "test").unwrap();
        assert_eq!(ok.n, 5);

        let err = from_yaml_validated::<Y>("n: 0", "test").unwrap_err();
        assert!(matches!(err, AppError::ValidationBatch { .. }));
    }
}
