//! gRPC streaming helpers.

use futures::Stream;
use std::pin::Pin;
use tonic::Status;

/// Boxed stream type used for server-streaming gRPC responses.
pub type TonicStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

/// Build a [`TonicStream`] from an iterator.
pub fn stream_from_iter<T, I>(iter: I) -> TonicStream<T>
where
    T: Send + 'static,
    I: IntoIterator<Item = T> + Send + 'static,
    I::IntoIter: Send,
{
    let stream = futures::stream::iter(iter.into_iter().map(Ok));
    Box::pin(stream)
}

/// Build a [`TonicStream`] from a [`tokio::sync::mpsc::Receiver`].
pub fn stream_from_rx<T: Send + 'static>(
    rx: tokio::sync::mpsc::Receiver<Result<T, Status>>,
) -> TonicStream<T> {
    Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
}
