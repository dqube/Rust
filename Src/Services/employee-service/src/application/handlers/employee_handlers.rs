use std::time::Duration;

use ddd_application::{register_command_handler, register_query_handler};
use ddd_shared_kernel::AppError;

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::queries::*;
use crate::domain::entities::Employee;

// ── CreateEmployee ────────────────────────────────────────────────────────────

register_command_handler!(CreateEmployee, AppDeps, |d: &AppDeps| {
    let repo    = d.employee_repo.clone();
    move |cmd: CreateEmployee| {
        let repo = repo.clone();
        async move {
            if repo.user_id_exists(cmd.user_id).await? {
                return Err(AppError::conflict("Employee already exists for this user_id"));
            }
            if repo.email_exists(&cmd.email).await? {
                return Err(AppError::conflict("Email already in use"));
            }
            let employee = Employee::create(
                cmd.user_id, cmd.first_name, cmd.last_name, cmd.middle_name,
                cmd.date_of_birth, cmd.gender, cmd.email, cmd.personal_email,
                cmd.phone, cmd.mobile, cmd.department_id, cmd.designation_id,
                cmd.manager_id, cmd.employment_type, cmd.date_of_joining,
                cmd.salary, cmd.bank_account_number, cmd.bank_ifsc_code, cmd.bank_name,
            );
            repo.save(&employee).await?;
            Ok(employee)
        }
    }
});

// ── UpdateEmployee ────────────────────────────────────────────────────────────

register_command_handler!(UpdateEmployee, AppDeps, |d: &AppDeps| {
    let repo = d.employee_repo.clone();
    move |cmd: UpdateEmployee| {
        let repo = repo.clone();
        async move {
            let mut emp = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Employee {} not found", cmd.id)))?;
            emp.update(
                cmd.first_name, cmd.last_name, cmd.middle_name, cmd.date_of_birth,
                cmd.gender, cmd.email, cmd.personal_email, cmd.phone, cmd.mobile,
                cmd.department_id, cmd.designation_id, cmd.manager_id,
                cmd.employment_type, cmd.salary, cmd.bank_account_number,
                cmd.bank_ifsc_code, cmd.bank_name,
            );
            repo.save(&emp).await?;
            Ok(emp)
        }
    }
});

// ── TerminateEmployee ─────────────────────────────────────────────────────────

register_command_handler!(TerminateEmployee, AppDeps, |d: &AppDeps| {
    let repo = d.employee_repo.clone();
    move |cmd: TerminateEmployee| {
        let repo = repo.clone();
        async move {
            let mut emp = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Employee {} not found", cmd.id)))?;
            emp.terminate(cmd.date_of_leaving);
            repo.save(&emp).await?;
            Ok(emp)
        }
    }
});

// ── ReactivateEmployee ────────────────────────────────────────────────────────

register_command_handler!(ReactivateEmployee, AppDeps, |d: &AppDeps| {
    let repo = d.employee_repo.clone();
    move |cmd: ReactivateEmployee| {
        let repo = repo.clone();
        async move {
            let mut emp = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Employee {} not found", cmd.id)))?;
            emp.reactivate();
            repo.save(&emp).await?;
            Ok(emp)
        }
    }
});

// ── AssignEmployeeToStore ─────────────────────────────────────────────────────

register_command_handler!(AssignEmployeeToStore, AppDeps, |d: &AppDeps| {
    let repo = d.employee_repo.clone();
    move |cmd: AssignEmployeeToStore| {
        let repo = repo.clone();
        async move {
            let mut emp = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Employee {} not found", cmd.id)))?;
            emp.assign_store(cmd.store_id);
            repo.save(&emp).await?;
            Ok(emp)
        }
    }
});

// ── RequestAvatarUploadUrl ────────────────────────────────────────────────────

