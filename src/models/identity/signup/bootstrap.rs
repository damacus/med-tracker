use super::*;
use serde_json::json;

pub(super) async fn household(
    transaction: &DatabaseTransaction,
    account_id: i64,
    name: &str,
) -> Result<i64, SignupError> {
    let name = format!("{} Household", name.trim());
    let mut suffix = [0u8; 4];
    getrandom::fill(&mut suffix).map_err(unavailable)?;
    let slug = format!("{}-{}", slug::slugify(&name), hex::encode(suffix));
    let timezone = super::super::time_zone::preferred(&json!({}))
        .map_err(unavailable)?
        .to_string();
    let row = transaction.query_one_raw(sql(
        "INSERT INTO households(name,slug,timezone,created_by_account_id,created_at,updated_at) VALUES($1,$2,$3,$4,now(),now()) RETURNING id",
        [name.into(),slug.into(),timezone.into(),account_id.into()])).await.map_err(unavailable)?.ok_or(SignupError::Unavailable)?;
    let id: i64 = row.try_get("", "id").map_err(unavailable)?;
    transaction
        .execute_raw(sql(
            "SELECT set_config('med_tracker.current_household_id',$1,true)",
            [id.to_string().into()],
        ))
        .await
        .map_err(unavailable)?;
    Ok(id)
}

pub(super) async fn owner(
    transaction: &DatabaseTransaction,
    actor: &SignupInvitationContext,
    household_id: i64,
    name: &str,
    request_id: Option<&str>,
) -> Result<(), SignupError> {
    let row = transaction.query_one_raw(sql(
        "INSERT INTO household_memberships(account_id,household_id,person_id,role,status,permissions_version,joined_at,created_at,updated_at) VALUES($1,$2,$3,'owner','active',1,now(),now(),now()) RETURNING id",
        [actor.account_id.into(),household_id.into(),actor.person_id.into()])).await.map_err(unavailable)?.ok_or(SignupError::Unavailable)?;
    let membership_id: i64 = row.try_get("", "id").map_err(unavailable)?;
    let row = transaction.query_one_raw(sql(
        "INSERT INTO person_access_grants(household_id,household_membership_id,person_id,access_level,relationship_type,granted_by_membership_id,created_at,updated_at) VALUES($1,$2,$3,'manage','self',$2,now(),now()) RETURNING id",
        [household_id.into(),membership_id.into(),actor.person_id.into()])).await.map_err(unavailable)?.ok_or(SignupError::Unavailable)?;
    let grant_id: i64 = row.try_get("", "id").map_err(unavailable)?;
    let context = json!({"actor_account_id":actor.account_id,"actor_user_id":actor.user_id,"request_id":request_id});
    let membership = json!({"target_account_id":actor.account_id,"target_membership_id":membership_id,"previous_state":null,"new_state":{"role":"owner","status":"active","person_id":actor.person_id,"permissions_version":1},"outcome":"success"});
    let grant = json!({"target_membership_id":membership_id,"target_grant_id":grant_id,"previous_state":null,"new_state":{"household_membership_id":membership_id,"person_id":actor.person_id,"access_level":"manage","relationship_type":"self","expires_at":null,"revoked_at":null,"carer_relationship_id":null},"outcome":"success"});
    for (event, member, metadata) in [
        ("household_access.membership_created", None, membership),
        (
            "household_access.person_grant_changed",
            Some(membership_id),
            grant,
        ),
    ] {
        transaction.execute_raw(sql("INSERT INTO security_audit_events(household_id,actor_account_id,actor_membership_id,event_type,metadata,audit_context,request_id,created_at,updated_at) VALUES($1,$2,$3,$4,$5,$6,$7,now(),now())",
            [household_id.into(),actor.account_id.into(),member.into(),event.into(),metadata.into(),context.clone().into(),request_id.map(str::to_owned).into()])).await.map_err(unavailable)?;
    }
    let changes =
        json!({"name":[null,name.trim()],"person_type":[null,0],"has_capacity":[null,true]});
    transaction.execute_raw(sql("INSERT INTO versions(item_type,item_id,event,object_changes,household_id,request_id,audit_context,created_at) VALUES('Person',$1,'create',$2,$3,$4,$5,now())",
        [actor.person_id.into(),changes.to_string().into(),household_id.into(),request_id.map(str::to_owned).into(),context.into()])).await.map_err(unavailable)?;
    transaction.execute_raw(sql("INSERT INTO api_change_events(household_id,account_id,action,record_type,record_id,record_portable_id,request_id,metadata,occurred_at,created_at,updated_at) SELECT $1,$2,'create','Person',id,portable_id,$3,jsonb_build_object('record_type','Person','record_id',id,'portable_id',portable_id),now(),now(),now() FROM people WHERE id=$4 AND household_id=$1",
        [household_id.into(),actor.account_id.into(),request_id.map(str::to_owned).into(),actor.person_id.into()])).await.map_err(unavailable)?;
    Ok(())
}
