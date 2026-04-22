use std::time::Duration;

use ddd_application::{register_command_handler, register_query_handler};
use ddd_shared_kernel::AppError;

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::queries::*;
use crate::domain::entities::{Supplier, SupplierAddress, SupplierContact, SupplierDocument, SupplierProduct};

// ── CreateSupplier ────────────────────────────────────────────────────────────

register_command_handler!(CreateSupplier, AppDeps, |d: &AppDeps| {
    let supplier_repo = d.supplier_repo.clone();
    let address_repo  = d.address_repo.clone();
    let contact_repo  = d.contact_repo.clone();
    move |cmd: CreateSupplier| {
        let supplier_repo = supplier_repo.clone();
        let address_repo  = address_repo.clone();
        let contact_repo  = contact_repo.clone();
        async move {
            let supplier = Supplier::create(
                cmd.company_name, cmd.tax_identification_number, cmd.registration_number,
                cmd.email, cmd.phone, cmd.website, cmd.business_type, cmd.notes, cmd.created_by.clone(),
            );
            supplier_repo.save(&supplier).await?;

            let addr = SupplierAddress::create(
                supplier.id, cmd.address_type, cmd.address_line1, cmd.address_city,
                cmd.address_postal, cmd.address_country, None, None, true, None, None,
            );
            address_repo.save(&addr).await?;

            let contact = SupplierContact::create(
                supplier.id, cmd.contact_type, cmd.contact_first_name, cmd.contact_last_name,
                cmd.contact_email, None, None, true, None, cmd.created_by,
            );
            contact_repo.save(&contact).await?;

            Ok(supplier)
        }
    }
});

// ── UpdateSupplier ────────────────────────────────────────────────────────────

register_command_handler!(UpdateSupplier, AppDeps, |d: &AppDeps| {
    let repo = d.supplier_repo.clone();
    move |cmd: UpdateSupplier| {
        let repo = repo.clone();
        async move {
            let mut s = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Supplier {} not found", cmd.id)))?;
            s.update(
                cmd.company_name, cmd.tax_identification_number, cmd.registration_number,
                cmd.email, cmd.phone, cmd.website, cmd.business_type, cmd.years_in_business,
                cmd.notes, cmd.updated_by,
            );
            repo.save(&s).await?;
            Ok(s)
        }
    }
});

// ── ActivateSupplier ──────────────────────────────────────────────────────────

register_command_handler!(ActivateSupplier, AppDeps, |d: &AppDeps| {
    let repo = d.supplier_repo.clone();
    move |cmd: ActivateSupplier| {
        let repo = repo.clone();
        async move {
            let mut s = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Supplier {} not found", cmd.id)))?;
            s.activate(cmd.updated_by);
            repo.save(&s).await?;
            Ok(s)
        }
    }
});

// ── DeactivateSupplier ────────────────────────────────────────────────────────

register_command_handler!(DeactivateSupplier, AppDeps, |d: &AppDeps| {
    let repo = d.supplier_repo.clone();
    move |cmd: DeactivateSupplier| {
        let repo = repo.clone();
        async move {
            let mut s = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Supplier {} not found", cmd.id)))?;
            s.deactivate(cmd.updated_by);
            repo.save(&s).await?;
            Ok(s)
        }
    }
});

// ── DeleteSupplier ────────────────────────────────────────────────────────────

register_command_handler!(DeleteSupplier, AppDeps, |d: &AppDeps| {
    let repo = d.supplier_repo.clone();
    move |cmd: DeleteSupplier| {
        let repo = repo.clone();
        async move {
            repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Supplier {} not found", cmd.id)))?;
            repo.delete(cmd.id).await
        }
    }
});

// ── UpdateSupplierStatus ──────────────────────────────────────────────────────

register_command_handler!(UpdateSupplierStatus, AppDeps, |d: &AppDeps| {
    let repo = d.supplier_repo.clone();
    move |cmd: UpdateSupplierStatus| {
        let repo = repo.clone();
        async move {
            let mut s = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Supplier {} not found", cmd.id)))?;
            s.update_status(cmd.status, cmd.updated_by);
            repo.save(&s).await?;
            Ok(s)
        }
    }
});

