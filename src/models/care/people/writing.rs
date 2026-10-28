use super::*;

pub(crate) struct SelfClinicalChanges {
    name: String,
    date_of_birth: NaiveDate,
}

impl SelfClinicalChanges {
    pub(crate) fn parse(name: &str, date: &str) -> Result<Self, OperationError> {
        crate::models::profile::self_account::Name::parse(name)?;
        let date_of_birth = crate::models::profile::parse_date(date)?;
        let errors =
            crate::models::identity::signup::profile_errors(name, Some(date_of_birth), false);
        if !errors.is_empty() {
            return Err(OperationError::Validation {
                details: json!({"errors":errors}),
            });
        }
        Ok(Self {
            name: name.trim().to_owned(),
            date_of_birth,
        })
    }
}

pub(crate) async fn update_self(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: crate::models::ids::AccountId,
    changes: SelfClinicalChanges,
    zone: Tz,
    request_id: &str,
    provenance: &CredentialProvenance,
) -> Result<(), OperationError> {
    let current = access::person_queries::account_person(access::Actor {
        account_id: account_id.get(),
    })
    .lock_exclusive()
    .one(transaction)
    .await?
    .ok_or(OperationError::Unauthenticated)?;
    let mut proposed = current.clone();
    validation::assign(
        &mut proposed,
        PersonUpdateInput {
            name: UpdateField::Value(changes.name),
            date_of_birth: UpdateField::Value(changes.date_of_birth),
            ..Default::default()
        },
    )?;
    validation::capacity(&mut proposed, Utc::now().with_timezone(&zone).date_naive())?;
    if proposed == current {
        return Ok(());
    }
    let before = persistence::snapshot(&current);
    let person_id = current.id;
    let mut active: person::ActiveModel = current.into();
    active.name = Set(proposed.name);
    active.date_of_birth = Set(proposed.date_of_birth);
    active.has_capacity = Set(proposed.has_capacity);
    active.updated_at = Set(Utc::now().naive_utc());
    let saved = active.update(transaction).await.map_err(write_error)?;
    version::ActiveModel {
        item_type: Set("Person".into()), item_id: Set(person_id), event: Set("update".into()),
        object: Set(Some(before.to_string())),
        object_changes: Set(Some(json!({"name":[before["name"],saved.name],"date_of_birth":[before["date_of_birth"],saved.date_of_birth]}).to_string())),
        whodunnit: Set(Some(account_id.get().to_string())),
        request_id: Set(Some(request_id.into())),
        audit_context: Set(json!({"actor_account_id":account_id.get(),"actor_person_id":person_id,"household_id":null,"request_id":request_id,"authentication_method":provenance.method.label(),"session_reference":format!("{}:{}",provenance.method.reference_prefix(),provenance.reference),"policy_class":"PersonalClinicalAccess","policy_query":"update_self"})),
        created_at: Set(Some(Utc::now().naive_utc())), ..Default::default()
    }.insert(transaction).await?;
    persistence::self_changes(transaction, account_id, &saved, request_id).await?;
    Ok(())
}

pub(crate) async fn update(
    tenant: &TenantTransaction,
    id: &str,
    input: PersonUpdateInput,
    today: NaiveDate,
    provenance: Option<&CredentialProvenance>,
) -> Result<person::Model, OperationError> {
    let found = authorize_update(tenant, id).await?;
    let record =
        crate::models::access::person_queries::household_participants(tenant.scope().household_id)
            .filter(crate::models::entities::person::Column::Id.eq(found.id))
            .lock_exclusive()
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::NotFound)?;
    let mut proposed = record.clone();
    validation::assign(&mut proposed, input)?;
    validation::capacity(&mut proposed, today)?;
    if proposed == record {
        return Ok(record);
    }
    let before = persistence::snapshot(&record);
    let mut active: person::ActiveModel = record.into();
    active.name = Set(proposed.name);
    active.email = Set(proposed.email);
    active.date_of_birth = Set(proposed.date_of_birth);
    active.person_type = Set(proposed.person_type);
    active.has_capacity = Set(proposed.has_capacity);
    active.updated_at = Set(Utc::now().naive_utc());
    let saved = active
        .update(tenant.transaction())
        .await
        .map_err(write_error)?;
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
