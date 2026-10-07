use axum::http::{HeaderMap, header};
use chrono::{DateTime, SecondsFormat, Utc};
use headers::{Authorization, HeaderMapExt, authorization::Bearer};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, Statement, TransactionTrait,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{AuthenticationLifetime, audit, store};
use crate::models::{
    care::doses::CredentialMethod,
    entities::{api_app_token, api_session, oauth_grant},
    errors::OperationError,
};

pub async fn households(db: &DatabaseConnection, account_id: i64) -> Result<Value, OperationError> {
    let transaction = begin(db, Some(account_id)).await?;
    let rows = transaction.query_all_raw(statement(
        "SELECT h.id,h.slug,h.name,m.role,m.id AS membership_id FROM household_memberships m JOIN households h ON h.id=m.household_id WHERE m.account_id=$1 AND m.status='active' AND m.revoked_at IS NULL AND h.status='active' AND h.lifecycle_state='active' ORDER BY m.id",
        [account_id.into()],
    )).await.map_err(unavailable)?;
    let data = rows
        .into_iter()
        .map(|row| -> Result<Value, OperationError> {
            Ok(json!({
                "id": row.try_get::<i64>("", "id").map_err(unavailable)?,
                "slug": row.try_get::<String>("", "slug").map_err(unavailable)?,
                "name": row.try_get::<String>("", "name").map_err(unavailable)?,
                "role": row.try_get::<String>("", "role").map_err(unavailable)?,
                "membership_id": row.try_get::<i64>("", "membership_id").map_err(unavailable)?,
            }))
        })
        .collect::<Result<Vec<_>, _>>()?;
    transaction.commit().await.map_err(unavailable)?;
    Ok(json!({"account_id":account_id,"data":data}))
}

pub async fn sessions(
    db: &DatabaseConnection,
    account_id: i64,
    method: CredentialMethod,
) -> Result<Value, OperationError> {
    let transaction = begin(db, Some(account_id)).await?;
    let data = match method {
        CredentialMethod::OauthGrant => mobile_sessions(&transaction, account_id).await?,
        CredentialMethod::ApiSession | CredentialMethod::ApiAppToken => {
            api_sessions(&transaction, account_id).await?
        }
        CredentialMethod::BrowserSession => return Err(OperationError::Forbidden),
    };
    transaction.commit().await.map_err(unavailable)?;
    Ok(json!({"data":data}))
}

