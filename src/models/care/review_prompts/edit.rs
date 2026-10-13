use crate::models::{
    access::{self, TenantTransaction},
    care::sync,
    entities::review_prompt,
    errors::OperationError,
};
use chrono::Utc;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};

fn conflict(code: &str, message: &str) -> OperationError {
    OperationError::Conflict {
        code: code.to_owned(),
        details: json!({"error":message}),
    }
}

async fn authorised(
    tenant: &TenantTransaction,
    id: &str,
) -> Result<review_prompt::Model, OperationError> {
    access::recheck(tenant).await?;
    let context = sync::review_prompts::AuthContext { tenant };
    if !sync::review_prompts::actor_adult(tenant.transaction(), &context).await? {
        return Err(OperationError::Forbidden);
    }
    let record = sync::review_prompts::visible_prompt(tenant.transaction(), &context, id)
        .await?
        .ok_or(OperationError::NotFound)?;
    if !sync::review_prompts::person_access(tenant.transaction(), &context, record.person_id, true)
        .await?
    {
        return Err(OperationError::Forbidden);
    }
    Ok(record)
}

pub async fn acquire(tenant: &TenantTransaction, id: &str) -> Result<Value, OperationError> {
    sync::lock(tenant).await?;
    let record = authorised(tenant, id).await?;
    super::reconcile_person(tenant, record.person_id).await?;
    let record = authorised(tenant, id).await?;
    let token = uuid::Uuid::new_v4().to_string();
    let row = tenant.transaction().query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO medication_review_edit_leases (prompt_id,household_id,membership_id,token,expires_at) VALUES ($1,$2,$3,$4::uuid,timezone('UTC',clock_timestamp())+interval '5 minutes') ON CONFLICT (prompt_id) DO UPDATE SET membership_id=EXCLUDED.membership_id,token=EXCLUDED.token,expires_at=EXCLUDED.expires_at,updated_at=timezone('UTC',clock_timestamp()) WHERE medication_review_edit_leases.expires_at<=timezone('UTC',clock_timestamp()) RETURNING token::text AS token,expires_at",
        [record.id.into(), tenant.scope().household_id.into(), tenant.membership().id.into(), token.into()],
    )).await?;
    let Some(row) = row else {
        return Err(conflict(
            "review_locked",
            "This review is being edited by someone else",
        ));
    };
    let token: String = row.try_get("", "token")?;
    let expires_at: chrono::NaiveDateTime = row.try_get("", "expires_at")?;
    Ok(
        json!({"data":{"token":token,"etag":sync::review_prompts::tag(&record),"expires_at":expires_at.and_utc().to_rfc3339()}}),
    )
}

pub async fn renew(
    tenant: &TenantTransaction,
    id: &str,
    token: &str,
) -> Result<Value, OperationError> {
    sync::lock(tenant).await?;
    let record = authorised(tenant, id).await?;
    let row = tenant.transaction().query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE medication_review_edit_leases SET expires_at=timezone('UTC',clock_timestamp())+interval '5 minutes',updated_at=timezone('UTC',clock_timestamp()) WHERE prompt_id=$1 AND household_id=$2 AND membership_id=$3 AND token::text=$4 AND expires_at>timezone('UTC',clock_timestamp()) RETURNING expires_at",
        [record.id.into(), tenant.scope().household_id.into(), tenant.membership().id.into(), token.into()],
    )).await?;
    let Some(row) = row else {
        return Err(conflict(
            "review_lease_expired",
            "The editing time has expired",
        ));
    };
    let expires_at: chrono::NaiveDateTime = row.try_get("", "expires_at")?;
    Ok(json!({"data":{"expires_at":expires_at.and_utc().to_rfc3339()}}))
}

