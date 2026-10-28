use super::fixture::Fixture;
use axum::http::{HeaderMap, HeaderValue, header};
use med_tracker::models::{
    _entities::{account_lockouts, api_app_tokens, api_sessions, household_memberships},
    identity::{api_session, resource::AuthenticationError},
};
use sea_orm::sea_query::{Expr, Func, Query};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel,
    QueryFilter, TransactionTrait,
};
use sha2::{Digest, Sha256};

fn now() -> chrono::NaiveDateTime {
    chrono::Utc::now().naive_utc()
}

async fn seed(fixture: &Fixture) -> (api_sessions::Model, HeaderMap) {
    let token = uuid::Uuid::new_v4().to_string();
    let now = now();
    let session = api_sessions::ActiveModel {
        account_id: Set(fixture.account.id),
        household_membership_id: Set(Some(fixture.membership.id)),
        access_token_digest: Set(hex::encode(Sha256::digest(token.as_bytes()))),
        refresh_token_digest: Set(hex::encode(Sha256::digest(uuid::Uuid::new_v4().as_bytes()))),
        permissions_version: Set(fixture.membership.permissions_version),
        access_expires_at: Set(now + chrono::Duration::minutes(15)),
        refresh_expires_at: Set(now - chrono::Duration::days(1)),
        last_used_at: Set(now - chrono::Duration::hours(1)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    (session, headers)
}

#[tokio::test]
async fn api_session_retained_principal_preserves_target_context_and_canonical_actor() {
    let fixture = Fixture::new().await;
    let (session, headers) = seed(&fixture).await;
    let principal = api_session::authenticate(&fixture.runtime, &headers, None).await;
    assert!(
        principal.is_ok(),
        "A valid persisted user API session must authenticate"
    );
    let principal = principal.unwrap();
    let target = fixture.foreign_household().await;
    let invitation = uuid::Uuid::new_v4().to_string();
    let transaction = fixture.runtime.begin().await.unwrap();
    let settings = Query::select()
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.current_household_id"),
            Expr::val(target.household.id.to_string()),
            Expr::val(true),
        ]))
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.current_membership_id"),
            Expr::val(target.membership.id.to_string()),
            Expr::val(true),
        ]))
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.current_invitation_token_digest"),
            Expr::val(invitation.clone()),
            Expr::val(true),
        ]))
        .to_owned();
    transaction
        .execute_raw(transaction.get_database_backend().build(&settings))
        .await
        .unwrap();
    let actor = principal.revalidate(&transaction, None).await.unwrap();
    assert_eq!(
        (actor.account_id, actor.person_id, actor.session_id),
        (fixture.account.id, fixture.adult.id, session.id)
    );
    assert_eq!(actor.person_id, fixture.adult.id);
    assert_eq!(actor.email, fixture.account.email);
    let settings = Query::select()
        .expr_as(
            Func::cust("current_setting").args([Expr::val("med_tracker.current_household_id")]),
            "household",
        )
        .expr_as(
            Func::cust("current_setting").args([Expr::val("med_tracker.current_membership_id")]),
            "membership",
        )
        .expr_as(
            Func::cust("current_setting")
                .args([Expr::val("med_tracker.current_invitation_token_digest")]),
            "invitation",
        )
        .to_owned();
    let row = transaction
        .query_one_raw(transaction.get_database_backend().build(&settings))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.try_get::<String>("", "household").unwrap(),
        target.household.id.to_string()
    );
    assert_eq!(
        row.try_get::<String>("", "membership").unwrap(),
        target.membership.id.to_string()
    );
    assert_eq!(row.try_get::<String>("", "invitation").unwrap(), invitation);
    transaction.commit().await.unwrap();
    let touched = api_sessions::Entity::find_by_id(session.id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(touched.last_used_at > now() - chrono::Duration::minutes(1));
    target.close().await;
    fixture.close().await;
}

