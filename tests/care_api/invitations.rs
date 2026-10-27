use super::*;
use med_tracker::models::_entities::{
    account_lockouts, api_change_events, carer_relationships, household_invitation_grants,
    household_invitations, person_access_grants,
};
use sea_orm::{
    QueryResult,
    sea_query::{Alias, Expr, ExprTrait, Func, Query},
};

async fn seed_invitation(
    app: &Application,
    email: &str,
    digest: String,
    expiry: chrono::NaiveDateTime,
) -> household_invitations::Model {
    let now = chrono::Utc::now().naive_utc();
    household_invitations::ActiveModel {
        household_id: Set(app.fixture.household.id),
        email: Set(email.into()),
        membership_role: Set("member".into()),
        token_digest: Set(digest),
        invited_by_membership_id: Set(app.fixture.membership.id),
        expires_at: Set(expiry),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap()
}

async fn invitation_effects(
    app: &Application,
    household: i64,
) -> std::collections::BTreeMap<String, i64> {
    let mut state = std::collections::BTreeMap::new();
    for (name, table) in [
        ("attempts", "invitation_mail_attempts"),
        ("invitations", "household_invitations"),
        ("grants", "household_invitation_grants"),
        ("keys", "api_idempotency_keys"),
    ] {
        let query = Query::select()
            .expr_as(Expr::col("id").count(), "count")
            .from(table)
            .and_where(Expr::col("household_id").eq(household))
            .to_owned();
        let count = app
            .fixture
            .admin
            .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
            .unwrap()
            .unwrap()
            .try_get::<i64>("", "count")
            .unwrap();
        state.insert(name.into(), count);
    }
    let versions = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(household))
        .filter(versions::Column::ItemType.eq("HouseholdInvitation"))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    state.insert("versions".into(), i64::try_from(versions).unwrap());
    state.insert(
        "jobs".into(),
        i64::try_from(mail_jobs(app, household).await.len()).unwrap(),
    );
    state
}

async fn mail_jobs(app: &Application, household: i64) -> Vec<(String, Value)> {
    let query = Query::select()
        .columns(["name", "task_data"])
        .from("pg_loco_queue")
        .and_where(
            Func::cust("jsonb_extract_path_text")
                .args([Expr::col("task_data"), Expr::val("household_id")])
                .eq(household.to_string()),
        )
        .order_by("id", sea_orm::Order::Asc)
        .to_owned();
    app.fixture
        .admin
        .query_all_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .into_iter()
        .map(|row| {
            (
                row.try_get("", "name").unwrap(),
                row.try_get("", "task_data").unwrap(),
            )
        })
        .collect()
}

async fn attempt(app: &Application, id: i64) -> QueryResult {
    let query = Query::select()
        .columns([
            "status",
            "category",
            "invitation_id",
            "started_at",
            "finished_at",
        ])
        .expr_as(
            Func::cust("to_jsonb")
                .arg(Expr::col("a"))
                .cast_as(Alias::new("text")),
            "safe_record",
        )
        .from_as("invitation_mail_attempts", "a")
        .and_where(Expr::col("id").eq(id))
        .to_owned();
    app.fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
}

async fn update_attempt(
    app: &Application,
    id: i64,
    column: &str,
    value: impl Into<sea_orm::Value>,
) {
    let query = Query::update()
        .table("invitation_mail_attempts")
        .value(Alias::new(column), Expr::val(value.into()))
        .and_where(Expr::col("id").eq(id))
        .to_owned();
    app.fixture
        .admin
        .execute_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap();
}

async fn acceptance_effects(app: &Application, acceptance: &Acceptance) -> Vec<i64> {
    let household = acceptance.target_household.id;
    let memberships = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(household))
        .filter(household_memberships::Column::PersonId.eq(app.fixture.adult.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let grants = person_access_grants::Entity::find()
        .filter(person_access_grants::Column::HouseholdId.eq(household))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let relationships = carer_relationships::Entity::find()
        .filter(carer_relationships::Column::HouseholdId.eq(household))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let changes = api_change_events::Entity::find()
        .filter(api_change_events::Column::HouseholdId.eq(household))
        .filter(api_change_events::Column::RecordType.eq("Person"))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    vec![
        i64::try_from(memberships).unwrap(),
        i64::try_from(memberships).unwrap(),
        i64::try_from(grants).unwrap(),
        i64::try_from(relationships).unwrap(),
        i64::try_from(changes).unwrap(),
    ]
}

async fn fault(app: &Application, label: &str) -> (String, String) {
    let function = format!("{label}_{}", uuid::Uuid::new_v4().simple());
    let sequence = format!("{function}_probe");
    // SeaORM has no trigger, PL/pgSQL function or GRANT builder.
    app.fixture.admin.execute_unprepared(&format!("CREATE SEQUENCE {sequence}; GRANT USAGE,SELECT ON SEQUENCE {sequence} TO med_tracker_app; CREATE FUNCTION {function}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('{sequence}'); RAISE EXCEPTION 'Synthetic invitation enqueue rejection'; END $$; CREATE TRIGGER {function} BEFORE INSERT ON pg_loco_queue FOR EACH ROW EXECUTE FUNCTION {function}()")).await.unwrap();
    (function, sequence)
}

async fn fault_reached(app: &Application, sequence: &str) -> bool {
    let query = Query::select()
        .column("is_called")
        .from(Alias::new(sequence))
        .to_owned();
    app.fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "is_called")
        .unwrap()
}

