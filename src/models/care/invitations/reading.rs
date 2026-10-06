use super::*;
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
        .filter(household_invitation::Column::HouseholdId.eq(tenant.scope().household_id))
        .order_by_desc(household_invitation::Column::CreatedAt)
        .order_by_desc(household_invitation::Column::Id)
        .limit(limit)
        .all(tenant.transaction())
        .await?;
    let now = Utc::now().naive_utc();
    Ok(json!({"data":rows.iter().map(|row|summary(row,now)).collect::<Vec<_>>()}))
}

pub async fn options(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    use crate::models::entities::person;
    let scoped = administration::delegation::options(tenant).await?;
    let ids: Vec<i64> = scoped["patients"]
        .as_array()
        .ok_or(OperationError::Unavailable)?
        .iter()
        .filter_map(|value| value["id"].as_i64())
        .collect();
    let people = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person::Column::Id.is_in(ids))
        .filter(person::Column::PersonType.is_in([1, 2]))
        .filter(person::Column::HasCapacity.eq(false))
        .order_by_asc(person::Column::Name)
        .order_by_asc(person::Column::Id)
        .all(tenant.transaction())
        .await?;
    Ok(
        json!({"dependents":people.into_iter().map(|person|json!({"id":person.id,"name":person.name})).collect::<Vec<_>>()}),
    )
}

pub async fn preview(db: &DatabaseConnection, token: &str) -> Result<Value, OperationError> {
    use crate::models::entities::household;
    if token.trim().is_empty() {
        return Err(super::acceptance::unavailable());
    }
    let transaction = db.begin().await?;
    transaction
        .execute_unprepared("SET LOCAL ROLE med_tracker_app")
        .await?;
    transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT set_config('med_tracker.current_account_id','',true),set_config('med_tracker.current_household_id','',true),set_config('med_tracker.current_membership_id','',true),set_config('med_tracker.current_invitation_token_digest',$1,true)",[tokens::digest(token).into()])).await?;
    let row = household_invitation::Entity::find()
        .filter(household_invitation::Column::TokenDigest.eq(tokens::digest(token)))
        .filter(household_invitation::Column::AcceptedAt.is_null())
        .filter(household_invitation::Column::RevokedAt.is_null())
        .filter(household_invitation::Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .one(&transaction)
        .await?
        .ok_or_else(super::acceptance::unavailable)?;
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_household_id',$1,true)",
            [row.household_id.to_string().into()],
        ))
        .await?;
    let home = household::Entity::find_by_id(row.household_id)
        .one(&transaction)
        .await?
        .ok_or_else(super::acceptance::unavailable)?;
    if home.status != "active" || home.lifecycle_state != "active" {
        return Err(super::acceptance::unavailable());
    }
    transaction.commit().await?;
    Ok(json!({"email":row.email,"membership_role":row.membership_role,"household_name":home.name}))
}