// ── UpdateOnboardingStatus ────────────────────────────────────────────────────

register_command_handler!(UpdateOnboardingStatus, AppDeps, |d: &AppDeps| {
    let repo = d.supplier_repo.clone();
    move |cmd: UpdateOnboardingStatus| {
        let repo = repo.clone();
        async move {
            let mut s = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Supplier {} not found", cmd.id)))?;
            s.update_onboarding_status(cmd.onboarding_status, cmd.updated_by);
            repo.save(&s).await?;
            Ok(s)
        }
    }
});

// ── CreateSupplierContact ─────────────────────────────────────────────────────

register_command_handler!(CreateSupplierContact, AppDeps, |d: &AppDeps| {
    let supplier_repo = d.supplier_repo.clone();
    let contact_repo  = d.contact_repo.clone();
    move |cmd: CreateSupplierContact| {
        let supplier_repo = supplier_repo.clone();
        let contact_repo  = contact_repo.clone();
        async move {
            supplier_repo.find_by_id(cmd.supplier_id).await?
                .ok_or_else(|| AppError::not_found(format!("Supplier {} not found", cmd.supplier_id)))?;
            let contact = SupplierContact::create(
                cmd.supplier_id, crate::domain::enums::ContactType::Primary,
                cmd.first_name, cmd.last_name, cmd.email, cmd.phone,
                cmd.position, cmd.is_primary, cmd.notes, cmd.created_by,
            );
            contact_repo.save(&contact).await?;
            Ok(contact)
        }
    }
});

// ── RequestDocumentUploadUrl ──────────────────────────────────────────────────

register_command_handler!(RequestDocumentUploadUrl, AppDeps, |d: &AppDeps| {
    let supplier_repo = d.supplier_repo.clone();
    let storage       = d.blob_storage.clone();
    let bucket        = d.blob_bucket.clone();
    let ttl           = d.presign_ttl_secs;
    move |cmd: RequestDocumentUploadUrl| {
        let supplier_repo = supplier_repo.clone();
        let storage       = storage.clone();
        let bucket        = bucket.clone();
        async move {
            supplier_repo.find_by_id(cmd.supplier_id).await?
                .ok_or_else(|| AppError::not_found(format!("Supplier {} not found", cmd.supplier_id)))?;
            let ext = cmd.file_name.rsplit('.').next().unwrap_or("bin");
            let key = format!("suppliers/{}/{}.{}", cmd.supplier_id, uuid::Uuid::new_v4(), ext);
            let presigned = storage.presigned_put(&bucket, &key, &cmd.content_type, Duration::from_secs(ttl)).await
                .map_err(|e| AppError::internal(e.to_string()))?;
            Ok((presigned.url, key, presigned.expires_at.to_rfc3339()))
        }
    }
});

// ── ConfirmDocumentUpload ─────────────────────────────────────────────────────

register_command_handler!(ConfirmDocumentUpload, AppDeps, |d: &AppDeps| {
    let document_repo = d.document_repo.clone();
    move |cmd: ConfirmDocumentUpload| {
        let document_repo = document_repo.clone();
        async move {
            let doc = SupplierDocument::create(
                cmd.supplier_id, cmd.file_name, cmd.object_name,
                cmd.content_type, cmd.document_type, cmd.created_by,
            );
            document_repo.save(&doc).await?;
            Ok(doc)
        }
    }
});

// ── DeleteSupplierDocument ────────────────────────────────────────────────────

register_command_handler!(DeleteSupplierDocument, AppDeps, |d: &AppDeps| {
    let document_repo = d.document_repo.clone();
    move |cmd: DeleteSupplierDocument| {
        let document_repo = document_repo.clone();
        async move {
            let doc = document_repo.find_by_id(cmd.document_id).await?
                .ok_or_else(|| AppError::not_found(format!("Document {} not found", cmd.document_id)))?;
            if doc.supplier_id != cmd.supplier_id {
                return Err(AppError::not_found("Document does not belong to this supplier"));
            }
            document_repo.delete(cmd.document_id).await
        }
    }
});

// ── AddSupplierProduct ────────────────────────────────────────────────────────

