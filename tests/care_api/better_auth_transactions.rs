use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use better_auth_core::{
    AuthError, CreateVerification,
    store::{AuthTransaction, SessionStore, TeamMemberLimits, TransactionStore},
};
use chrono::{Duration, Utc};
use med_tracker::models::identity::better_auth::{ClinicalAuthSchema, ClinicalStore};
use sea_orm::{ConnectionTrait, DbBackend, Statement};

use super::fixture::Fixture;

fn verification(identifier: &str) -> CreateVerification {
    CreateVerification {
        identifier: identifier.to_owned().into(),
        value: "transaction-state".to_owned().into(),
        expires_at: (Utc::now() + Duration::minutes(5)).into(),
        ..Default::default()
    }
}

async fn count(fixture: &Fixture, table: &str, predicate: &str) -> i64 {
    let sql = format!("SELECT count(*) AS count FROM {table} WHERE {predicate}");
    let row = fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres, sql))
        .await
        .unwrap()
        .unwrap();
    row.try_get("", "count").unwrap()
}

#[tokio::test]
async fn retained_transaction_handles_fail_after_commit_and_queued_effect_runs_once() {
    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    let effects = Arc::new(AtomicUsize::new(0));
    let retained: Arc<Mutex<Option<Arc<dyn AuthTransaction<ClinicalAuthSchema>>>>> =
        Arc::new(Mutex::new(None));
    let effect_count = effects.clone();
    let retained_slot = retained.clone();
    TransactionStore::<ClinicalAuthSchema>::transaction_boxed(
        &store,
        Box::new(move |transaction| {
            Box::pin(async move {
                let handle = transaction.clone_handle();
                *retained_slot.lock().unwrap() = Some(handle.clone());
                transaction.queue_after_commit(Box::pin(async move {
                    assert!(handle.get_user_by_id("71001").await.is_err());
                    effect_count.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }))?;
                transaction
                    .create_verification(verification("committed-cloned-handle"))
                    .await?;
                Ok(Box::new(()) as _)
            })
        }),
    )
    .await
    .unwrap();
    assert_eq!(effects.load(Ordering::SeqCst), 1);
    let retained_handle = retained.lock().unwrap().take().unwrap();
    assert!(retained_handle.get_user_by_id("71001").await.is_err());
    assert_eq!(
        count(
            &fixture,
            "identity_verifications",
            "identifier='committed-cloned-handle'"
        )
        .await,
        1
    );
    fixture.close().await;
}

#[tokio::test]
async fn rollback_discards_effect_capturing_cloned_handle() {
    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    let effects = Arc::new(AtomicUsize::new(0));
    let effect_count = effects.clone();
    let result = TransactionStore::<ClinicalAuthSchema>::transaction_boxed(
        &store,
        Box::new(move |transaction| {
            Box::pin(async move {
                let handle = transaction.clone_handle();
                transaction.queue_after_commit(Box::pin(async move {
                    drop(handle);
                    effect_count.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }))?;
                transaction
                    .create_verification(verification("rolled-back-cloned-handle"))
                    .await?;
                Err(AuthError::internal("force rollback"))
            })
        }),
    )
    .await;
    assert!(result.is_err());
    assert_eq!(effects.load(Ordering::SeqCst), 0);
    assert_eq!(
        count(
            &fixture,
            "identity_verifications",
            "identifier='rolled-back-cloned-handle'"
        )
        .await,
        0
    );
    fixture.close().await;
}

#[tokio::test]
async fn plugin_invitation_acceptance_rejects_without_membership_mutation() {
    let fixture = Fixture::new().await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    let memberships_before = count(&fixture, "household_memberships", "true").await;
    let invitations_before = count(&fixture, "household_invitations", "true").await;
    let result = SessionStore::<ClinicalAuthSchema>::accept_invitation_with_teams(
        &store,
        "87090",
        "71001",
        None,
        true,
        TeamMemberLimits::Fixed(Some(1)),
    )
    .await;
    assert!(result.is_err());
    assert_eq!(
        count(&fixture, "household_memberships", "true").await,
        memberships_before
    );
    assert_eq!(
        count(&fixture, "household_invitations", "true").await,
        invitations_before
    );
    fixture.close().await;
}