async fn remove_fault(app: &Application, function: &str, sequence: &str) {
    app.fixture.admin.execute_unprepared(&format!("DROP TRIGGER {function} ON pg_loco_queue; DROP FUNCTION {function}(); DROP SEQUENCE {sequence}")).await.unwrap();
}

#[path = "invitations/browser_administration.rs"]
mod browser_administration;
#[path = "invitations/delivery_race.rs"]
mod delivery_race;
#[path = "invitations/global_person.rs"]
mod global_person;

#[tokio::test]
async fn invitations_api_collection_retains_newest_hundred_limit() {
    let app = Application::new().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let prefix = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();
    for n in 1..=101 {
        let mut row = seed_invitation(
            &app,
            &format!("synthetic-limit-{prefix}-{n}@example.test"),
            uuid::Uuid::new_v4().to_string(),
            now + chrono::Duration::days(7),
        )
        .await
        .into_active_model();
        row.created_at = Set(now + chrono::Duration::seconds(n));
        row.update(&app.fixture.admin).await.unwrap();
    }
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{household_id}/admin/invitations",
            app.origin
        ))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    app.close().await;
    assert_eq!(status, 200);
    let rows = body["data"].as_array().unwrap();
    assert_eq!(rows.len(), 100);
    assert_eq!(
        rows[0]["email"],
        format!("synthetic-limit-{prefix}-101@example.test")
    );
    assert_eq!(
        rows[99]["email"],
        format!("synthetic-limit-{prefix}-2@example.test")
    );
}

