use better_auth_core::store::{SessionStore, TeamMemberLimits};
use med_tracker::models::identity::better_auth::{ClinicalAuthSchema, ClinicalStore};
use sea_orm::{ConnectionTrait, DbBackend, Statement};

use super::fixture::Fixture;

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
