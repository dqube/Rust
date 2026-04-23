use ddd_bff::prelude::TracingInterceptor;

use crate::proto_catalog::catalog_service_client::CatalogServiceClient;

#[derive(Clone)]
pub struct CatalogClient {
    channel: tonic::transport::Channel,
}

impl CatalogClient {
    pub fn new(channel: tonic::transport::Channel) -> Self {
        Self { channel }
    }

    pub fn client(
        &self,
    ) -> CatalogServiceClient<
        tonic::service::interceptor::InterceptedService<
            tonic::transport::Channel,
            TracingInterceptor,
        >,
    > {
        CatalogServiceClient::with_interceptor(self.channel.clone(), TracingInterceptor)
    }
}
