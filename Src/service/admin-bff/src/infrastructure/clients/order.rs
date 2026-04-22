use ddd_bff::prelude::TracingInterceptor;
use crate::proto_order::order_service_client::OrderServiceClient;

/// Wraps a tonic channel for constructing order-service gRPC clients.
#[derive(Clone)]
pub struct OrderClient {
    channel: tonic::transport::Channel,
}

impl OrderClient {
    pub fn new(channel: tonic::transport::Channel) -> Self {
        Self { channel }
    }

    pub fn client(
        &self,
    ) -> OrderServiceClient<
        tonic::service::interceptor::InterceptedService<tonic::transport::Channel, TracingInterceptor>,
    > {
        OrderServiceClient::with_interceptor(self.channel.clone(), TracingInterceptor)
    }
}
