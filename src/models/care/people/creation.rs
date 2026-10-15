use super::*;
use sea_orm::{ConnectionTrait, DbBackend, Statement};

pub async fn create(
    tenant: &TenantTransaction,
    attributes: Value,
    today: NaiveDate,
    provenance: Option<&CredentialProvenance>,
) -> Result<person::Model, OperationError> {
    authorize_create(tenant).await?;
    let now = Utc::now().naive_utc();
    let mut record = person::Model {
        id: 0,
        portable_id: uuid::Uuid::new_v4().to_string(),
        clinical_name: None,
        person_type: 0,
        date_of_birth: None,
        has_capacity: true,
        created_at: now,
        updated_at: now,
    };
    validation::assign(&mut record, &attributes, true)?;
    validation::capacity(&mut record, today)?;
    let row = tenant
        .transaction()
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT person_id,membership_id FROM med_tracker.create_household_person($1,$2,$3,$4)",
            [
                record.clinical_name.into(),
                record.date_of_birth.into(),
                record.person_type.into(),
                record.has_capacity.into(),
            ],
        ))
        .await
        .map_err(write_error)?
        .ok_or(OperationError::Unavailable)?;
    let person_id = crate::models::ids::PersonId::from_database(row.try_get("", "person_id")?);
    let saved = person::Entity::find_by_id(person_id.get())
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unavailable)?;
    persistence::record_version(
        tenant,
        "Person",
        saved.id,
        "create",
        None,
        persistence::snapshot(&saved),
        provenance,
    )
    .await?;
    persistence::change(
        tenant,
        "Person",
        saved.id,
        &saved.portable_id,
        "create",
        provenance,
    )
    .await?;
    Ok(saved)
}
