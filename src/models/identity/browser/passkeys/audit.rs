use super::*;

pub(super) async fn record(
    transaction: &DatabaseTransaction,
    account_id: i64,
    token_type: &str,
    action: &str,
    request_id: Option<&str>,
) -> Result<(), AuthenticationError> {
    transaction
        .execute_raw(sql(
            "SELECT set_config('med_tracker.current_account_id',$1,true)",
            [account_id.to_string().into()],
        ))
        .await
        .map_err(unavailable)?;
    let actor = transaction.query_one_raw(sql("SELECT p.household_id,u.id AS user_id FROM people p LEFT JOIN users u ON u.person_id=p.id WHERE p.account_id=$1 ORDER BY p.id LIMIT 1", [account_id.into()])).await.map_err(unavailable)?.ok_or(AuthenticationError::Unauthenticated)?;
    let user_id: Option<i64> = actor.try_get("", "user_id").map_err(unavailable)?;
    let household_id: Option<i64> = actor.try_get("", "household_id").map_err(unavailable)?;
    let event = format!("auth_token/{token_type}/{action}");
    let mut metadata =
        serde_json::json!({"account_id":account_id,"token_type":token_type,"action":action});
    if token_type == "webauthn_verification" {
        metadata["outcome"] = if action == "succeeded" {
            "success"
        } else {
            "failure"
        }
        .into();
    }
    let context = serde_json::json!({"actor_account_id":account_id,"actor_user_id":user_id,"request_id":request_id});
    transaction.execute_raw(sql("INSERT INTO versions (item_type,item_id,event,object,whodunnit,request_id,audit_context,created_at) VALUES ('AuthenticationToken',$1,$2,$3,$4,$5,$6,timezone('UTC',clock_timestamp()))", [account_id.into(),event.clone().into(),metadata.to_string().into(),user_id.map(|id|id.to_string()).into(),request_id.map(str::to_owned).into(),context.clone().into()])).await.map_err(unavailable)?;
    if let Some(household_id) = household_id {
        transaction
            .execute_raw(sql(
                "SELECT set_config('med_tracker.current_household_id',$1,true)",
                [household_id.to_string().into()],
            ))
            .await
            .map_err(unavailable)?;
        transaction.execute_raw(sql("INSERT INTO security_audit_events (household_id,actor_account_id,event_type,metadata,audit_context,request_id,created_at,updated_at) VALUES ($1,$2,$3,$4,$5,$6,timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()))", [household_id.into(),account_id.into(),event.into(),metadata.into(),context.into(),request_id.map(str::to_owned).into()])).await.map_err(unavailable)?;
    }
    Ok(())
}
