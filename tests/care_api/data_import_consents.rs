use super::Application;
use med_tracker::models::{
    _entities::{data_import_consents, household_memberships},
    access::{self, Actor, HouseholdScope},
    data_imports::{consents::{self, ConsentBinding, Decision}, contracts::{PersonId, RecordKind, SourceIdentity, SourceSystem}},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set, sea_query::Expr};

#[tokio::test]
async fn recipient_consent_is_bound_revocable_and_private() {
    let app = Application::new().await;
    let recipient = app.fixture.foreign_household().await;
    let stranger = app.fixture.foreign_household().await;
    let household = app.fixture.scenario.household.id;
    let now = chrono::Utc::now().naive_utc();
    let member = household_memberships::ActiveModel {
        household_id: Set(household), person_id: Set(recipient.account.person_id), role: Set("member".into()), status: Set("active".into()), joined_at: Set(now), created_at: Set(now), updated_at: Set(now), ..Default::default()
    }.insert(&app.fixture.admin).await.unwrap();
    let requester_scope = app.fixture.scenario.scope();
    let recipient_scope = HouseholdScope { actor: Actor { account_id: recipient.account.id }, household_id: household, request_id: uuid::Uuid::new_v4().to_string() };
    let input = ConsentBinding { source_sha256: "a".repeat(64), source_person: SourceIdentity { source: SourceSystem::MedTracker05, instance: "synthetic".into(), kind: RecordKind::Person, id: "source-person".into() }, target_person: PersonId(recipient.account.person_id) };
    let requester = access::begin(&app.fixture.runtime, &requester_scope, None).await.unwrap();
    let id = consents::request(&requester, input.clone()).await.unwrap();
    assert!(consents::read(&requester, &id).await.is_err());
    assert!(consents::decide(&requester, &id, &input, Decision::Approved).await.is_err());
    requester.commit().await.unwrap();
    let foreign = access::begin(&stranger.runtime, &stranger.scope(), None).await.unwrap();
    assert!(consents::read(&foreign, &id).await.is_err());
    assert!(consents::decide(&foreign, &id, &input, Decision::Approved).await.is_err());
    foreign.rollback().await.unwrap();
    let actor = access::begin(&app.fixture.runtime, &recipient_scope, None).await.unwrap();
    let personal = consents::read(&actor, &id).await.unwrap();
    assert_eq!(personal.binding, input);
    assert_eq!(personal.decision, Decision::Pending);
    let mut forged = input.clone();
    forged.source_sha256 = "b".repeat(64);
    assert!(consents::decide(&actor, &id, &forged, Decision::Approved).await.is_err());
    let mut forged_person = input.clone();
    forged_person.source_person.id = "other".into();
    assert!(consents::decide(&actor, &id, &forged_person, Decision::Approved).await.is_err());
    let mut forged_target = input.clone();
    forged_target.target_person = PersonId(app.fixture.scenario.account.person_id);
    assert!(consents::decide(&actor, &id, &forged_target, Decision::Approved).await.is_err());
    assert_eq!(consents::decide(&actor, &id, &input, Decision::Approved).await.unwrap().decision, Decision::Approved);
    actor.commit().await.unwrap();
    let requester = access::begin(&app.fixture.runtime, &requester_scope, None).await.unwrap();
    assert!(consents::current(&requester, &id, &input).await.unwrap());
    assert!(!consents::current(&requester, &id, &forged).await.unwrap());
    requester.commit().await.unwrap();
    let actor = access::begin(&app.fixture.runtime, &recipient_scope, None).await.unwrap();
    assert_eq!(consents::decide(&actor, &id, &input, Decision::Rejected).await.unwrap().decision, Decision::Rejected);
    actor.commit().await.unwrap();
    let requester = access::begin(&app.fixture.runtime, &requester_scope, None).await.unwrap();
    assert!(!consents::current(&requester, &id, &input).await.unwrap());
    let changed = data_import_consents::Entity::update_many().col_expr(data_import_consents::Column::Decision, Expr::value("approved")).filter(data_import_consents::Column::Id.eq(id.clone())).exec(requester.transaction()).await;
    assert!(changed.is_err() || changed.unwrap().rows_affected == 0);
    requester.rollback().await.unwrap();
    let requester = access::begin(&app.fixture.runtime, &requester_scope, None).await.unwrap();
    let approved_id = consents::request(&requester, input.clone()).await.unwrap();
    requester.commit().await.unwrap();
    let actor = access::begin(&app.fixture.runtime, &recipient_scope, None).await.unwrap();
    consents::decide(&actor, &approved_id, &input, Decision::Approved).await.unwrap();
    actor.commit().await.unwrap();
    let mut revoked = member.into_active_model();
    revoked.revoked_at = Set(Some(now));
    revoked.update(&app.fixture.admin).await.unwrap();
    let requester = access::begin(&app.fixture.runtime, &requester_scope, None).await.unwrap();
    assert!(consents::current(&requester, &approved_id, &input).await.is_err());
    requester.rollback().await.unwrap();
    assert!(access::begin(&app.fixture.runtime, &recipient_scope, None).await.is_err());
}