#[tokio::test]
async fn invitations_resend_rotates_once_replay_does_not_send_again_and_revoke_invalidates_token() {
    use med_tracker::models::care::invitations;
    use sha2::{Digest, Sha256};
    let app = Application::new().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let original = uuid::Uuid::new_v4().to_string();
    let email = format!("rotation-{}@example.test", uuid::Uuid::new_v4());
    let replay_key = uuid::Uuid::new_v4().to_string();
    let invitation = seed_invitation(
        &app,
        &email,
        hex::encode(Sha256::digest(original.as_bytes())),
        "2020-01-01T00:00:00".parse().unwrap(),
    )
    .await;
    let invitation_id = invitation.id;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/admin/invitations/{invitation_id}",
        app.origin
    );
    let first = app
        .client
        .post(format!("{endpoint}/resend"))
        .bearer_auth(&token)
        .header("idempotency-key", &replay_key)
        .send()
        .await
        .unwrap();
    let repeated = app
        .client
        .post(format!("{endpoint}/resend"))
        .bearer_auth(&token)
        .header("idempotency-key", &replay_key)
        .send()
        .await
        .unwrap();
    let statuses = (first.status().as_u16(), repeated.status().as_u16());
    let replayed = repeated
        .headers()
        .get("idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let jobs = mail_jobs(&app, household_id).await;
    let data = &jobs[0].1;
    let link = data["text"]
        .as_str()
        .unwrap()
        .split_whitespace()
        .last()
        .unwrap();
    let target = url::Url::parse(link).unwrap();
    let new_token = target
        .query_pairs()
        .find(|(name, _)| name == "token")
        .unwrap()
        .1
        .into_owned();
    let versions = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(household_id))
        .filter(versions::Column::ItemType.eq("HouseholdInvitation"))
        .filter(versions::Column::Event.eq("resend"))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let jobs = jobs.len();
    let old_available = invitations::preview(&app.fixture.runtime, &original)
        .await
        .is_ok();
    let new_available = invitations::preview(&app.fixture.runtime, &new_token)
        .await
        .is_ok();
    let revoked = app
        .client
        .delete(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let revoked_status = revoked.status().as_u16();
    let revoked_available = invitations::preview(&app.fixture.runtime, &new_token)
        .await
        .is_ok();
    let forbidden = app
        .client
        .post(format!("{endpoint}/resend"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let forbidden_status = forbidden.status().as_u16();
    app.close().await;
    assert_eq!(statuses, (200, 200));
    assert_eq!(replayed, "true");
    assert_eq!((jobs, versions), (1, 1));
    assert!(!old_available && new_available && !revoked_available);
    assert_eq!((revoked_status, forbidden_status), (204, 422));
}

#[tokio::test]
async fn invitations_resend_queue_rejection_returns_503_without_rotation_or_delivery() {
    let app = Application::new().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let email = format!("resend-{}@example.test", uuid::Uuid::new_v4());
    let invitation = seed_invitation(
        &app,
        &email,
        uuid::Uuid::new_v4().to_string(),
        "2020-01-01T00:00:00".parse().unwrap(),
    )
    .await;
    let invitation_id = invitation.id;
    let (failure, probe) = fault(&app, "resend_queue").await;
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/admin/invitations/{invitation_id}/resend",
            app.origin
        ))
        .bearer_auth(&token)
        .header(
            "idempotency-key",
            format!("{}-synthetic-failed-resend", app.fixture.client_uuid),
        )
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let no_store = response
        .headers()
        .get("cache-control")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    let reached = fault_reached(&app, &probe).await;
    remove_fault(&app, &failure, &probe).await;
    let stored = household_invitations::Entity::find_by_id(invitation.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let effects = invitation_effects(&app, household_id).await;
    let counts = vec![
        i64::from(
            stored.token_digest == invitation.token_digest
                && stored.expires_at == invitation.expires_at,
        ),
        effects["jobs"],
        effects["versions"],
        effects["keys"],
        effects["attempts"],
    ];
    app.close().await;
    assert_eq!(status, 503);
    assert_eq!(body["error"]["code"], "invitation_delivery_unavailable");
    assert_eq!(no_store, "no-store");
    assert!(reached);
    assert_eq!(counts, vec![1, 0, 0, 0, 0]);
}

pub(super) struct Acceptance {
    pub bearer: String,
    pub token: String,
    pub invitation: med_tracker::models::_entities::household_invitations::Model,
    pub inviter_account: med_tracker::models::_entities::accounts::Model,
    pub inviter_membership: med_tracker::models::_entities::household_memberships::Model,
    pub target_household: med_tracker::models::_entities::households::Model,
    pub target_slug: String,
    pub invited_child: med_tracker::models::_entities::people::Model,
    pub invitation_grant: med_tracker::models::_entities::household_invitation_grants::Model,
}

pub(super) async fn seed_acceptance(app: &Application) -> Acceptance {
    use med_tracker::models::_entities::{
        api_sessions, household_invitation_grants, household_invitations, people,
    };
    use sha2::{Digest, Sha256};
    let bearer = uuid::Uuid::new_v4().to_string();
    let token = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();
    api_sessions::ActiveModel {
        account_id: Set(app.fixture.account.id),
        household_membership_id: Set(Some(app.fixture.membership.id)),
        access_token_digest: Set(hex::encode(Sha256::digest(bearer.as_bytes()))),
        refresh_token_digest: Set(uuid::Uuid::new_v4().to_string()),
        permissions_version: Set(app.fixture.membership.permissions_version),
        access_expires_at: Set(now + chrono::Duration::hours(1)),
        refresh_expires_at: Set(now + chrono::Duration::days(1)),
        last_used_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let inviter = app.fixture.foreign_household().await;
    let mut membership = inviter.membership.clone().into_active_model();
    membership.role = Set("owner".into());
    let inviter_membership = membership.update(&app.fixture.admin).await.unwrap();
    let invited_child = people::ActiveModel {
        name: Set("Synthetic invited child".into()),
        person_type: Set(1),
        has_capacity: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    household_memberships::ActiveModel {
        household_id: Set(inviter.household.id),
        person_id: Set(invited_child.id),
        role: Set("member".into()),
        status: Set("active".into()),
        permissions_version: Set(1),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let invitation = household_invitations::ActiveModel {
        household_id: Set(inviter.household.id),
        email: Set(app.fixture.account.email.clone()),
        membership_role: Set("member".into()),
        token_digest: Set(hex::encode(Sha256::digest(token.as_bytes()))),
        invited_by_membership_id: Set(inviter_membership.id),
        expires_at: Set(now + chrono::Duration::days(7)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let invitation_grant = household_invitation_grants::ActiveModel {
        household_id: Set(inviter.household.id),
        household_invitation_id: Set(invitation.id),
        person_id: Set(invited_child.id),
        access_level: Set("manage".into()),
        relationship_type: Set("parent".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let result = Acceptance {
        bearer,
        token,
        invitation,
        inviter_account: inviter.account.clone(),
        inviter_membership,
        target_household: inviter.household.clone(),
        target_slug: inviter.household.slug.clone(),
        invited_child,
        invitation_grant,
    };
    inviter.close().await;
    result
}

#[tokio::test]
async fn invitations_user_session_acceptance_replays_once_and_never_restores_revoked_access() {
    let app = Application::new().await;
    let acceptance = seed_acceptance(&app).await;
    let access = &acceptance.bearer;
    let endpoint = format!("{}/api/v1/invitations/accept", app.origin);
    let body = json!({"token":acceptance.token});
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(access)
        .json(&body)
        .send()
        .await
        .unwrap();
    let first_status = first.status().as_u16();
    let first: Value = first.json().await.unwrap_or(Value::Null);
    let repeated = app
        .client
        .post(&endpoint)
        .bearer_auth(access)
        .json(&body)
        .send()
        .await
        .unwrap();
    let repeated_status = repeated.status().as_u16();
    let repeated: Value = repeated.json().await.unwrap_or(Value::Null);
    let counts = acceptance_effects(&app, &acceptance).await;
    household_memberships::Entity::update_many()
        .col_expr(
            household_memberships::Column::Status,
            Expr::value("revoked"),
        )
        .col_expr(
            household_memberships::Column::RevokedAt,
            Expr::value(chrono::Utc::now().naive_utc()),
        )
        .filter(household_memberships::Column::HouseholdId.eq(acceptance.target_household.id))
        .filter(household_memberships::Column::PersonId.eq(app.fixture.adult.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let revoked = app
        .client
        .post(&endpoint)
        .bearer_auth(access)
        .json(&body)
        .send()
        .await
        .unwrap();
    let revoked_status = revoked.status().as_u16();
    app.close().await;
    assert_eq!(
        (first_status, repeated_status, revoked_status),
        (200, 200, 422)
    );
    assert_eq!(first, repeated);
    assert_eq!(counts, vec![1, 1, 0, 0, 1]);
}

#[tokio::test]
async fn invitations_expiry_and_current_inviter_authority_prevent_partial_acceptance() {
    let app = Application::new().await;
    let acceptance = seed_acceptance(&app).await;
    let access = &acceptance.bearer;
    let endpoint = format!("{}/api/v1/invitations/accept", app.origin);
    let body = json!({"token":acceptance.token});
    let mut invitation = acceptance.invitation.clone().into_active_model();
    invitation.expires_at = Set(chrono::Utc::now().naive_utc() - chrono::Duration::seconds(1));
    invitation.update(&app.fixture.admin).await.unwrap();
    let expired = app
        .client
        .post(&endpoint)
        .bearer_auth(access)
        .json(&body)
        .send()
        .await
        .unwrap();
    let mut invitation = acceptance.invitation.clone().into_active_model();
    invitation.expires_at = Set(chrono::Utc::now().naive_utc() + chrono::Duration::days(1));
    invitation.update(&app.fixture.admin).await.unwrap();
    let mut inviter = acceptance.inviter_membership.clone().into_active_model();
    inviter.role = Set("member".into());
    inviter.update(&app.fixture.admin).await.unwrap();
    let demoted = app
        .client
        .post(&endpoint)
        .bearer_auth(access)
        .json(&body)
        .send()
        .await
        .unwrap();
    let people = acceptance_effects(&app, &acceptance).await[0];
    let statuses = (expired.status().as_u16(), demoted.status().as_u16());
    app.close().await;
    assert_eq!(statuses, (422, 422));
    assert_eq!(people, 0);
}

#[tokio::test]
async fn invitations_mail_queue_failure_rolls_back_invitation_and_audit() {
    use med_tracker::models::{care::invitations, identity::resource};
    let app = Application::new().await;
    let recipient = format!("invitation-{}@example.test", uuid::Uuid::new_v4());
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("authorization", format!("Bearer {token}").parse().unwrap());
    let principal = resource::authenticate(&app.fixture.runtime, &headers, None)
        .await
        .unwrap();
    let (failure, probe) = fault(&app, "invitation_queue").await;
    let tenant = principal
        .begin_household(
            &app.fixture.runtime,
            household_id,
            "synthetic-invitation-rollback".into(),
            None,
        )
        .await
        .unwrap();
    let target = url::Url::parse("https://synthetic.example.test/invitations/accept").unwrap();
    let result = invitations::create(
        &tenant,
        invitations::InvitationInput::from_form(&recipient, "member", false),
        Some(&target),
        Some(principal.provenance()),
    )
    .await;
    tenant.rollback().await.unwrap();
    let reached = fault_reached(&app, &probe).await;
    remove_fault(&app, &failure, &probe).await;
    let effects = invitation_effects(&app, household_id).await;
    let counts = vec![
        effects["invitations"],
        effects["grants"],
        effects["jobs"],
        effects["versions"],
        effects["attempts"],
    ];
    app.close().await;
    assert!(result.is_err());
    assert!(reached);
    assert_eq!(counts, vec![0, 0, 0, 0, 0]);
}

#[tokio::test]
async fn invitations_mail_committed_job_has_durable_attempt_and_provider_receipt() {
    use loco_rs::{app::Hooks, mailer::EmailSender};
    use med_tracker::models::{care::invitations, identity::resource};
    crate::reference_fixture::clean_queue().await;
    let mut app = Application::new().await;
    let recipient = format!("invitation-{}@example.test", uuid::Uuid::new_v4());
    let household_id = app.fixture.household.id;
    app.context.mailer = Some(EmailSender::stub());
    let token = app.token().await;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("authorization", format!("Bearer {token}").parse().unwrap());
    let principal = resource::authenticate(&app.fixture.runtime, &headers, None)
        .await
        .unwrap();
    let tenant = principal
        .begin_household(
            &app.fixture.runtime,
            household_id,
            "synthetic-invitation-delivery".into(),
            None,
        )
        .await
        .unwrap();
    let target = url::Url::parse("https://synthetic.example.test/invitations/accept").unwrap();
    let created = invitations::create(
        &tenant,
        invitations::InvitationInput::from_form(&recipient, "member", false),
        Some(&target),
        Some(principal.provenance()),
    )
    .await
    .unwrap();
    tenant.commit().await.unwrap();
    let jobs = mail_jobs(&app, household_id).await;
    assert_eq!(jobs.len(), 1);
    let attempt_id = jobs[0].1["attempt_id"].as_i64().unwrap();
    let initial_attempt = attempt(&app, attempt_id).await;
    assert_eq!(
        initial_attempt.try_get::<String>("", "status").unwrap(),
        "queued"
    );
    assert_eq!(
        initial_attempt
            .try_get::<Option<chrono::NaiveDateTime>>("", "started_at")
            .unwrap(),
        None
    );
    assert_eq!(
        initial_attempt
            .try_get::<Option<chrono::NaiveDateTime>>("", "finished_at")
            .unwrap(),
        None
    );
    assert!(jobs[0].0.ends_with("InvitationMailWorker"));
    let queue = app.context.queue_provider.as_ref().unwrap().clone();
    med_tracker::app::App::connect_workers(&app.context, &queue)
        .await
        .unwrap();
    let worker = tokio::spawn(async move { queue.run(vec![]).await });
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let receipt = attempt(&app, attempt_id).await;
            if receipt.try_get::<String>("", "status").unwrap() == "sent_to_provider"
                && receipt
                    .try_get::<Option<chrono::NaiveDateTime>>("", "started_at")
                    .unwrap()
                    .is_some()
                && receipt
                    .try_get::<Option<chrono::NaiveDateTime>>("", "finished_at")
                    .unwrap()
                    .is_some()
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    let count = app.context.mailer.as_ref().unwrap().deliveries().count;
    let grants = household_invitation_grants::Entity::find()
        .filter(household_invitation_grants::Column::HouseholdId.eq(household_id))
        .filter(household_invitation_grants::Column::PersonId.eq(app.fixture.minor.id))
        .filter(household_invitation_grants::Column::AccessLevel.eq("manage"))
        .filter(household_invitation_grants::Column::RelationshipType.eq("parent"))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    worker.abort();
    let _ = worker.await;
    app.close().await;
    assert!(created["data"]["id"].as_i64().is_some());
    assert_eq!(count, 1);
    assert_eq!(grants, 0);
}

#[tokio::test]
async fn invitations_manager_collection_create_and_validation_use_real_routes() {
    let app = Application::new().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/admin/invitations",
        app.origin
    );
    let listed = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let invalid = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"household_invitation":{"email":"invalid","membership_role":"member"}}))
        .send()
        .await
        .unwrap();
    let created = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"household_invitation":{"email":"synthetic-invite@example.test","membership_role":"member"}})).send().await.unwrap();
    let statuses = (
        listed.status().as_u16(),
        invalid.status().as_u16(),
        created.status().as_u16(),
    );
    app.close().await;
    assert_eq!(statuses, (200, 422, 201));
}

#[tokio::test]
async fn invitations_acceptance_rejects_mobile_and_resend_requires_authentication() {
    let app = Application::new().await;
    let invitation_id = seed_invitation(
        &app,
        &app.fixture.account.email,
        uuid::Uuid::new_v4().to_string(),
        (chrono::Utc::now() + chrono::Duration::days(1)).naive_utc(),
    )
    .await
    .id;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let accepted = app
        .client
        .post(format!("{}/api/v1/invitations/accept", app.origin))
        .bearer_auth(&token)
        .json(&json!({"invitation":{"token":"synthetic-unavailable-token"}}))
        .send()
        .await
        .unwrap();
    let resend = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/admin/invitations/{invitation_id}/resend",
            app.origin
        ))
        .send()
        .await
        .unwrap();
    let statuses = (accepted.status().as_u16(), resend.status().as_u16());
    app.close().await;
    assert_eq!(statuses, (403, 401));
}

#[tokio::test]
async fn invitation_list_matches_documented_contract() {
    use super::contract::{assert_value, contract, resolve};
    let app = Application::new().await;
    let recipient = format!("invitation-{}@example.test", uuid::Uuid::new_v4());
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/admin/invitations",
        app.origin
    );
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/admin/invitations"]["get"];
    assert_eq!(operation["operationId"], "listInvitations");
    assert_eq!(
        operation["responses"]["403"]["$ref"],
        "#/components/responses/Forbidden"
    );
    let forbidden = resolve(contract, &operation["responses"]["403"]);
    let forbidden_schema = resolve(
        contract,
        &forbidden["content"]["application/json"]["schema"],
    );

    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"household_invitation":{"email":&recipient,"membership_role":"member"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let list = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let list_status = list.status().as_u16();
    let body: Value = list.json().await.unwrap();
    assert_eq!(list_status, 200);
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/HouseholdInvitationCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "invitation collection",
    );
    assert_eq!(body["data"][0]["email"], recipient);
    assert_eq!(body["data"][0]["pending"], true);

    app.fixture.member().await;
    let denied = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    let denied_body: Value = denied.json().await.unwrap();
    assert_eq!(denied_status, 403);
    assert_value(
        contract,
        forbidden_schema,
        &denied_body,
        "member invitation list",
    );
    app.close().await;
}

