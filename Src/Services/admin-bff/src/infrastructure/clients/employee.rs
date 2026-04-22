use ddd_bff::prelude::TracingInterceptor;

use crate::proto_employee::employee_service_client::EmployeeServiceClient;

#[derive(Clone)]
pub struct EmployeeClient {
    channel: tonic::transport::Channel,
}

impl EmployeeClient {
    pub fn new(channel: tonic::transport::Channel) -> Self {
        Self { channel }
    }

    pub fn client(
        &self,
    ) -> EmployeeServiceClient<
        tonic::service::interceptor::InterceptedService<
            tonic::transport::Channel,
            TracingInterceptor,
        >,
    > {
        EmployeeServiceClient::with_interceptor(self.channel.clone(), TracingInterceptor)
    }
}
