//! NATS messaging adapters.

pub mod event_serializer;
pub mod jetstream_publisher;
pub mod jetstream_subscriber;
pub mod nats_publisher;
pub mod nats_subscriber;

pub use event_serializer::{deserialize_event, serialize_event};
pub use jetstream_publisher::{
    stream_name_for, JetStreamPublisher, DEFAULT_STREAM_MAX_AGE,
};
pub use jetstream_subscriber::{
    JetStreamSubscriber, JetStreamSubscriberConfig, DEFAULT_ACK_WAIT, DEFAULT_MAX_DELIVER,
};
pub use nats_publisher::NatsPublisher;
pub use nats_subscriber::NatsSubscriber;