#[tokio::test]
async fn parent_label_cannot_replace_inviter_household_authority() {
    let app = Application::new().await;
    let acceptance = seed_acceptance(&app).await;
    let access = &acceptance.bearer;
    let now = chrono::Utc::now().naive_utc();
    let mut inviter = acceptance.inviter_membership.clone().into_active_model();
    inviter.role = Set("member".into());
    inviter.update(&app.fixture.admin).await.unwrap();
    let grant = person_access_grants::ActiveModel {
        household_id: Set(acceptance.target_household.id),
        household_membership_id: Set(acceptance.inviter_membership.id),
        person_id: Set(acceptance.invited_child.id),
        access_level: Set("manage".into()),
        relationship_type: Set("parent".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let endpoint = format!("{}/api/v1/invitations/accept", app.origin);
    for mutation in 0..8 {
        let mut current = person_access_grants::Entity::find_by_id(grant.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .into_active_model();
        let mut invitation = household_invitations::Entity::find_by_id(acceptance.invitation.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .into_active_model();
        let mut metadata =
            household_invitation_grants::Entity::find_by_id(acceptance.invitation_grant.id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .into_active_model();
        let mut inviter =
            household_memberships::Entity::find_by_id(acceptance.inviter_membership.id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .into_active_model();
        match mutation {
            0 => {
                current.revoked_at = Set(Some(now));
            }
            1 => {
                current.revoked_at = Set(None);
                current.access_level = Set("record".into());
            }
            2 => {
                current.access_level = Set("manage".into());
                current.expires_at = Set(Some(now - chrono::Duration::seconds(1)));
            }
            3 => {
                current.expires_at = Set(None);
                metadata.relationship_type = Set("professional".into());
            }
            4 => {
                metadata.relationship_type = Set("parent".into());
                metadata.access_level = Set("record".into());
            }
            5 => {
                metadata.access_level = Set("manage".into());
                invitation.membership_role = Set("administrator".into());
            }
            6 => {
                invitation.membership_role = Set("member".into());
                inviter.revoked_at = Set(Some(now));
            }
            _ => {
                inviter.revoked_at = Set(None);
                let mut child = acceptance.invited_child.clone().into_active_model();
                child.person_type = Set(0);
                child.has_capacity = Set(true);
                child.update(&app.fixture.admin).await.unwrap();
            }
        }
        current.update(&app.fixture.admin).await.unwrap();
        invitation.update(&app.fixture.admin).await.unwrap();
        metadata.update(&app.fixture.admin).await.unwrap();
        inviter.update(&app.fixture.admin).await.unwrap();
        let response = app
            .client
            .post(&endpoint)
            .bearer_auth(access)
            .json(&json!({"token":acceptance.token}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 422, "{mutation}");
        let effects = acceptance_effects(&app, &acceptance).await;
        let audits = invitation_effects(&app, acceptance.target_household.id).await["versions"];
        for (field, value) in [
            ("memberships", effects[1]),
            ("relationships", effects[3]),
            ("audits", audits),
        ] {
            assert_eq!(value, 0, "{mutation}: {field}");
        }
    }
    let mut child = acceptance.invited_child.clone().into_active_model();
    child.person_type = Set(1);
    child.has_capacity = Set(false);
    child.update(&app.fixture.admin).await.unwrap();
    let accepted = app
        .client
        .post(&endpoint)
        .bearer_auth(access)
        .json(&json!({"token":acceptance.token}))
        .send()
        .await
        .unwrap();
    let status = accepted.status().as_u16();
    let effects = acceptance_effects(&app, &acceptance).await;
    let audits = invitation_effects(&app, acceptance.target_household.id).await["versions"];
    let invitation = household_invitations::Entity::find_by_id(acceptance.invitation.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let target_grants = person_access_grants::Entity::find()
        .filter(person_access_grants::Column::HouseholdId.eq(acceptance.target_household.id))
        .filter(person_access_grants::Column::PersonId.eq(app.fixture.adult.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 422);
    assert!(invitation.accepted_at.is_none());
    for value in [
        effects[1],
        effects[3],
        audits,
        i64::try_from(target_grants).unwrap(),
    ] {
        assert_eq!(value, 0);
    }
}

#[tokio::test]
async fn invitation_mail_worker_rechecks_authority_tokens_and_replay_without_resending() {
    use futures_util::FutureExt;
    use loco_rs::{bgworker::BackgroundWorker, mailer::EmailSender};
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::invitations::{
            self,
            delivery::{InvitationMail, InvitationMailWorker},
        },
    };
    use std::panic::AssertUnwindSafe;
    for scenario in [
        "sent",
        "cancelled",
        "expired",
        "accepted",
        "revoked",
        "superseded",
        "sender_revoked",
        "inviter_revoked",
        "sender_locked",
        "inviter_locked",
        "uncertain",
        "provider_failure",
        "foreign",
        "race",
        "commit_failure",
    ] {
        let mut app = Application::new().await;
        let recipient = format!("invitation-{}@example.test", uuid::Uuid::new_v4());
        let household_id = app.fixture.household.id;
        app.context.mailer = Some(EmailSender::stub());
        let scope = HouseholdScope {
            actor: Actor {
                account_id: app.fixture.account.id,
            },
            household_id: app.fixture.household.id,
            request_id: format!("invitation-worker-{scenario}"),
        };
        let tenant = access::begin(&app.fixture.runtime, &scope, None)
            .await
            .unwrap();
        let target = url::Url::parse("https://synthetic.example.test/invitations/accept").unwrap();
        let created = invitations::create(
            &tenant,
            invitations::InvitationInput::from_form(&recipient, "member", false),
            Some(&target),
            None,
        )
        .await
        .unwrap();
        let id = created["data"]["id"].as_i64().unwrap();
        tenant.commit().await.unwrap();
        if matches!(scenario, "inviter_revoked" | "inviter_locked") {
            let secondary = app.fixture.foreign_household().await;
            let now = chrono::Utc::now().naive_utc();
            household_memberships::ActiveModel {
                household_id: Set(household_id),
                person_id: Set(secondary.account.person_id),
                role: Set("administrator".into()),
                status: Set("active".into()),
                joined_at: Set(now),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&app.fixture.admin)
            .await
            .unwrap();
            let sender = HouseholdScope {
                actor: Actor {
                    account_id: secondary.account.id,
                },
                ..scope.clone()
            };
            let tenant = access::begin(&app.fixture.runtime, &sender, None)
                .await
                .unwrap();
            invitations::resend(&tenant, id, &target, None)
                .await
                .unwrap();
            tenant.commit().await.unwrap();
            secondary.close().await;
        }
        let mut payload = mail_jobs(&app, household_id).await.pop().unwrap().1;
        let attempt_id = payload["attempt_id"].as_i64().unwrap();
        let mail_before = app.context.mailer.as_ref().unwrap().deliveries().count;
        let receipt_fault = format!("invitation_receipt_{}", uuid::Uuid::new_v4().simple());
        match scenario {
            "cancelled" => {
                let tenant = access::begin(&app.fixture.runtime, &scope, None)
                    .await
                    .unwrap();
                invitations::cancel(&tenant, id, None).await.unwrap();
                tenant.commit().await.unwrap();
            }
            "expired" => {
                let mut invitation = household_invitations::Entity::find_by_id(id)
                    .one(&app.fixture.admin)
                    .await
                    .unwrap()
                    .unwrap()
                    .into_active_model();
                invitation.expires_at =
                    Set(chrono::Utc::now().naive_utc() - chrono::Duration::seconds(1));
                invitation.update(&app.fixture.admin).await.unwrap();
            }
            "accepted" => {
                let mut invitation = household_invitations::Entity::find_by_id(id)
                    .one(&app.fixture.admin)
                    .await
                    .unwrap()
                    .unwrap()
                    .into_active_model();
                invitation.accepted_at = Set(Some(chrono::Utc::now().naive_utc()));
                invitation.update(&app.fixture.admin).await.unwrap();
            }
            "revoked" => {
                let mut invitation = household_invitations::Entity::find_by_id(id)
                    .one(&app.fixture.admin)
                    .await
                    .unwrap()
                    .unwrap()
                    .into_active_model();
                invitation.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
                invitation.update(&app.fixture.admin).await.unwrap();
            }
            "superseded" => {
                let mut invitation = household_invitations::Entity::find_by_id(id)
                    .one(&app.fixture.admin)
                    .await
                    .unwrap()
                    .unwrap()
                    .into_active_model();
                invitation.token_digest = Set(uuid::Uuid::new_v4().to_string());
                invitation.update(&app.fixture.admin).await.unwrap();
            }
            "sender_revoked" | "inviter_revoked" => {
                let mut membership = app.fixture.membership.clone().into_active_model();
                membership.status = Set("revoked".into());
                membership.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
                membership.permissions_version =
                    Set(app.fixture.membership.permissions_version + 1);
                membership.update(&app.fixture.admin).await.unwrap();
            }
            "sender_locked" | "inviter_locked" => {
                account_lockouts::ActiveModel {
                    account_id: Set(app.fixture.account.id),
                    key: Set(uuid::Uuid::new_v4().to_string()),
                    deadline: Set(chrono::Utc::now().naive_utc() + chrono::Duration::hours(1)),
                    ..Default::default()
                }
                .insert(&app.fixture.admin)
                .await
                .unwrap();
            }
            "uncertain" => {
                update_attempt(
                    &app,
                    attempt_id,
                    "started_at",
                    chrono::Utc::now().naive_utc(),
                )
                .await;
            }
            "provider_failure" => app.context.mailer = None,
            "foreign" => {
                let foreign = app.fixture.foreign_household().await;
                payload["household_id"] = json!(foreign.household.id);
                foreign.close().await;
            }
            _ => {}
        }
        let mail: InvitationMail = serde_json::from_value(payload).unwrap();
        let worker = InvitationMailWorker::build(&app.context);
        if scenario == "commit_failure" {
            let observations = AssertUnwindSafe(async {
                // SeaORM has no trigger or PL/pgSQL function builder.
                app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION {receipt_fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'Synthetic terminal receipt failure'; END $$; CREATE TRIGGER {receipt_fault} BEFORE UPDATE ON invitation_mail_attempts FOR EACH ROW WHEN (NEW.id={attempt_id} AND NEW.status='sent_to_provider') EXECUTE FUNCTION {receipt_fault}()")).await.unwrap();
                let failed = worker.perform(mail.clone()).await.is_err();
                let deliveries = app.context.mailer.as_ref().unwrap().deliveries().count - mail_before;
                let started = attempt(&app, attempt_id).await;
                (
                    failed,
                    deliveries,
                    started.try_get::<String>("", "status").unwrap(),
                    started.try_get::<Option<chrono::NaiveDateTime>>("", "started_at").unwrap().is_some(),
                )
            }).catch_unwind().await;
            // Complete awaited cleanup before any assertion or resumed panic.
            app.fixture.admin.execute_unprepared(&format!("DROP TRIGGER IF EXISTS {receipt_fault} ON invitation_mail_attempts; DROP FUNCTION IF EXISTS {receipt_fault}()")).await.unwrap();
            let (failed, deliveries, status, started) = match observations {
                Ok(observations) => observations,
                Err(panic) => std::panic::resume_unwind(panic),
            };
            assert!(failed);
            assert_eq!(deliveries, 1);
            assert_eq!(status, "queued");
            assert!(started);
        } else if scenario == "race" {
            let (first, second) =
                tokio::join!(worker.perform(mail.clone()), worker.perform(mail.clone()));
            first.unwrap();
            second.unwrap();
        } else {
            worker.perform(mail.clone()).await.unwrap();
        }
        worker.perform(mail).await.unwrap();
        let attempt = attempt(&app, attempt_id).await;
        let state: String = attempt.try_get("", "status").unwrap();
        let category: Option<String> = attempt.try_get("", "category").unwrap();
        let safe: String = attempt.try_get("", "safe_record").unwrap();
        assert!(!safe.contains(&recipient));
        assert!(!safe.contains("https://"));
        let expected = match scenario {
            "sent" | "race" => "sent_to_provider",
            "foreign" => "queued",
            _ => "failed",
        };
        assert_eq!(state, expected, "{scenario}");
        if matches!(scenario, "uncertain" | "commit_failure") {
            assert_eq!(category.as_deref(), Some("uncertain"));
        }
        if scenario == "provider_failure" {
            assert_eq!(category.as_deref(), Some("provider_unavailable"));
        }
        if scenario == "cancelled" {
            assert_eq!(
                attempt.try_get::<Option<i64>>("", "invitation_id").unwrap(),
                None
            );
        }
        if let Some(mailer) = &app.context.mailer {
            assert_eq!(
                mailer.deliveries().count - mail_before,
                usize::from(matches!(scenario, "sent" | "race" | "commit_failure")),
                "{scenario}"
            );
        }
        app.close().await;
    }
}

#[tokio::test]
async fn invitation_mail_attempts_enforce_runtime_tenancy_and_immutable_provenance() {
    use med_tracker::models::{access, care::invitations};
    use sea_orm::TransactionTrait;
    let app = Application::new().await;
    let scope = app.fixture.scope();
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let email = format!("rls-attempt-{}@example.test", uuid::Uuid::new_v4());
    let created = invitations::create(
        &tenant,
        invitations::InvitationInput::from_form(&email, "member", false),
        Some(&url::Url::parse("https://synthetic.example.test/invitations/accept").unwrap()),
        None,
    )
    .await
    .unwrap();
    let id = created["data"]["id"].as_i64().unwrap();
    tenant.commit().await.unwrap();
    let foreign = app.fixture.foreign_household().await;
    for mutation in 0..3 {
        let db = app.fixture.runtime.begin().await.unwrap();
        let context = Query::select()
            .expr(Func::cust("set_config").args([
                Expr::val("med_tracker.current_household_id"),
                Expr::val(app.fixture.household.id.to_string()),
                Expr::val(true),
            ]))
            .to_owned();
        db.execute_raw(db.get_database_backend().build(&context))
            .await
            .unwrap();
        let denied = if mutation == 2 {
            let query = Query::delete()
                .from_table("invitation_mail_attempts")
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .to_owned();
            db.execute_raw(db.get_database_backend().build(&query))
                .await
        } else {
            let mut query = Query::update();
            query
                .table("invitation_mail_attempts")
                .and_where(Expr::col("household_id").eq(app.fixture.household.id));
            if mutation == 0 {
                query.value("sender_account_id", app.fixture.account.id);
            } else {
                query.value("token_digest", uuid::Uuid::new_v4().to_string());
            }
            db.execute_raw(db.get_database_backend().build(&query))
                .await
        };
        assert!(denied.is_err(), "{mutation}");
        db.rollback().await.unwrap();
    }
    let db = app.fixture.runtime.begin().await.unwrap();
    let context = Query::select()
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.current_household_id"),
            Expr::val(foreign.household.id.to_string()),
            Expr::val(true),
        ]))
        .to_owned();
    db.execute_raw(db.get_database_backend().build(&context))
        .await
        .unwrap();
    let query = Query::select()
        .expr_as(Expr::col("id").count(), "count")
        .expr_as(
            Expr::custom_keyword("CURRENT_USER").cast_as(Alias::new("text")),
            "role",
        )
        .from("invitation_mail_attempts")
        .and_where(Expr::col("household_id").eq(app.fixture.household.id))
        .to_owned();
    let visible = db
        .query_one_raw(db.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        visible.try_get::<String>("", "role").unwrap(),
        url::Url::parse(&app.fixture.runtime_uri)
            .unwrap()
            .username()
    );
    let role_query = Query::select()
        .columns([
            "rolcanlogin",
            "rolsuper",
            "rolcreatedb",
            "rolcreaterole",
            "rolreplication",
            "rolbypassrls",
        ])
        .expr_as(
            Func::cust("pg_has_role").args([
                Expr::custom_keyword("CURRENT_USER"),
                Expr::val("med_tracker_app"),
                Expr::val("MEMBER"),
            ]),
            "runtime_member",
        )
        .from("pg_roles")
        .and_where(Expr::col("rolname").eq(Expr::custom_keyword("CURRENT_USER")))
        .to_owned();
    let runtime_role = db
        .query_one_raw(db.get_database_backend().build(&role_query))
        .await
        .unwrap()
        .unwrap();
    assert!(runtime_role.try_get::<bool>("", "rolcanlogin").unwrap());
    assert!(runtime_role.try_get::<bool>("", "runtime_member").unwrap());
    for attribute in [
        "rolsuper",
        "rolcreatedb",
        "rolcreaterole",
        "rolreplication",
        "rolbypassrls",
    ] {
        assert!(
            !runtime_role.try_get::<bool>("", attribute).unwrap(),
            "{attribute}"
        );
    }
    assert_eq!(visible.try_get::<i64>("", "count").unwrap(), 0);
    let query = Query::insert()
        .into_table("invitation_mail_attempts")
        .columns([
            "household_id",
            "original_invitation_id",
            "sender_account_id",
            "sender_membership_id",
            "sender_permissions_version",
            "token_digest",
        ])
        .values_panic([
            app.fixture.household.id.into(),
            id.into(),
            app.fixture.account.id.into(),
            app.fixture.membership.id.into(),
            app.fixture.membership.permissions_version.into(),
            uuid::Uuid::new_v4().to_string().into(),
        ])
        .to_owned();
    assert!(
        db.execute_raw(db.get_database_backend().build(&query))
            .await
            .is_err()
    );
    db.rollback().await.unwrap();
    foreign.close().await;
    app.close().await;
}