#[tokio::test]
async fn api_session_rejects_retargeting_to_an_existing_different_person_membership()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new().await;
    let (session, headers) = seed(&fixture).await;
    let principal = api_session::authenticate(&fixture.runtime, &headers, None)
        .await
        .unwrap();
    let target = household_memberships::Entity::find_by_id(fixture.dependent_membership.id)
        .one(&fixture.admin)
        .await?
        .unwrap();
    assert_eq!(target.household_id, fixture.household.id);
    assert_eq!(target.person_id, fixture.dependent.id);
    assert_eq!(target.status, "active");
    assert!(target.revoked_at.is_none());
    let changed = api_sessions::Entity::update_many()
        .col_expr(
            api_sessions::Column::HouseholdMembershipId,
            Expr::val(target.id),
        )
        .filter(api_sessions::Column::Id.eq(session.id))
        .filter(api_sessions::Column::AccountId.eq(fixture.account.id))
        .filter(api_sessions::Column::HouseholdMembershipId.eq(fixture.membership.id))
        .exec(&fixture.admin)
        .await?;
    let transaction = fixture.runtime.begin().await?;
    let retained = principal.revalidate(&transaction, None).await;
    transaction.rollback().await?;
    let fresh = api_session::authenticate(&fixture.runtime, &headers, None).await;
    fixture.close().await;
    assert_eq!(changed.rows_affected, 1);
    assert!(matches!(
        retained,
        Err(AuthenticationError::Unauthenticated)
    ));
    assert!(matches!(fresh, Err(AuthenticationError::Unauthenticated)));
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum AuthorityChange {
    Expired,
    Revoked,
    Rotated,
    Permissions,
    MembershipRevoked,
    MembershipStatus,
    Unbound,
    AccountClosed,
    Lockout,
    AccountInactive,
    HouseholdArchived,
}

