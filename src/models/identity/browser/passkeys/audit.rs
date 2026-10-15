use super::*;

pub(in crate::models::identity) async fn record(
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
    let actor = transaction
        .query_one_raw(sql(
            "SELECT person_id FROM accounts WHERE id=$1",
            [account_id.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unauthenticated)?;
    let person_id = crate::models::ids::PersonId::from_database(
        actor.try_get("", "person_id").map_err(unavailable)?,
    );
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
    let context = serde_json::json!({"actor_account_id":account_id,"actor_person_id":person_id,"request_id":request_id});
    transaction.execute_raw(sql("INSERT INTO versions (item_type,item_id,event,object,whodunnit,request_id,audit_context,created_at) VALUES ('AuthenticationToken',$1,$2,$3,$4,$5,$6,timezone('UTC',clock_timestamp()))", [account_id.into(),event.clone().into(),metadata.to_string().into(),person_id.to_string().into(),request_id.map(str::to_owned).into(),context.clone().into()])).await.map_err(unavailable)?;
    Ok(())
}
