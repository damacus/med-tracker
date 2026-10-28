use crate::models::{
    entities::{account, version},
    errors::OperationError,
    identity::resource::HistoryActor,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseTransaction, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
    sea_query::{Expr, Func, Query},
};
use serde_json::json;

pub(crate) mod taken_dose {
    use sea_orm::entity::prelude::*;
    use serde::Serialize;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
    #[sea_orm(table_name = "personal_taken_doses")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: i64,
        pub person_id: i64,
        pub taken_at: DateTime,
        pub dose_amount: Option<Decimal>,
        pub dose_unit: Option<String>,
        pub medicine: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

#[derive(Clone, Copy)]
pub(crate) struct Cursor {
    pub taken_at: chrono::NaiveDateTime,
    pub id: i64,
}

pub(crate) struct Page {
    pub records: Vec<taken_dose::Model>,
    pub next: Option<Cursor>,
}

pub(crate) enum AccessOutcome {
    Read,
    Denied,
}

pub(crate) async fn page(
    transaction: &DatabaseTransaction,
    actor: &HistoryActor,
    before: Option<Cursor>,
    request_id: &str,
) -> Result<Page, OperationError> {
    if !actor.permits_history() {
        return Err(OperationError::Forbidden);
    }
    let account = account::Entity::find_by_id(actor.account_id().get())
        .one(transaction)
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let enable = Query::select()
        .expr(Func::cust("pg_catalog.set_config").args([
            Expr::val("med_tracker.personal_history_access"),
            Expr::val("true"),
            Expr::val(true),
        ]))
        .to_owned();
    transaction
        .execute_raw(transaction.get_database_backend().build(&enable))
        .await?;
    let mut query =
        taken_dose::Entity::find().filter(taken_dose::Column::PersonId.eq(account.person_id));
    if let Some(before) = before {
        query = query.filter(
            Condition::any()
                .add(taken_dose::Column::TakenAt.lt(before.taken_at))
                .add(
                    Condition::all()
                        .add(taken_dose::Column::TakenAt.eq(before.taken_at))
                        .add(taken_dose::Column::Id.lt(before.id)),
                ),
        );
    }
    let mut records = query
        .order_by_desc(taken_dose::Column::TakenAt)
        .order_by_desc(taken_dose::Column::Id)
        .limit(26)
        .all(transaction)
        .await?;
    let next = if records.len() > 25 {
        records.truncate(25);
        records.last().map(|record| Cursor {
            taken_at: record.taken_at,
            id: record.id,
        })
    } else {
        None
    };
    record_access(transaction, Some(actor), request_id, AccessOutcome::Read).await?;
    Ok(Page { records, next })
}

pub(crate) async fn record_access(
    transaction: &DatabaseTransaction,
    actor: Option<&HistoryActor>,
    request_id: &str,
    outcome: AccessOutcome,
) -> Result<(), OperationError> {
    let account_id = actor.map(|actor| actor.account_id().get());
    let person_id = match account_id {
        Some(id) => account::Entity::find_by_id(id)
            .one(transaction)
            .await?
            .map(|account| account.person_id),
        None => None,
    };
    let (event, outcome) = match outcome {
        AccessOutcome::Read => ("personal_history_read", "success"),
        AccessOutcome::Denied => ("personal_history_denied", "denied"),
    };
    let provenance = actor.map(HistoryActor::provenance);
    version::ActiveModel {
        item_type: Set(if actor.is_some() { "AccountClinicalAccess" } else { "PersonalHistoryRequest" }.into()),
        item_id: Set(account_id.unwrap_or_default()), event: Set(event.into()),
        whodunnit: Set(account_id.map(|id| id.to_string())), request_id: Set(Some(request_id.into())),
        object: Set(Some(json!({"outcome":outcome}).to_string())),
        audit_context: Set(json!({"actor_account_id":account_id,"actor_person_id":person_id,"household_id":null,"request_id":request_id,"outcome":outcome,"authentication_method":provenance.map(|value|value.method.label()),"session_reference":provenance.map(|value|format!("{}:{}",value.method.reference_prefix(),value.reference)),"policy_class":"PersonalClinicalAccess","policy_query":"read_self_taken_doses"})),
        created_at: Set(Some(chrono::Utc::now().naive_utc())), ..Default::default()
    }.insert(transaction).await?;
    Ok(())
}