pub async fn require(
    tenant: &TenantTransaction,
    record: &review_prompt::Model,
    token: &str,
) -> Result<(), OperationError> {
    authorised(tenant, &record.id.to_string()).await?;
    let row = tenant.transaction().query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT 1 AS valid FROM medication_review_edit_leases WHERE prompt_id=$1 AND household_id=$2 AND membership_id=$3 AND token::text=$4 AND expires_at>timezone('UTC',clock_timestamp())",
        [record.id.into(), tenant.scope().household_id.into(), tenant.membership().id.into(), token.into()],
    )).await?;
    if row.is_none() {
        return Err(conflict(
            "review_lease_required",
            "Start editing this review again",
        ));
    }
    Ok(())
}

pub async fn expiry(
    tenant: &TenantTransaction,
    id: &str,
    token: &str,
) -> Result<Option<String>, OperationError> {
    let record = authorised(tenant, id).await?;
    let row = tenant.transaction().query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT expires_at FROM medication_review_edit_leases WHERE prompt_id=$1 AND household_id=$2 AND membership_id=$3 AND token::text=$4",
        [record.id.into(), tenant.scope().household_id.into(), tenant.membership().id.into(), token.into()],
    )).await?;
    row.map(|row| -> Result<String, sea_orm::DbErr> {
        let expires_at: chrono::NaiveDateTime = row.try_get("", "expires_at")?;
        Ok(expires_at.and_utc().to_rfc3339())
    })
    .transpose()
    .map_err(Into::into)
}

pub async fn release(
    tenant: &TenantTransaction,
    id: &str,
    token: &str,
) -> Result<(), OperationError> {
    sync::lock(tenant).await?;
    let record = authorised(tenant, id).await?;
    tenant.transaction().execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM medication_review_edit_leases WHERE prompt_id=$1 AND household_id=$2 AND membership_id=$3 AND token::text=$4",
        [record.id.into(), tenant.scope().household_id.into(), tenant.membership().id.into(), token.into()],
    )).await?;
    Ok(())
}

pub async fn consume(
    tenant: &TenantTransaction,
    record: &review_prompt::Model,
    token: &str,
) -> Result<(), OperationError> {
    require(tenant, record, token).await?;
    let result = tenant.transaction().execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM medication_review_edit_leases WHERE prompt_id=$1 AND household_id=$2 AND membership_id=$3 AND token::text=$4",
        [record.id.into(), tenant.scope().household_id.into(), tenant.membership().id.into(), token.into()],
    )).await?;
    if result.rows_affected() != 1 {
        return Err(conflict(
            "review_lease_required",
            "Start editing this review again",
        ));
    }
    Ok(())
}

pub async fn archive(
    tenant: &TenantTransaction,
    record: &review_prompt::Model,
    event_type: &str,
) -> Result<(), OperationError> {
    let snapshot = super::value(record);
    tenant.transaction().execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO medication_review_histories (household_id,prompt_id,event_type,snapshot,created_at) VALUES ($1,$2,$3,$4::jsonb,$5)",
        [tenant.scope().household_id.into(), record.id.into(), event_type.into(), snapshot.to_string().into(), Utc::now().naive_utc().into()],
    )).await?;
    Ok(())
}

pub async fn history(
    tenant: &TenantTransaction,
    record: &review_prompt::Model,
) -> Result<Vec<Value>, OperationError> {
    let rows = tenant.transaction().query_all_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT event_type,snapshot,created_at FROM medication_review_histories WHERE household_id=$1 AND prompt_id=$2 ORDER BY id DESC",
        [tenant.scope().household_id.into(), record.id.into()],
    )).await?;
    rows.into_iter().map(|row| {
        let event_type: String = row.try_get("", "event_type")?;
        let snapshot: Value = row.try_get("", "snapshot")?;
        let created_at: chrono::NaiveDateTime = row.try_get("", "created_at")?;
        Ok(json!({"event_type":event_type,"snapshot":snapshot,"created_at":created_at.and_utc().to_rfc3339()}))
    }).collect::<Result<Vec<_>, sea_orm::DbErr>>().map_err(Into::into)
}
