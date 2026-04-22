use ddd_bff::prelude::TracingInterceptor;

use crate::proto_supplier::supplier_service_client::SupplierServiceClient;

#[derive(Clone)]
pub struct SupplierClient {
    channel: tonic::transport::Channel,
}

impl SupplierClient {
    pub fn new(channel: tonic::transport::Channel) -> Self {
        Self { channel }
    }

    pub fn client(
        &self,
    ) -> SupplierServiceClient<
        tonic::service::interceptor::InterceptedService<
            tonic::transport::Channel,
            TracingInterceptor,
        >,
    > {
        SupplierServiceClient::with_interceptor(self.channel.clone(), TracingInterceptor)
    }
}
