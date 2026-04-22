use ddd_bff::prelude::TracingInterceptor;

use crate::proto_auth::auth_service_client::AuthServiceClient;

/// Wraps a tonic channel for constructing auth-service gRPC clients.
#[derive(Clone)]
pub struct AuthClient {
    channel: tonic::transport::Channel,
}

impl AuthClient {
    pub fn new(channel: tonic::transport::Channel) -> Self {
        Self { channel }
    }

    pub fn client(
        &self,
    ) -> AuthServiceClient<
        tonic::service::interceptor::InterceptedService<
            tonic::transport::Channel,
            TracingInterceptor,
        >,
    > {
        AuthServiceClient::with_interceptor(self.channel.clone(), TracingInterceptor)
    }
}
