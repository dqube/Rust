use ddd_bff::prelude::TracingInterceptor;

use crate::proto_customer::customer_service_client::CustomerServiceClient;

/// Wraps a tonic channel for constructing customer-service gRPC clients.
#[derive(Clone)]
pub struct CustomerClient {
    channel: tonic::transport::Channel,
}

impl CustomerClient {
    pub fn new(channel: tonic::transport::Channel) -> Self {
        Self { channel }
    }

    pub fn client(
        &self,
    ) -> CustomerServiceClient<
        tonic::service::interceptor::InterceptedService<
            tonic::transport::Channel,
            TracingInterceptor,
        >,
    > {
        CustomerServiceClient::with_interceptor(self.channel.clone(), TracingInterceptor)
    }
}
