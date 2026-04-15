//! BFF edge — routing, upstream selection, and HTTP service.

pub mod metrics_server;
pub mod observability;
pub mod path_template;
pub mod route_config;
// Activates `impl Validate for RouteConfigFile` and the
// `RouteConfigFile::load_validated` / `from_yaml_validated` constructors.
mod route_config_validate;
pub mod router;
pub mod server;
pub mod service;
pub mod shutdown;
pub mod upstream;

pub use observability::{record_upstream, RequestObs, REQUEST_ID_HEADER, UNKNOWN_ROUTE};
pub use route_config::{
    Binding, BindingSource, GrpcTarget, MatchDef, RouteConfigFile, RouteDef, TargetDef,
    UpstreamConfig,
};
pub use router::{CompiledRoute, RouteMatch, Router};
pub use service::{BffEdge, EdgeState};
pub use shutdown::{drain_with_timeout, install_signal_handler, wait_for_shutdown_signal};
pub use upstream::{Upstream, UpstreamRegistry};
