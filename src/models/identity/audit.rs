use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseTransaction, DbBackend, Set, Statement};
use serde_json::json;

use super::ExchangeError;
use crate::models::entities::{oauth_grant, security_audit_event, version};

pub(super) async fn record(
    transaction: &DatabaseTransaction,
    grant: &oauth_grant::Model,
    action: &str,
    request_id: Option<&str>,
) -> Result<(), ExchangeError> {
    transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT set_config('med_tracker.current_account_id',$1,true),set_config('med_tracker.current_household_id','',true),set_config('med_tracker.current_membership_id','',true)", [grant.account_id.to_string().into()])).await.map_err(|_| ExchangeError::Unavailable)?;
    let actor = transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT u.id FROM people p LEFT JOIN users u ON u.person_id=p.id WHERE p.account_id=$1 ORDER BY p.id LIMIT 1", [grant.account_id.into()])).await.map_err(|_| ExchangeError::Unavailable)?;
    let user_id = match actor {
        Some(row) => row
            .try_get::<Option<i64>>("", "id")
            .map_err(|_| ExchangeError::Unavailable)?,
        None => None,
    };
    let now = chrono::Utc::now().naive_utc();
    let mut context = json!({"actor_account_id":grant.account_id,"actor_user_id":user_id,"request_id":request_id});
    if grant.client_kind == "mobile" {
        version::ActiveModel {
            item_type: Set("Account".into()),
            item_id: Set(grant.account_id),
            event: Set(format!("mobile_oauth.{action}")),
            object: Set(Some(
                json!({"oauth_application_id":grant.oauth_application_id,"oauth_grant_id":grant.id}).to_string(),
            )),
            whodunnit: Set(user_id.map(|id| id.to_string())),
            request_id: Set(request_id.map(str::to_owned)),
            audit_context: Set(context),
            created_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(transaction)
        .await
        .map_err(|_| ExchangeError::Unavailable)?;
        return Ok(());
    }
    let member = transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT id,household_id,role,permissions_version FROM household_memberships WHERE id=$1 AND account_id=$2", [grant.household_membership_id.into(),grant.account_id.into()])).await.map_err(|_| ExchangeError::Unavailable)?.ok_or(ExchangeError::Unavailable)?;
    let membership_id: i64 = member
        .try_get("", "id")
        .map_err(|_| ExchangeError::Unavailable)?;
    let household_id: i64 = member
        .try_get("", "household_id")
        .map_err(|_| ExchangeError::Unavailable)?;
    context["actor_membership_id"] = membership_id.into();
    context["household_id"] = household_id.into();
    context["active_role"] = member
        .try_get::<String>("", "role")
        .map_err(|_| ExchangeError::Unavailable)?
        .into();
    context["permissions_version"] = member
        .try_get::<i32>("", "permissions_version")
        .map_err(|_| ExchangeError::Unavailable)?
        .into();
    transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT set_config('med_tracker.current_household_id',$1,true),set_config('med_tracker.current_membership_id',$2,true)", [household_id.to_string().into(),membership_id.to_string().into()])).await.map_err(|_| ExchangeError::Unavailable)?;
    security_audit_event::ActiveModel {
        household_id: Set(household_id), actor_account_id: Set(Some(grant.account_id)), actor_membership_id: Set(Some(membership_id)),
        event_type: Set(format!("smart_oauth.{action}")), request_id: Set(request_id.map(str::to_owned)),
        metadata: Set(json!({"oauth_application_id":grant.oauth_application_id,"account_id":grant.account_id,"household_membership_id":membership_id,"person_id":grant.person_id,"scopes":grant.scopes})),
        audit_context: Set(context), created_at: Set(now), updated_at: Set(now), ..Default::default()
    }.insert(transaction).await.map_err(|_| ExchangeError::Unavailable)?;
    Ok(())
}