async fn change_authority(
    fixture: &Fixture,
    session: &api_sessions::Model,
    change: AuthorityChange,
    restore: bool,
) {
    match change {
        AuthorityChange::Expired
        | AuthorityChange::Revoked
        | AuthorityChange::Rotated
        | AuthorityChange::Unbound => {
            let mut row = api_sessions::Entity::find_by_id(session.id)
                .one(&fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .into_active_model();
            match change {
                AuthorityChange::Expired => {
                    row.access_expires_at = Set(if restore {
                        now() + chrono::Duration::minutes(15)
                    } else {
                        now()
                    })
                }
                AuthorityChange::Revoked => {
                    row.revoked_at = Set(if restore { None } else { Some(now()) })
                }
                AuthorityChange::Rotated => {
                    row.access_token_digest = Set(if restore {
                        session.access_token_digest.clone()
                    } else {
                        uuid::Uuid::new_v4().to_string()
                    })
                }
                AuthorityChange::Unbound => {
                    row.household_membership_id = Set(if restore {
                        Some(fixture.membership.id)
                    } else {
                        None
                    })
                }
                _ => unreachable!(),
            }
            row.update(&fixture.admin).await.unwrap();
        }
        AuthorityChange::Permissions
        | AuthorityChange::MembershipRevoked
        | AuthorityChange::MembershipStatus => {
            let mut row = household_memberships::Entity::find_by_id(fixture.membership.id)
                .one(&fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .into_active_model();
            match change {
                AuthorityChange::Permissions => {
                    row.permissions_version =
                        Set(fixture.membership.permissions_version + i32::from(!restore))
                }
                AuthorityChange::MembershipRevoked => {
                    row.revoked_at = Set(if restore { None } else { Some(now()) })
                }
                AuthorityChange::MembershipStatus => {
                    row.status = Set(if restore { "active" } else { "revoked" }.into())
                }
                _ => unreachable!(),
            }
            row.update(&fixture.admin).await.unwrap();
        }
        AuthorityChange::AccountClosed => {
            let mut row = fixture.account.clone().into_active_model();
            row.status = Set(if restore { 2 } else { 3 });
            row.update(&fixture.admin).await.unwrap();
        }
        AuthorityChange::Lockout => {
            if restore {
                account_lockouts::Entity::delete_by_id(fixture.account.id)
                    .exec(&fixture.admin)
                    .await
                    .unwrap();
            } else {
                account_lockouts::ActiveModel {
                    account_id: Set(fixture.account.id),
                    deadline: Set(now() + chrono::Duration::hours(1)),
                    key: Set(uuid::Uuid::new_v4().to_string()),
                    created_at: Set(Some(now())),
                    updated_at: Set(Some(now())),
                    ..Default::default()
                }
                .insert(&fixture.admin)
                .await
                .unwrap();
            }
        }
        AuthorityChange::AccountInactive => {
            let mut row = fixture.account.clone().into_active_model();
            row.active = Set(restore);
            row.update(&fixture.admin).await.unwrap();
        }
        AuthorityChange::HouseholdArchived => {
            let mut row = fixture.household.clone().into_active_model();
            row.lifecycle_state = Set(if restore { "active" } else { "archived" }.into());
            row.update(&fixture.admin).await.unwrap();
        }
    }
}

#[tokio::test]
async fn api_session_revalidation_rejects_changed_live_authority() {
    let fixture = Fixture::new().await;
    let (session, headers) = seed(&fixture).await;
    let principal = api_session::authenticate(&fixture.runtime, &headers, None)
        .await
        .unwrap();
    for change in [
        AuthorityChange::Expired,
        AuthorityChange::Revoked,
        AuthorityChange::Rotated,
        AuthorityChange::Permissions,
        AuthorityChange::MembershipRevoked,
        AuthorityChange::MembershipStatus,
        AuthorityChange::Unbound,
        AuthorityChange::AccountClosed,
        AuthorityChange::Lockout,
        AuthorityChange::AccountInactive,
        AuthorityChange::HouseholdArchived,
    ] {
        change_authority(&fixture, &session, change, false).await;
        let transaction = fixture.runtime.begin().await.unwrap();
        assert!(
            matches!(
                principal.revalidate(&transaction, None).await,
                Err(AuthenticationError::Unauthenticated)
            ),
            "Changed authority was accepted: {change:?}"
        );
        transaction.rollback().await.unwrap();
        change_authority(&fixture, &session, change, true).await;
    }
    fixture.close().await;
}

#[tokio::test]
async fn api_session_missing_or_malformed_credentials_fail_closed() {
    let fixture = Fixture::new().await;
    for value in [
        None,
        Some("Basic cGFzc3dvcmQ="),
        Some("Bearer absent"),
        Some("Bearer"),
    ] {
        let mut headers = HeaderMap::new();
        if let Some(value) = value {
            headers.insert(header::AUTHORIZATION, HeaderValue::from_str(value).unwrap());
        }
        assert!(matches!(
            api_session::authenticate(&fixture.runtime, &headers, None).await,
            Err(AuthenticationError::Unauthenticated)
        ));
    }
    fixture.close().await;
}

#[tokio::test]
async fn api_session_app_token_classification_respects_configured_maximum_age() {
    let fixture = Fixture::new().await;
    let token = uuid::Uuid::new_v4().to_string();
    let now = now();
    let app_token = api_app_tokens::ActiveModel {
        account_id: Set(fixture.account.id),
        household_membership_id: Set(fixture.membership.id),
        token_digest: Set(hex::encode(Sha256::digest(token.as_bytes()))),
        permissions_version: Set(fixture.membership.permissions_version),
        name: Set("Synthetic application".into()),
        expires_at: Set(now + chrono::Duration::days(1)),
        last_used_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    assert!(matches!(
        api_session::authenticate(&fixture.runtime, &headers, None).await,
        Err(AuthenticationError::Forbidden)
    ));
    let mut app_token = app_token.into_active_model();
    app_token.created_at = Set(now - chrono::Duration::days(36_525));
    app_token.update(&fixture.admin).await.unwrap();
    assert!(
        matches!(
            api_session::authenticate(&fixture.runtime, &headers, None).await,
            Err(AuthenticationError::Unauthenticated)
        ),
        "A stored future expiry cannot extend the configured application-token maximum age"
    );
    fixture.close().await;
}
