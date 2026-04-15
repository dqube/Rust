//! NATS messaging adapters.

pub mod event_serializer;
pub mod nats_publisher;
pub mod nats_subscriber;

pub use event_serializer::{deserialize_event, serialize_event};
pub use nats_publisher::NatsPublisher;
pub use nats_subscriber::NatsSubscriber;
