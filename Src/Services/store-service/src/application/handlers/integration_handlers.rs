use ddd_shared_kernel::AppError;
use tracing::info;

use crate::application::integration_events::EmployeeStoreAssignedIntegrationEvent;

pub async fn handle_employee_store_assigned(
    evt: EmployeeStoreAssignedIntegrationEvent,
) -> Result<(), AppError> {
    info!(
        employee_id   = %evt.employee_id,
        store_id      = evt.store_id,
        employee_code = %evt.employee_code,
        "Employee assigned to store (roster log only)"
    );
    Ok(())
}
