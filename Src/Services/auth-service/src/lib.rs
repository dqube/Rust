//! Auth Service — library root.

pub mod api;
pub mod application;
pub mod domain;
pub mod infrastructure;

pub mod proto {
    tonic::include_proto!("auth.v1");
}
