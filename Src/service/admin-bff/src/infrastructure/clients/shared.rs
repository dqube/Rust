use ddd_bff::prelude::TracingInterceptor;

use crate::proto_shared::shared_service_client::SharedServiceClient;

/// Wraps a tonic channel for constructing shared-service gRPC clients.
#[derive(Clone)]
pub struct SharedClient {
    channel: tonic::transport::Channel,
}

impl SharedClient {
    pub fn new(channel: tonic::transport::Channel) -> Self {
        Self { channel }
    }

    pub fn client(
        &self,
    ) -> SharedServiceClient<
        tonic::service::interceptor::InterceptedService<
            tonic::transport::Channel,
            TracingInterceptor,
        >,
    > {
        SharedServiceClient::with_interceptor(self.channel.clone(), TracingInterceptor)
    }
}
