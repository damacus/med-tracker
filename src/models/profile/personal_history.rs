use crate::models::{
    care::doses::CredentialProvenance,
    entities::{account, user, version},
    errors::OperationError,
    ids::AccountId,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
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

pub(crate) async fn page(
    transaction: &DatabaseTransaction,
    account_id: AccountId,
    before_id: Option<i64>,
    request_id: &str,
    provenance: &CredentialProvenance,
) -> Result<Vec<taken_dose::Model>, OperationError> {
    let actor = account::Entity::find_by_id(account_id.get())
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
        taken_dose::Entity::find().filter(taken_dose::Column::PersonId.eq(actor.person_id));
    if let Some(before_id) = before_id {
        query = query.filter(taken_dose::Column::Id.lt(before_id));
    }
    let records = query
        .order_by_desc(taken_dose::Column::Id)
        .limit(26)
        .all(transaction)
        .await?;
    let actor_user = user::Entity::find()
        .filter(user::Column::PersonId.eq(actor.person_id))
        .one(transaction)
        .await?;
    version::ActiveModel {
        item_type: Set("AccountClinicalAccess".into()), item_id: Set(account_id.get()), event: Set("personal_history_read".into()),
        whodunnit: Set(actor_user.as_ref().map(|user|user.id.to_string())),
        request_id: Set(Some(request_id.into())),
        audit_context: Set(json!({"actor_account_id":account_id.get(),"actor_person_id":actor.person_id,"actor_user_id":actor_user.map(|user|user.id),"household_id":null,"request_id":request_id,"authentication_method":provenance.method.label(),"session_reference":format!("{}:{}",provenance.method.reference_prefix(),provenance.reference),"policy_class":"PersonalClinicalAccess","policy_query":"read_self_taken_doses"})),
        created_at: Set(Some(chrono::Utc::now().naive_utc())), ..Default::default()
    }.insert(transaction).await?;
    Ok(records)
}
