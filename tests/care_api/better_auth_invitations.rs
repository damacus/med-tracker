use super::fixture::Fixture;
use med_tracker::models::{
    _entities::{account_lockouts, household_invitations, household_memberships},
    entities::identity_session,
};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::Set,
    ConnectionTrait, EntityTrait, IntoActiveModel, PaginatorTrait, TransactionTrait,
    sea_query::{Expr, Func, Query},
};

async fn projection(
    fixture: &Fixture,
    invitation: i64,
    account: i64,
    token: &str,
) -> (u64, Option<i64>, u64) {
    let tx = fixture.runtime.begin().await.unwrap();
    let context = Query::select()
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.current_account_id"),
            Expr::val(account.to_string()),
            Expr::val(true),
        ]))
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.identity_session_token"),
            Expr::val(token),
            Expr::val(true),
        ]))
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.current_household_id"),
            Expr::val(""),
            Expr::val(true),
        ]))
        .to_owned();
    tx.execute_raw(tx.get_database_backend().build(&context))
        .await
        .unwrap();
    let invitations = household_invitations::Entity::find_by_id(invitation)
        .count(&tx)
        .await
        .unwrap();
    let inviter = Query::select()
        .expr_as(
            Func::cust("identity_invitation_inviter").arg(Expr::val(invitation)),
            "inviter",
        )
        .to_owned();
    let inviter = tx
        .query_one_raw(tx.get_database_backend().build(&inviter))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "inviter")
        .unwrap();
    let memberships = household_memberships::Entity::find_by_id(fixture.membership.id)
        .count(&tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    (invitations, inviter, memberships)
}

async fn recipient(
    fixture: &Fixture,
) -> (
    crate::scenario::Scenario,
    identity_session::Model,
    household_invitations::Model,
) {
    let recipient = fixture.foreign_household().await;
    let now = chrono::Utc::now();
    let session = identity_session::ActiveModel {
        user_id: Set(recipient.account.id),
        token: Set(uuid::Uuid::new_v4().to_string()),
        expires_at: Set(now + chrono::Duration::hours(1)),
        active: Set(true),
        clinical_session_purpose: Set("authenticated".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let invitation = household_invitations::ActiveModel {
        household_id: Set(fixture.household.id),
        email: Set(recipient.account.email.to_uppercase()),
        membership_role: Set("member".into()),
        token_digest: Set(uuid::Uuid::new_v4().to_string()),
        invited_by_membership_id: Set(fixture.membership.id),
        expires_at: Set((now + chrono::Duration::days(1)).naive_utc()),
        created_at: Set(now.naive_utc()),
        updated_at: Set(now.naive_utc()),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    (recipient, session, invitation)
}

#[tokio::test]
async fn better_auth_invitation_projection_reveals_only_verified_recipient_inviter_id() {
    let fixture = Fixture::new().await;
    let (recipient, session, invitation) = recipient(&fixture).await;
    assert_eq!(
        projection(
            &fixture,
            invitation.id,
            recipient.account.id,
            &session.token
        )
        .await,
        (1, Some(fixture.account.id), 0)
    );
    let other = projection(&fixture, invitation.id, fixture.account.id, &session.token).await;
    assert_eq!((other.0, other.1), (0, None));
    assert_eq!(
        projection(
            &fixture,
            invitation.id,
            recipient.account.id,
            &uuid::Uuid::new_v4().to_string()
        )
        .await,
        (0, None, 0)
    );
    let token = session.token.clone();
    let mut session = session.into_active_model();
    session.clinical_session_purpose = Set("enrolment".into());
    session.update(&fixture.admin).await.unwrap();
    assert_eq!(
        projection(&fixture, invitation.id, recipient.account.id, &token).await,
        (0, None, 0)
    );
    recipient.close().await;
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_invitation_projection_denies_expiry_revocation_and_account_lockout() {
    let fixture = Fixture::new().await;
    let (recipient, session, invitation) = recipient(&fixture).await;
    assert_eq!(
        projection(
            &fixture,
            invitation.id,
            recipient.account.id,
            &session.token
        )
        .await,
        (1, Some(fixture.account.id), 0)
    );
    let now = chrono::Utc::now().naive_utc();
    let mut expired = invitation.clone().into_active_model();
    expired.expires_at = Set(now - chrono::Duration::seconds(1));
    expired.update(&fixture.admin).await.unwrap();
    assert_eq!(
        projection(
            &fixture,
            invitation.id,
            recipient.account.id,
            &session.token
        )
        .await,
        (0, None, 0)
    );
    let mut revoked = invitation.clone().into_active_model();
    revoked.expires_at = Set(now + chrono::Duration::days(1));
    revoked.revoked_at = Set(Some(now));
    revoked.update(&fixture.admin).await.unwrap();
    assert_eq!(
        projection(
            &fixture,
            invitation.id,
            recipient.account.id,
            &session.token
        )
        .await,
        (0, None, 0)
    );
    let mut restored = invitation.clone().into_active_model();
    restored.revoked_at = Set(None);
    restored.update(&fixture.admin).await.unwrap();
    account_lockouts::ActiveModel {
        account_id: Set(recipient.account.id),
        key: Set(uuid::Uuid::new_v4().to_string()),
        deadline: Set(now + chrono::Duration::hours(1)),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    assert_eq!(
        projection(
            &fixture,
            invitation.id,
            recipient.account.id,
            &session.token
        )
        .await,
        (0, None, 0)
    );
    recipient.close().await;
    fixture.close().await;
}
