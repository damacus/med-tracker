use super::*;
use loco_rs::model::TenantQueryExt;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, QuerySelect, Statement, TransactionTrait,
};

pub async fn list(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    collection(tenant, None).await
}

pub async fn list_api(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    collection(tenant, Some(100)).await
}

async fn collection(
    tenant: &TenantTransaction,
    limit: Option<u64>,
) -> Result<Value, OperationError> {
    administration::authorize(tenant).await?;
    let rows = household_invitation::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .order_by_desc(household_invitation::Column::CreatedAt)
        .order_by_desc(household_invitation::Column::Id)
        .limit(limit)
        .all(tenant.transaction())
        .await?;
    let now = Utc::now().naive_utc();
    Ok(json!({"data":rows.iter().map(|row|summary(row,now)).collect::<Vec<_>>()}))
}

pub async fn options(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    administration::authorize(tenant).await?;
    Ok(json!({"dependents":[]}))
}

pub async fn preview(db: &DatabaseConnection, token: &str) -> Result<Value, OperationError> {
    if token.trim().is_empty() {
        return Err(super::acceptance::unavailable());
    }
    let transaction = db.begin().await?;
    transaction
        .execute_unprepared("SET LOCAL ROLE med_tracker_app")
        .await?;
    let row = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.invitation_preview($1)",
            [tokens::digest(token).into()],
        ))
        .await?
        .ok_or_else(super::acceptance::unavailable)?;
    let details = json!({"email":row.try_get::<String>("","email")?,"membership_role":row.try_get::<String>("","membership_role")?,"household_name":row.try_get::<String>("","household_name")?});
    transaction.commit().await?;
    Ok(details)
}