async fn api_sessions(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<Vec<Value>, OperationError> {
    let rows = transaction.query_all_raw(statement(
        "SELECT s.id,s.device_name,m.household_id,s.last_used_at,s.access_expires_at,s.refresh_expires_at,s.created_at FROM api_sessions s LEFT JOIN household_memberships m ON m.id=s.household_membership_id AND m.account_id=s.account_id WHERE s.account_id=$1 AND s.revoked_at IS NULL ORDER BY s.created_at DESC,s.id DESC",
        [account_id.into()],
    )).await.map_err(unavailable)?;
    rows.into_iter().map(|row| -> Result<Value, OperationError> {
        Ok(json!({
            "id":row.try_get::<i64>("","id").map_err(unavailable)?,
            "device_name":row.try_get::<Option<String>>("","device_name").map_err(unavailable)?,
            "household_id":row.try_get::<Option<i64>>("","household_id").map_err(unavailable)?,
            "last_used_at":stamp(row.try_get("","last_used_at").map_err(unavailable)?),
            "access_token_expires_at":stamp(row.try_get("","access_expires_at").map_err(unavailable)?),
            "refresh_token_expires_at":stamp(row.try_get("","refresh_expires_at").map_err(unavailable)?),
            "created_at":stamp(row.try_get("","created_at").map_err(unavailable)?),
        }))
    }).collect()
}

async fn mobile_sessions(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<Vec<Value>, OperationError> {
    let lifetime = AuthenticationLifetime::from_environment().map_err(unavailable)?;
    let now = Utc::now().naive_utc();
    let rows = oauth_grant::Entity::find()
        .filter(oauth_grant::Column::AccountId.eq(account_id))
        .filter(oauth_grant::Column::ClientKind.eq("mobile"))
        .filter(oauth_grant::Column::TokenHash.is_not_null())
        .filter(oauth_grant::Column::RevokedAt.is_null())
        .order_by_desc(oauth_grant::Column::CreatedAt)
        .order_by_desc(oauth_grant::Column::Id)
        .all(transaction)
        .await
        .map_err(unavailable)?;
    let data = rows
        .into_iter()
        .filter_map(|row| {
            let (Some(last_used), Some(authenticated)) = (row.last_used_at, row.authenticated_at)
            else {
                return None;
            };
            let refresh = last_used.checked_add_signed(lifetime.inactivity)?;
            let refresh = match lifetime.maximum_age {
                Some(maximum) => refresh.min(authenticated.checked_add_signed(maximum)?),
                None => refresh,
            };
            (refresh > now).then(|| {
                json!({
                    "id":row.id,"device_name":row.device_name,"household_id":Value::Null,
                    "last_used_at":stamp(last_used),
                    "access_token_expires_at":stamp(row.expires_in),
                    "refresh_token_expires_at":stamp(refresh),
                    "created_at":stamp(row.created_at),
                })
            })
        })
        .collect();
    Ok(data)
}

pub async fn revoke(
    db: &DatabaseConnection,
    account_id: i64,
    method: CredentialMethod,
    id: i64,
    request_id: &str,
) -> Result<(), OperationError> {
    let transaction = begin(db, Some(account_id)).await?;
    let result = match method {
        CredentialMethod::ApiSession | CredentialMethod::ApiAppToken => {
            revoke_api_session(&transaction, account_id, id, request_id).await
        }
        CredentialMethod::OauthGrant => {
            revoke_mobile(&transaction, account_id, id, request_id).await
        }
        CredentialMethod::BrowserSession => Err(OperationError::Forbidden),
    };
    match result {
        Ok(()) => transaction.commit().await.map_err(unavailable),
        Err(error) => {
            transaction.rollback().await.map_err(unavailable)?;
            Err(error)
        }
    }
}

async fn revoke_api_session(
    transaction: &DatabaseTransaction,
    account_id: i64,
    id: i64,
    request_id: &str,
) -> Result<(), OperationError> {
    let row = transaction.query_one_raw(statement(
        "UPDATE api_sessions SET revoked_at=timezone('UTC',clock_timestamp()),updated_at=timezone('UTC',clock_timestamp()) WHERE id=$1 AND account_id=$2 AND revoked_at IS NULL RETURNING household_membership_id",
        [id.into(), account_id.into()],
    )).await.map_err(unavailable)?.ok_or(OperationError::NotFound)?;
    let member: Option<i64> = row
        .try_get("", "household_membership_id")
        .map_err(unavailable)?;
    audit_token(
        transaction,
        account_id,
        member,
        id,
        "api_session",
        request_id,
    )
    .await
}

async fn revoke_mobile(
    transaction: &DatabaseTransaction,
    account_id: i64,
    id: i64,
    request_id: &str,
) -> Result<(), OperationError> {
    let row = oauth_grant::Entity::find_by_id(id)
        .filter(oauth_grant::Column::AccountId.eq(account_id))
        .filter(oauth_grant::Column::ClientKind.eq("mobile"))
        .filter(oauth_grant::Column::TokenHash.is_not_null())
        .filter(oauth_grant::Column::RevokedAt.is_null())
        .one(transaction)
        .await
        .map_err(unavailable)?
        .ok_or(OperationError::NotFound)?;
    let changed = transaction.execute_raw(statement(
        "UPDATE oauth_grants SET revoked_at=timezone('UTC',clock_timestamp()),updated_at=timezone('UTC',clock_timestamp()) WHERE id=$1 AND account_id=$2 AND client_kind='mobile' AND token_hash IS NOT NULL AND revoked_at IS NULL",
        [id.into(), account_id.into()],
    )).await.map_err(unavailable)?;
    if changed.rows_affected() != 1 {
        return Err(OperationError::NotFound);
    }
    audit::record(transaction, &row, "revoked", Some(request_id))
        .await
        .map_err(unavailable)
}

pub async fn logout(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    request_id: &str,
) -> Result<(), OperationError> {
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
        return Ok(());
    }
    let Some(bearer) = headers
        .typed_try_get::<Authorization<Bearer>>()
        .ok()
        .flatten()
    else {
        return Ok(());
    };
    let token = bearer.token();
    let stored_digest = hex::encode(Sha256::digest(token.as_bytes()));
    let mobile_digest = store::digest(token);
    let transaction = begin(db, None).await?;
    let result = logout_in(&transaction, &stored_digest, &mobile_digest, request_id).await;
    match result {
        Ok(()) => transaction.commit().await.map_err(unavailable),
        Err(error) => {
            transaction.rollback().await.map_err(unavailable)?;
            Err(error)
        }
    }
}

async fn logout_in(
    transaction: &DatabaseTransaction,
    stored_digest: &str,
    mobile_digest: &str,
    request_id: &str,
) -> Result<(), OperationError> {
    if let Some(row) = api_session::Entity::find()
        .filter(api_session::Column::AccessTokenDigest.eq(stored_digest))
        .one(transaction)
        .await
        .map_err(unavailable)?
    {
        set_account(transaction, row.account_id).await?;
        match revoke_api_session(transaction, row.account_id, row.id, request_id).await {
            Ok(()) | Err(OperationError::NotFound) => {}
            Err(error) => return Err(error),
        }
        return Ok(());
    }
    if let Some(row) = api_app_token::Entity::find()
        .filter(api_app_token::Column::TokenDigest.eq(stored_digest))
        .one(transaction)
        .await
        .map_err(unavailable)?
    {
        set_account(transaction, row.account_id).await?;
        let changed = transaction.execute_raw(statement(
            "UPDATE api_app_tokens SET revoked_at=timezone('UTC',clock_timestamp()),updated_at=timezone('UTC',clock_timestamp()) WHERE id=$1 AND account_id=$2 AND revoked_at IS NULL",
            [row.id.into(), row.account_id.into()],
        )).await.map_err(unavailable)?;
        if changed.rows_affected() == 1 {
            audit_token(
                transaction,
                row.account_id,
                Some(row.household_membership_id),
                row.id,
                "api_app_token",
                request_id,
            )
            .await?;
        }
        return Ok(());
    }
    if let Some(row) = oauth_grant::Entity::find()
        .filter(oauth_grant::Column::TokenHash.eq(mobile_digest))
        .one(transaction)
        .await
        .map_err(unavailable)?
    {
        set_account(transaction, row.account_id).await?;
        let changed = transaction.execute_raw(statement(
            "UPDATE oauth_grants SET revoked_at=timezone('UTC',clock_timestamp()),updated_at=timezone('UTC',clock_timestamp()) WHERE id=$1 AND account_id=$2 AND token_hash=$3 AND revoked_at IS NULL",
            [row.id.into(), row.account_id.into(), mobile_digest.into()],
        )).await.map_err(unavailable)?;
        if changed.rows_affected() == 1 {
            audit::record(transaction, &row, "revoked", Some(request_id))
                .await
                .map_err(unavailable)?;
        }
    }
    Ok(())
}

async fn audit_token(
    transaction: &DatabaseTransaction,
    account_id: i64,
    member_id: Option<i64>,
    credential_id: i64,
    kind: &str,
    request_id: &str,
) -> Result<(), OperationError> {
    let event = format!("auth_token/{kind}/revoked");
    let metadata = json!({"account_id":account_id,"credential_id":credential_id,"token_type":kind,"action":"revoked"});
    let actor = transaction.query_one_raw(statement(
        "SELECT u.id AS user_id FROM people p LEFT JOIN users u ON u.person_id=p.id WHERE p.account_id=$1 ORDER BY p.id LIMIT 1",
        [account_id.into()],
    )).await.map_err(unavailable)?;
    let user_id: Option<i64> = match actor {
        Some(row) => row.try_get("", "user_id").map_err(unavailable)?,
        None => None,
    };
    let context =
        json!({"actor_account_id":account_id,"actor_user_id":user_id,"request_id":request_id});
    transaction.execute_raw(statement(
        "INSERT INTO versions(item_type,item_id,event,object,whodunnit,request_id,audit_context,created_at) VALUES('AuthenticationToken',$1,$2,$3,$4,$5,$6,timezone('UTC',clock_timestamp()))",
        [account_id.into(), event.clone().into(), metadata.to_string().into(), user_id.map(|id| id.to_string()).into(), request_id.into(), context.clone().into()],
    )).await.map_err(unavailable)?;
    if let Some(member_id) = member_id {
        let row = transaction
            .query_one_raw(statement(
                "SELECT household_id FROM household_memberships WHERE id=$1 AND account_id=$2",
                [member_id.into(), account_id.into()],
            ))
            .await
            .map_err(unavailable)?;
        if let Some(row) = row {
            let household_id: i64 = row.try_get("", "household_id").map_err(unavailable)?;
            transaction.query_one_raw(statement(
                "SELECT set_config('med_tracker.current_household_id',$1,true),set_config('med_tracker.current_membership_id',$2,true)",
                [household_id.to_string().into(), member_id.to_string().into()],
            )).await.map_err(unavailable)?;
            transaction.execute_raw(statement(
                "INSERT INTO security_audit_events(household_id,actor_account_id,actor_membership_id,event_type,metadata,audit_context,request_id,created_at,updated_at) VALUES($1,$2,$3,$4,$5,$6,$7,timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()))",
                [household_id.into(), account_id.into(), member_id.into(), event.into(), metadata.into(), context.into(), request_id.into()],
            )).await.map_err(unavailable)?;
        }
    }
    Ok(())
}

async fn begin(
    db: &DatabaseConnection,
    account_id: Option<i64>,
) -> Result<DatabaseTransaction, OperationError> {
    let transaction = db.begin().await.map_err(unavailable)?;
    transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app; SET LOCAL search_path = pg_catalog, public, pg_temp; SET LOCAL lock_timeout = '5s'; SET LOCAL statement_timeout = '30s'").await.map_err(unavailable)?;
    transaction.query_one_raw(statement(
        "SELECT set_config('med_tracker.current_account_id',$1,true),set_config('med_tracker.current_household_id','',true),set_config('med_tracker.current_membership_id','',true),set_config('med_tracker.current_invitation_token_digest','',true)",
        [account_id.map(|id| id.to_string()).unwrap_or_default().into()],
    )).await.map_err(unavailable)?;
    Ok(transaction)
}

async fn set_account(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<(), OperationError> {
    transaction
        .query_one_raw(statement(
            "SELECT set_config('med_tracker.current_account_id',$1,true)",
            [account_id.to_string().into()],
        ))
        .await
        .map_err(unavailable)?;
    Ok(())
}

fn statement(sql: &str, values: impl IntoIterator<Item = sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

fn stamp(value: chrono::NaiveDateTime) -> String {
    DateTime::<Utc>::from_naive_utc_and_offset(value, Utc)
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn unavailable<E>(_: E) -> OperationError {
    OperationError::Unavailable
}
