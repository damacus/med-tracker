use super::*;

pub async fn update(
    tenant: &TenantTransaction,
    id: &str,
    attributes: Value,
    today: NaiveDate,
    provenance: Option<&CredentialProvenance>,
) -> Result<person::Model, OperationError> {
    let found = authorize_update(tenant, id).await?;
    let mut query = crate::models::access::person_scope(
        crate::models::ids::HouseholdId::from_database(tenant.scope().household_id),
    )
    .filter(person::Column::Id.eq(found.id));
    if tenant.person_id().get() == found.id {
        query = query.lock_exclusive();
    }
    let record = query
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    let mut proposed = record.clone();
    validation::assign(&mut proposed, &attributes, false)?;
    validation::capacity(&mut proposed, today)?;
    if attributes.get("name").is_some() && tenant.person_id().get() == record.id {
        use crate::models::entities::account;
        let account = account::Entity::find_by_id(tenant.scope().actor.account_id)
            .filter(account::Column::PersonId.eq(record.id))
            .lock_exclusive()
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::Forbidden)?;
        let mut account: account::ActiveModel = account.into();
        account.name = Set(proposed.clinical_name.clone());
        account.updated_at = Set(Utc::now().naive_utc());
        account.update(tenant.transaction()).await?;
        crate::models::care::administration::persistence::event(tenant, "account.profile.updated", json!({"target_account_id":tenant.scope().actor.account_id,"person_id":record.id,"changed_fields":["name"]}), provenance).await?;
        proposed.clinical_name = record.clinical_name.clone();
    }
    if proposed == record {
        return Ok(record);
    }
    let before = persistence::snapshot(&record);
    let saved = if tenant.person_id().get() != record.id {
        use sea_orm::{ConnectionTrait, DbBackend, Statement};
        tenant
            .transaction()
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT * FROM med_tracker.update_clinical_person($1,$2,$3,$4,$5)",
                [
                    record.id.into(),
                    proposed.clinical_name.into(),
                    proposed.date_of_birth.into(),
                    proposed.person_type.into(),
                    proposed.has_capacity.into(),
                ],
            ))
            .await?
            .ok_or(OperationError::Forbidden)?;
        person::Entity::find_by_id(record.id)
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::NotFound)?
    } else {
        let mut active: person::ActiveModel = record.into();
        active.clinical_name = Set(proposed.clinical_name);
        active.date_of_birth = Set(proposed.date_of_birth);
        active.person_type = Set(proposed.person_type);
        active.has_capacity = Set(proposed.has_capacity);
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(tenant.transaction())
            .await
            .map_err(write_error)?
    };
    persistence::record_version(
        tenant,
        "Person",
        saved.id,
        "update",
        Some(before),
        persistence::snapshot(&saved),
        provenance,
    )
    .await?;
    persistence::change(
        tenant,
        "Person",
        saved.id,
        &saved.portable_id,
        "update",
        provenance,
    )
    .await?;
    Ok(saved)
}
