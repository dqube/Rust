use ddd_application::{register_command_handler, register_query_handler};
use ddd_shared_kernel::AppError;

use crate::application::commands::{CreateDepartment, UpdateDepartment};
use crate::application::deps::AppDeps;
use crate::application::queries::{GetDepartment, ListDepartments};
use crate::domain::entities::Department;

register_command_handler!(CreateDepartment, AppDeps, |d: &AppDeps| {
    let repo = d.department_repo.clone();
    move |cmd: CreateDepartment| {
        let repo = repo.clone();
        async move {
            if let Some(ref code) = cmd.department_code {
                if !code.is_empty() && repo.code_exists(code).await? {
                    return Err(AppError::conflict(format!("Department code '{}' already exists", code)));
                }
            }
            let dept = Department::create(
                cmd.department_name,
                cmd.department_code,
                cmd.parent_department_id,
                cmd.head_of_department_id,
            );
            repo.save(&dept).await?;
            Ok(dept)
        }
    }
});

register_command_handler!(UpdateDepartment, AppDeps, |d: &AppDeps| {
    let repo = d.department_repo.clone();
    move |cmd: UpdateDepartment| {
        let repo = repo.clone();
        async move {
            let mut dept = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Department {} not found", cmd.id)))?;
            dept.department_name = cmd.department_name;
            dept.department_code = cmd.department_code;
            dept.updated_at = Some(chrono::Utc::now());
            repo.save(&dept).await?;
            Ok(dept)
        }
    }
});

register_query_handler!(GetDepartment, AppDeps, |d: &AppDeps| {
    let repo = d.department_repo.clone();
    move |q: GetDepartment| {
        let repo = repo.clone();
        async move { repo.find_by_id(q.id).await }
    }
});

register_query_handler!(ListDepartments, AppDeps, |d: &AppDeps| {
    let repo = d.department_repo.clone();
    move |_q: ListDepartments| {
        let repo = repo.clone();
        async move { repo.get_all().await }
    }
});
