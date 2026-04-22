use ddd_bff::prelude::TracingInterceptor;
use crate::proto::product_service_client::ProductServiceClient;

/// Wraps a tonic channel for constructing product-service gRPC clients.
#[derive(Clone)]
pub struct ProductClient {
    channel: tonic::transport::Channel,
}

impl ProductClient {
    pub fn new(channel: tonic::transport::Channel) -> Self {
        Self { channel }
    }

    pub fn client(
        &self,
    ) -> ProductServiceClient<
        tonic::service::interceptor::InterceptedService<tonic::transport::Channel, TracingInterceptor>,
    > {
        ProductServiceClient::with_interceptor(self.channel.clone(), TracingInterceptor)
    }
}
