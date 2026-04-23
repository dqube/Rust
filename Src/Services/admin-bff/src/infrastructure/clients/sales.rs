use ddd_bff::prelude::TracingInterceptor;

use crate::proto_sales::sales_service_client::SalesServiceClient;

#[derive(Clone)]
pub struct SalesClient {
    channel: tonic::transport::Channel,
}

impl SalesClient {
    pub fn new(channel: tonic::transport::Channel) -> Self {
        Self { channel }
    }

    pub fn client(
        &self,
    ) -> SalesServiceClient<
        tonic::service::interceptor::InterceptedService<
            tonic::transport::Channel,
            TracingInterceptor,
        >,
    > {
        SalesServiceClient::with_interceptor(self.channel.clone(), TracingInterceptor)
    }
}