register_command_handler!(RequestAvatarUploadUrl, AppDeps, |d: &AppDeps| {
    let repo    = d.employee_repo.clone();
    let storage = d.blob_storage.clone();
    let bucket  = d.blob_bucket.clone();
    let ttl     = d.presign_ttl_secs;
    move |cmd: RequestAvatarUploadUrl| {
        let repo    = repo.clone();
        let storage = storage.clone();
        let bucket  = bucket.clone();
        async move {
            repo.find_by_id(cmd.employee_id).await?
                .ok_or_else(|| AppError::not_found(format!("Employee {} not found", cmd.employee_id)))?;
            let key = format!("employees/{}/avatar/{}", cmd.employee_id, uuid::Uuid::new_v4());
            let presigned = storage.presigned_put(&bucket, &key, &cmd.content_type, Duration::from_secs(ttl)).await
                .map_err(|e| AppError::internal(e.to_string()))?;
            let expires_at = presigned.expires_at.to_rfc3339();
            Ok((presigned.url, key, expires_at))
        }
    }
});

// ── ConfirmAvatarUpload ───────────────────────────────────────────────────────

register_command_handler!(ConfirmAvatarUpload, AppDeps, |d: &AppDeps| {
    let repo = d.employee_repo.clone();
    move |cmd: ConfirmAvatarUpload| {
        let repo = repo.clone();
        async move {
            let mut emp = repo.find_by_id(cmd.employee_id).await?
                .ok_or_else(|| AppError::not_found(format!("Employee {} not found", cmd.employee_id)))?;
            emp.set_avatar(cmd.object_name);
            repo.save(&emp).await?;
            Ok(emp)
        }
    }
});

// ── DeleteAvatar ──────────────────────────────────────────────────────────────

register_command_handler!(DeleteAvatar, AppDeps, |d: &AppDeps| {
    let repo = d.employee_repo.clone();
    move |cmd: DeleteAvatar| {
        let repo = repo.clone();
        async move {
            let mut emp = repo.find_by_id(cmd.employee_id).await?
                .ok_or_else(|| AppError::not_found(format!("Employee {} not found", cmd.employee_id)))?;
            emp.clear_avatar();
            repo.save(&emp).await?;
            Ok(emp)
        }
    }
});

// ── GetEmployee ───────────────────────────────────────────────────────────────

register_query_handler!(GetEmployee, AppDeps, |d: &AppDeps| {
    let repo = d.employee_repo.clone();
    move |q: GetEmployee| {
        let repo = repo.clone();
        async move { repo.find_by_id(q.id).await }
    }
});

register_query_handler!(GetEmployeeByUserId, AppDeps, |d: &AppDeps| {
    let repo = d.employee_repo.clone();
    move |q: GetEmployeeByUserId| {
        let repo = repo.clone();
        async move { repo.find_by_user_id(q.user_id).await }
    }
});

register_query_handler!(GetEmployeeByCode, AppDeps, |d: &AppDeps| {
    let repo = d.employee_repo.clone();
    move |q: GetEmployeeByCode| {
        let repo = repo.clone();
        async move { repo.find_by_code(&q.code).await }
    }
});

register_query_handler!(ListEmployees, AppDeps, |d: &AppDeps| {
    let repo = d.employee_repo.clone();
    move |q: ListEmployees| {
        let repo = repo.clone();
        async move {
            repo.list_paged(
                q.status_filter.as_deref(),
                q.department_id,
                q.search.as_deref(),
                &q.req,
            ).await
        }
    }
});

register_query_handler!(GetAvatarUrl, AppDeps, |d: &AppDeps| {
    let repo    = d.employee_repo.clone();
    let storage = d.blob_storage.clone();
    let bucket  = d.blob_bucket.clone();
    let ttl     = d.presign_ttl_secs;
    move |q: GetAvatarUrl| {
        let repo    = repo.clone();
        let storage = storage.clone();
        let bucket  = bucket.clone();
        async move {
            let emp = repo.find_by_id(q.employee_id).await?
                .ok_or_else(|| AppError::not_found(format!("Employee {} not found", q.employee_id)))?;
            let key = emp.avatar_object_name
                .ok_or_else(|| AppError::not_found("Employee has no avatar"))?;
            let presigned = storage.presigned_get(&bucket, &key, Duration::from_secs(ttl)).await
                .map_err(|e| AppError::internal(e.to_string()))?;
            Ok((presigned.url, presigned.expires_at.to_rfc3339()))
        }
    }
});
