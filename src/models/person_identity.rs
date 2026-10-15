use std::collections::BTreeMap;

use sea_orm::{ConnectionTrait, DbBackend, Statement};

use super::{
    access::TenantTransaction,
    errors::OperationError,
    ids::{AccountId, PersonId},
};

pub(crate) struct PublicAccount {
    pub id: AccountId,
    pub name: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

pub(crate) async fn household_accounts(
    tenant: &TenantTransaction,
) -> Result<BTreeMap<PersonId, PublicAccount>, OperationError> {
    super::access::recheck(tenant).await?;
    let rows = tenant
        .transaction()
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.household_accounts($1)",
            [tenant.scope().household_id.into()],
        ))
        .await?;
    rows.into_iter()
        .map(|row| {
            let person_id = PersonId::from_database(row.try_get("", "person_id")?);
            Ok((
                person_id,
                PublicAccount {
                    id: AccountId::from_database(row.try_get("", "account_id")?),
                    name: row.try_get("", "name")?,
                    created_at: row.try_get("", "created_at")?,
                    updated_at: row.try_get("", "updated_at")?,
                },
            ))
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct PersonSummary {
    pub id: PersonId,
    pub name: String,
    pub date_of_birth: Option<chrono::NaiveDate>,
}

pub(crate) async fn summaries(
    tenant: &TenantTransaction,
    people: Vec<super::entities::person::Model>,
) -> Result<Vec<PersonSummary>, OperationError> {
    let mut names = display_names(
        tenant,
        people
            .iter()
            .map(|person| PersonId::from_database(person.id)),
    )
    .await?;
    people
        .into_iter()
        .map(|person| {
            Ok(PersonSummary {
                id: PersonId::from_database(person.id),
                name: names
                    .remove(&PersonId::from_database(person.id))
                    .ok_or(OperationError::Unavailable)?,
                date_of_birth: person.date_of_birth,
            })
        })
        .collect()
}

pub(crate) async fn current_summaries(
    tenant: &TenantTransaction,
) -> Result<Vec<PersonSummary>, OperationError> {
    let people = super::access::current_people(tenant).await?;
    let mut rows = summaries(tenant, people).await?;
    rows.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    Ok(rows)
}

pub(crate) async fn display_names(
    tenant: &TenantTransaction,
    people: impl IntoIterator<Item = PersonId>,
) -> Result<BTreeMap<PersonId, String>, OperationError> {
    display_names_in(tenant.transaction(), people).await
}

pub(crate) async fn display_names_in(
    transaction: &sea_orm::DatabaseTransaction,
    people: impl IntoIterator<Item = PersonId>,
) -> Result<BTreeMap<PersonId, String>, OperationError> {
    let ids = people.into_iter().map(PersonId::get).collect::<Vec<_>>();
    if ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let rows = transaction.query_all_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT id,med_tracker.person_display_name(id) AS display_name FROM people WHERE id=ANY($1::bigint[])",
        [ids.into()],
    )).await?;
    rows.into_iter()
        .map(|row| {
            Ok((
                PersonId::from_database(row.try_get("", "id")?),
                row.try_get("", "display_name")?,
            ))
        })
        .collect()
}
