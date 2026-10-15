use super::*;
use loco_rs::model::TenantQueryExt;

pub async fn list(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    authorize(tenant).await?;
    let rows = membership::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .order_by_asc(membership::Column::Id)
        .all(tenant.transaction())
        .await?;
    Ok(json!({"data":values(tenant,&rows).await?}))
}
async fn values(
    tenant: &TenantTransaction,
    rows: &[membership::Model],
) -> Result<Vec<Value>, OperationError> {
    let accounts = crate::models::person_identity::household_accounts(tenant).await?;
    let names = crate::models::person_identity::display_names(
        tenant,
        rows.iter()
            .map(|row| crate::models::ids::PersonId::from_database(row.person_id)),
    )
    .await?;
    Ok(rows.iter().map(|row| {
        let person_id = crate::models::ids::PersonId::from_database(row.person_id);
        json!({"id":row.id,"account_id":accounts.get(&person_id).map(|account|account.id),"person_id":person_id,"person_name":names.get(&person_id),"role":row.role,"status":row.status,"permissions_version":row.permissions_version,"joined_at":row.joined_at.and_utc().to_rfc3339()})
    }).collect())
}

pub async fn change(
    tenant: &TenantTransaction,
    id: i64,
    attributes: Value,
    revoke: bool,
    provenance: Option<&CredentialProvenance>,
) -> Result<Value, OperationError> {
    authorize(tenant).await?;
    change_record(tenant, id, attributes, revoke, provenance).await
}
async fn change_record(
    tenant: &TenantTransaction,
    id: i64,
    attributes: Value,
    revoke: bool,
    provenance: Option<&CredentialProvenance>,
) -> Result<Value, OperationError> {
    let current = membership::Entity::find_by_id(id)
        .in_tenant(tenant.scope().household_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    let attributes = if revoke {
        json!({"status":"revoked"})
    } else {
        attributes
    };
    let fields = fields(&attributes, &["role", "status"], "household_membership")?;
    let role = fields
        .get("role")
        .map(Value::as_str)
        .unwrap_or(Some(&current.role))
        .filter(|role| matches!(*role, "owner" | "administrator" | "member"))
        .ok_or_else(|| invalid("role", "is invalid"))?;
    let status = if revoke {
        "revoked"
    } else {
        fields
            .get("status")
            .map(Value::as_str)
            .unwrap_or(Some(&current.status))
            .filter(|status| matches!(*status, "active" | "suspended" | "revoked"))
            .ok_or_else(|| invalid("status", "is invalid"))?
    };
    let mut body = values(tenant, std::slice::from_ref(&current))
        .await?
        .remove(0);
    use sea_orm::{ConnectionTrait, DbBackend, FromQueryResult, Statement};
    let row = tenant
        .transaction()
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.change_household_membership($1,$2,$3,$4,$5)",
            [
                id.into(),
                role.into(),
                status.into(),
                tenant.scope().request_id.clone().into(),
                crate::models::audit_context::household(tenant, provenance).into(),
            ],
        ))
        .await
        .map_err(mutation_error)?
        .ok_or(OperationError::NotFound)?;
    let next = membership::Model::from_query_result(&row, "")?;
    body["role"] = json!(next.role);
    body["status"] = json!(next.status);
    body["permissions_version"] = json!(next.permissions_version);
    Ok(json!({"data":body}))
}

fn mutation_error(error: sea_orm::DbErr) -> OperationError {
    if let sea_orm::DbErr::Exec(sea_orm::RuntimeErr::SqlxError(ref error))
    | sea_orm::DbErr::Query(sea_orm::RuntimeErr::SqlxError(ref error)) = error
        && let Some(database) = error.as_database_error()
    {
        match database.code().as_deref() {
            Some("42501") => return OperationError::Forbidden,
            Some("23514") => return invalid("base", "Participation change is not allowed"),
            _ => {}
        }
    }
    OperationError::from(error)
}