register_command_handler!(AddSupplierProduct, AppDeps, |d: &AppDeps| {
    let supplier_repo = d.supplier_repo.clone();
    let product_repo  = d.product_repo.clone();
    move |cmd: AddSupplierProduct| {
        let supplier_repo = supplier_repo.clone();
        let product_repo  = product_repo.clone();
        async move {
            supplier_repo.find_by_id(cmd.supplier_id).await?
                .ok_or_else(|| AppError::not_found(format!("Supplier {} not found", cmd.supplier_id)))?;
            if product_repo.exists(cmd.supplier_id, cmd.product_id, cmd.variant_id).await? {
                return Err(AppError::conflict("Supplier already has this product/variant"));
            }
            let product = SupplierProduct::create(
                cmd.supplier_id, cmd.product_id, cmd.variant_id, cmd.unit_cost,
                cmd.supplier_sku, cmd.lead_time_days, cmd.min_order_quantity,
                cmd.is_preferred, cmd.created_by,
            );
            product_repo.save(&product).await?;
            Ok(product)
        }
    }
});

// ── RemoveSupplierProduct ─────────────────────────────────────────────────────

register_command_handler!(RemoveSupplierProduct, AppDeps, |d: &AppDeps| {
    let product_repo = d.product_repo.clone();
    move |cmd: RemoveSupplierProduct| {
        let product_repo = product_repo.clone();
        async move {
            let p = product_repo.find_by_id(cmd.supplier_product_id).await?
                .ok_or_else(|| AppError::not_found(format!("SupplierProduct {} not found", cmd.supplier_product_id)))?;
            if p.supplier_id != cmd.supplier_id {
                return Err(AppError::not_found("Product does not belong to this supplier"));
            }
            product_repo.delete(cmd.supplier_product_id).await
        }
    }
});

// ── Queries ───────────────────────────────────────────────────────────────────

register_query_handler!(GetSupplier, AppDeps, |d: &AppDeps| {
    let repo = d.supplier_repo.clone();
    move |q: GetSupplier| {
        let repo = repo.clone();
        async move { repo.find_by_id(q.id).await }
    }
});

register_query_handler!(ListSuppliers, AppDeps, |d: &AppDeps| {
    let repo = d.supplier_repo.clone();
    move |q: ListSuppliers| {
        let repo = repo.clone();
        async move { repo.get_paged(q.active_only, q.search.as_deref(), &q.req).await }
    }
});

register_query_handler!(GetSupplierAddresses, AppDeps, |d: &AppDeps| {
    let repo = d.address_repo.clone();
    move |q: GetSupplierAddresses| {
        let repo = repo.clone();
        async move { repo.get_by_supplier(q.supplier_id).await }
    }
});

register_query_handler!(GetSupplierContacts, AppDeps, |d: &AppDeps| {
    let repo = d.contact_repo.clone();
    move |q: GetSupplierContacts| {
        let repo = repo.clone();
        async move { repo.get_by_supplier(q.supplier_id).await }
    }
});

register_query_handler!(GetSupplierDocuments, AppDeps, |d: &AppDeps| {
    let document_repo = d.document_repo.clone();
    let storage       = d.blob_storage.clone();
    let bucket        = d.blob_bucket.clone();
    let ttl           = d.presign_ttl_secs;
    move |q: GetSupplierDocuments| {
        let document_repo = document_repo.clone();
        let storage       = storage.clone();
        let bucket        = bucket.clone();
        async move {
            let docs = document_repo.get_by_supplier(q.supplier_id).await?;
            let mut result = Vec::with_capacity(docs.len());
            for doc in docs {
                let (url, expires_at) = storage.presigned_get(&bucket, &doc.object_name, Duration::from_secs(ttl)).await
                    .map(|p| (p.url, p.expires_at.to_rfc3339()))
                    .unwrap_or_default();
                result.push((doc, url, expires_at));
            }
            Ok(result)
        }
    }
});

register_query_handler!(ListSupplierProducts, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |q: ListSupplierProducts| {
        let repo = repo.clone();
        async move { repo.get_by_supplier(q.supplier_id).await }
    }
});
