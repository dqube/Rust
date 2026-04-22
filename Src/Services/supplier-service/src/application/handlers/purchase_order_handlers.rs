use ddd_application::{register_command_handler, register_query_handler};
use ddd_shared_kernel::AppError;

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::queries::*;
use crate::domain::entities::PurchaseOrder;

register_command_handler!(CreatePurchaseOrder, AppDeps, |d: &AppDeps| {
    let supplier_repo = d.supplier_repo.clone();
    let order_repo    = d.order_repo.clone();
    move |cmd: CreatePurchaseOrder| {
        let supplier_repo = supplier_repo.clone();
        let order_repo    = order_repo.clone();
        async move {
            supplier_repo.find_by_id(cmd.supplier_id).await?
                .ok_or_else(|| AppError::not_found(format!("Supplier {} not found", cmd.supplier_id)))?;
            let mut order = PurchaseOrder::create(
                cmd.supplier_id, cmd.store_id, cmd.expected_date,
                cmd.shipping_address_id, cmd.contact_person_id, cmd.created_by,
            );
            for d in cmd.order_details {
                order.add_detail(d.product_id, d.quantity, d.unit_cost, None);
            }
            order_repo.save(&order).await?;
            Ok(order)
        }
    }
});

register_command_handler!(SubmitPurchaseOrder, AppDeps, |d: &AppDeps| {
    let repo = d.order_repo.clone();
    move |cmd: SubmitPurchaseOrder| {
        let repo = repo.clone();
        async move {
            let mut order = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("PurchaseOrder {} not found", cmd.id)))?;
            order.submit(cmd.updated_by).map_err(|e| AppError::conflict(e))?;
            repo.save(&order).await?;
            Ok(order)
        }
    }
});

register_command_handler!(CancelPurchaseOrder, AppDeps, |d: &AppDeps| {
    let repo = d.order_repo.clone();
    move |cmd: CancelPurchaseOrder| {
        let repo = repo.clone();
        async move {
            let mut order = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("PurchaseOrder {} not found", cmd.id)))?;
            order.cancel(cmd.updated_by).map_err(|e| AppError::conflict(e))?;
            repo.save(&order).await?;
            Ok(order)
        }
    }
});

register_query_handler!(GetPurchaseOrder, AppDeps, |d: &AppDeps| {
    let repo = d.order_repo.clone();
    move |q: GetPurchaseOrder| {
        let repo = repo.clone();
        async move { repo.find_by_id(q.id).await }
    }
});

register_query_handler!(ListPurchaseOrders, AppDeps, |d: &AppDeps| {
    let repo = d.order_repo.clone();
    move |q: ListPurchaseOrders| {
        let repo = repo.clone();
        async move {
            repo.get_filtered(q.supplier_id, q.store_id, q.status.as_deref(), q.from_date, q.to_date).await
        }
    }
});
