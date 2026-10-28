use super::*;
use med_tracker::models::_entities::{
    household_memberships, medication_takes, oauth_grants, people, person_medications, schedules,
    versions,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
    QuerySelect, Set,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const ENDPOINT: &str = "/api/v1/auth/profile/history";

#[path = "personal_history/refusals.rs"]
mod refusals;

async fn audits(fixture: &RequestFixture, request_id: &str) -> Vec<versions::Model> {
    versions::Entity::find()
        .filter(versions::Column::RequestId.eq(request_id))
        .filter(versions::Column::Event.is_in(["personal_history_read", "personal_history_denied"]))
        .all(&fixture.admin)
        .await
        .unwrap()
}

#[tokio::test]
async fn personal_history_oauth_reads_all_own_taken_doses_after_departure() -> TestResult {
    let (fixture, boot) = boot_request_fixture().await;
    let token = fixture.token().await;
    let assignment = person_medications::Entity::find()
        .filter(person_medications::Column::PersonId.eq(fixture.account.person_id))
        .one(&fixture.admin)
        .await?
        .ok_or("missing assignment")?;
    let now = chrono::Utc::now().naive_utc();
    for index in 0..30 {
        medication_takes::ActiveModel {
            household_id: Set(fixture.household.id),
            person_medication_id: Set(Some(assignment.id)),
            taken_from_medication_id: Set(Some(fixture.medication.id)),
            dose_amount: Set(Some(sea_orm::prelude::Decimal::new(index + 1, 0))),
            dose_unit: Set(Some("tablet".into())),
            taken_at: Set(Some(now - chrono::Duration::days(index / 10))),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&fixture.admin)
        .await?;
    }
    let foreign_medicine = fixture
        .foreign
        .medication(
            "Own medicine in departed home",
            sea_orm::prelude::Decimal::new(20, 0),
        )
        .await;
    household_memberships::ActiveModel {
        household_id: Set(fixture.foreign.household.id),
        person_id: Set(fixture.account.person_id),
        role: Set("member".into()),
        status: Set("revoked".into()),
        permissions_version: Set(1),
        joined_at: Set(now),
        revoked_at: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    let schedule = schedules::ActiveModel {
        household_id: Set(fixture.foreign.household.id),
        person_id: Set(fixture.account.person_id),
        medication_id: Set(foreign_medicine.id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    let extra = medication_takes::ActiveModel {
        household_id: Set(fixture.foreign.household.id),
        schedule_id: Set(Some(schedule.id)),
        taken_from_medication_id: Set(Some(foreign_medicine.id)),
        dose_amount: Set(Some(sea_orm::prelude::Decimal::new(125, 2))),
        dose_unit: Set(Some("tablet".into())),
        taken_at: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    let original_attribution = versions::ActiveModel {
        item_type: Set("MedicationTake".into()), item_id: Set(extra.id), event: Set("create".into()),
        household_id: Set(Some(fixture.foreign.household.id)), whodunnit: Set(Some(fixture.foreign.account.id.to_string())),
        audit_context: Set(json!({"actor_account_id":fixture.foreign.account.id,"actor_person_id":fixture.foreign.account.person_id,"household_id":fixture.foreign.household.id})),
        created_at: Set(Some(now)), ..Default::default()
    }.insert(&fixture.admin).await?;
    let private = fixture
        .foreign
        .medication(
            "PRIVATE FOREIGN MEDICINE",
            sea_orm::prelude::Decimal::new(20, 0),
        )
        .await;
    let private_assignment = fixture.foreign.personal_assignment(private.id, 0).await;
    medication_takes::ActiveModel {
        household_id: Set(fixture.foreign.household.id),
        person_medication_id: Set(Some(private_assignment.id)),
        taken_from_medication_id: Set(Some(private.id)),
        taken_at: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    let before = medication_takes::Entity::find()
        .filter(
            medication_takes::Column::HouseholdId
                .is_in([fixture.household.id, fixture.foreign.household.id]),
        )
        .order_by_asc(medication_takes::Column::Id)
        .all(&fixture.admin)
        .await?;
    let mut membership = fixture.membership.clone().into_active_model();
    membership.status = Set("revoked".into());
    membership.revoked_at = Set(Some(now));
    membership.update(&fixture.admin).await?;
    let mut person = people::Entity::find_by_id(fixture.account.person_id)
        .one(&fixture.admin)
        .await?
        .ok_or("missing person")?
        .into_active_model();
    person.person_type = Set(2);
    person.has_capacity = Set(false);
    person.update(&fixture.admin).await?;
    let request = server(&boot);
    let mut path = ENDPOINT.to_owned();
    let mut ids = Vec::new();
    for page in 0..2 {
        let response = request
            .get(&path)
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(response.status_code(), 200, "{}", response.text());
        assert_eq!(response.header("cache-control"), "no-store");
        let body: Value = response.json();
        let records = body["data"].as_array().ok_or("missing records")?;
        assert_eq!(records.len(), if page == 0 { 25 } else { 6 });
        for row in records {
            let fields: BTreeSet<_> = row
                .as_object()
                .ok_or("missing record")?
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(
                fields,
                BTreeSet::from(["id", "taken_at", "dose_amount", "dose_unit", "medicine"])
            );
            assert!(row["dose_amount"].is_string());
            ids.push(row["id"].as_i64().ok_or("missing id")?);
        }
        assert!(!response.text().contains("PRIVATE FOREIGN"));
        let audit = audits(&fixture, response.header("x-request-id").to_str()?).await;
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].item_id, fixture.account.id);
        assert_eq!(
            audit[0].audit_context["actor_person_id"],
            fixture.account.person_id
        );
        assert_eq!(audit[0].audit_context["authentication_method"], "oauth");
        assert!(audit[0].household_id.is_none());
        let next = &body["meta"]["next_cursor"];
        path = if next.is_null() {
            String::new()
        } else {
            format!(
                "{ENDPOINT}?before_id={}&before_taken_at={}",
                next["id"],
                next["taken_at"].as_str().ok_or("missing cursor time")?
            )
        };
    }
    assert!(path.is_empty());
    let mut expected: Vec<_> = before
        .iter()
        .filter(|row| row.person_medication_id == Some(assignment.id) || row.id == extra.id)
        .collect();
    expected.sort_by_key(|row| std::cmp::Reverse((row.taken_at, row.id)));
    assert_eq!(ids, expected.iter().map(|row| row.id).collect::<Vec<_>>());
    assert_eq!(
        before,
        medication_takes::Entity::find()
            .filter(
                medication_takes::Column::HouseholdId
                    .is_in([fixture.household.id, fixture.foreign.household.id])
            )
            .order_by_asc(medication_takes::Column::Id)
            .all(&fixture.admin)
            .await?
    );
    assert_eq!(
        Some(original_attribution.clone()),
        versions::Entity::find_by_id(original_attribution.id)
            .one(&fixture.admin)
            .await?
    );
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn personal_history_rejects_household_credentials_with_attributable_denials() -> TestResult {
    let (fixture, boot) = boot_request_fixture().await;
    let session = fixture.credential("history-session");
    fixture.session(&session).await;
    let app_token = fixture.credential("history-app");
    fixture.app_token(&app_token).await;
    let request = server(&boot);
    for (token, method) in [(session, "api_session"), (app_token, "api_app_token")] {
        let response = request
            .get(ENDPOINT)
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(response.status_code(), 403, "{}", response.text());
        assert_eq!(response.header("cache-control"), "no-store");
        let audit = audits(&fixture, response.header("x-request-id").to_str()?).await;
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].event, "personal_history_denied");
        assert_eq!(
            audit[0].audit_context["actor_account_id"],
            fixture.account.id
        );
        assert_eq!(audit[0].audit_context["authentication_method"], method);
        assert!(audit[0].household_id.is_none());
        assert!(!serde_json::to_string(&audit[0])?.contains(&token));
    }
    let token = fixture.token().await;
    oauth_grants::Entity::update_many()
        .col_expr(
            oauth_grants::Column::RevokedAt,
            sea_orm::sea_query::Expr::val(chrono::Utc::now().naive_utc()),
        )
        .filter(oauth_grants::Column::Id.eq(fixture.authorization.id))
        .exec(&fixture.admin)
        .await?;
    let revoked = request
        .get(ENDPOINT)
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(revoked.status_code(), 401);
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn personal_history_browser_rejected_subject_is_audited() -> TestResult {
    let (fixture, boot) = boot_request_fixture().await;
    let mut request = server(&boot);
    request.save_cookies();
    install_browser_session(&mut request, &boot, fixture.account.id).await;
    let response = request
        .get(&format!(
            "/profile/history?person_id={}",
            fixture.foreign.account.person_id
        ))
        .await;
    assert_eq!(response.status_code(), 400);
    assert_eq!(response.header("cache-control"), "no-store");
    let audit = audits(&fixture, response.header("x-request-id").to_str()?).await;
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].event, "personal_history_denied");
    assert_eq!(
        audit[0].audit_context["actor_account_id"],
        fixture.account.id
    );
    assert_eq!(
        audit[0].audit_context["authentication_method"],
        "browser_session"
    );
    assert!(audit[0].household_id.is_none());
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn personal_history_validates_cursors_and_keeps_oauth_challenges() -> TestResult {
    let (fixture, boot) = boot_request_fixture().await;
    let token = fixture.token().await;
    let request = server(&boot);
    let empty = request
        .get(ENDPOINT)
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(empty.status_code(), 200, "{}", empty.text());
    assert_eq!(
        empty.json::<Value>(),
        json!({"data":[],"meta":{"next_cursor":null}})
    );
    for query in [
        "before_id=1",
        "before_taken_at=invalid",
        "before_id=-1&before_taken_at=2026-01-01T00:00:00Z",
        "person_id=1",
        "household_id=1",
        "limit=500",
    ] {
        let response = request
            .get(&format!("{ENDPOINT}?{query}"))
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(response.status_code(), 422, "{query}: {}", response.text());
        assert_eq!(response.header("cache-control"), "no-store");
        let audit = audits(&fixture, response.header("x-request-id").to_str()?).await;
        assert_eq!(audit.len(), 1);
        assert_eq!(
            audit[0].audit_context["actor_account_id"],
            fixture.account.id
        );
        assert_eq!(audit[0].event, "personal_history_denied");
    }
    let grant = oauth_grants::Entity::find_by_id(fixture.authorization.id)
        .one(&fixture.admin)
        .await?
        .ok_or("missing grant")?;
    let mut active = grant.clone().into_active_model();
    active.access_token_scopes = Set(Some("patient/*.read".into()));
    active.access_token_scope_hash = Set(grant.token_hash);
    active.update(&fixture.admin).await?;
    let scoped = request
        .get(ENDPOINT)
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(scoped.status_code(), 403);
    assert!(
        scoped
            .header("www-authenticate")
            .to_str()?
            .contains("insufficient_scope")
    );
    assert_eq!(
        audits(&fixture, scoped.header("x-request-id").to_str()?)
            .await
            .len(),
        1
    );
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn personal_history_rejects_native_keys_integrations_and_unknown_credentials() -> TestResult {
    let (fixture, boot) = boot_request_fixture().await;
    let token = fixture.token().await;
    let service = boot
        .app_context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .ok_or("missing identity service")?;
    let key = service
        .api_keys()?
        .create(
            &fixture.account.id.to_string(),
            better_auth::server_api::CreateKeyOptions {
                config_id: Some("medtracker-personal".into()),
                name: Some("History rejection key".into()),
                expires_in: Some(86_400.0),
                metadata: Some(json!({"households":[fixture.household.id]})),
                permissions: Some(std::collections::HashMap::from([(
                    "medtracker".into(),
                    vec!["care:read".into()],
                )])),
                ..Default::default()
            },
        )
        .await?;
    let request = server(&boot);
    let denied_key = request
        .get(ENDPOINT)
        .add_header("authorization", format!("Bearer {}", key.key))
        .add_header("x-medtracker-key-permission", "care:read")
        .await;
    assert!(matches!(denied_key.status_code().as_u16(), 401 | 403));
    let mut grant = fixture.authorization.clone().into_active_model();
    let mut application = fixture.application.clone().into_active_model();
    application.id = sea_orm::ActiveValue::NotSet;
    application.client_id = Set(format!("history-integration-{}", uuid::Uuid::new_v4()));
    application.client_kind = Set("integration".into());
    application.scopes = Set("patient/*.read".into());
    let application = application.insert(&fixture.admin).await?;
    grant.oauth_application_id = Set(application.id);
    grant.client_kind = Set("integration".into());
    grant.scopes = Set("patient/*.read".into());
    grant.access_token_scopes = Set(Some("patient/*.read".into()));
    grant.household_membership_id = Set(Some(fixture.membership.id));
    grant.person_id = Set(Some(fixture.account.person_id));
    grant.permissions_version = Set(Some(fixture.membership.permissions_version));
    grant.update(&fixture.admin).await?;
    for credential in [Some(token.as_str()), Some("unknown-history-token"), None] {
        let mut get = request.get(ENDPOINT);
        if let Some(credential) = credential {
            get = get.add_header("authorization", format!("Bearer {credential}"));
        }
        let response = get.await;
        assert_eq!(response.status_code(), 401, "{}", response.text());
        assert_eq!(response.header("cache-control"), "no-store");
        let audit = audits(&fixture, response.header("x-request-id").to_str()?).await;
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].event, "personal_history_denied");
        assert!(audit[0].audit_context["actor_account_id"].is_null());
        assert!(audit[0].household_id.is_none());
    }
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

async fn backend_pid(
    transaction: &sea_orm::DatabaseTransaction,
) -> Result<i32, Box<dyn std::error::Error>> {
    let query = sea_orm::sea_query::Query::select()
        .expr_as(
            sea_orm::sea_query::Func::cust("pg_catalog.pg_backend_pid"),
            "pid",
        )
        .to_owned();
    Ok(transaction
        .query_one_raw(transaction.get_database_backend().build(&query))
        .await?
        .ok_or("missing backend")?
        .try_get("", "pid")?)
}

async fn blocked_by(
    fixture: &RequestFixture,
    blocker: i32,
) -> Result<i32, Box<dyn std::error::Error>> {
    use sea_orm::sea_query::{Expr, ExprTrait, Func, Query};
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let query = Query::select()
                .column("pid")
                .from(("pg_catalog", "pg_stat_activity"))
                .and_where(
                    Expr::val(blocker).eq(Func::cust("ANY")
                        .arg(Func::cust("pg_catalog.pg_blocking_pids").arg(Expr::col("pid")))),
                )
                .to_owned();
            if let Some(row) = fixture
                .admin
                .query_one_raw(fixture.admin.get_database_backend().build(&query))
                .await?
            {
                return Ok::<_, Box<dyn std::error::Error>>(row.try_get::<i32>("", "pid")?);
            }
            tokio::task::yield_now().await;
        }
    })
    .await?
}

#[tokio::test]
async fn personal_history_rechecks_revocation_after_waiting_for_the_account() -> TestResult {
    use med_tracker::models::_entities::accounts;
    use sea_orm::sea_query::Expr;
    for disable_account in [false, true] {
        let (fixture, boot) = boot_request_fixture().await;
        let token = fixture.token().await;
        let blocker = fixture.admin.begin().await?;
        accounts::Entity::find_by_id(fixture.account.id)
            .lock_exclusive()
            .one(&blocker)
            .await?
            .ok_or("missing account")?;
        let pid = backend_pid(&blocker).await?;
        let request = server(&boot);
        let read = async {
            request
                .get(ENDPOINT)
                .add_header("authorization", format!("Bearer {token}"))
                .await
        };
        let revoke = async {
            blocked_by(&fixture, pid).await?;
            if disable_account {
                accounts::Entity::update_many()
                    .col_expr(accounts::Column::Active, Expr::val(false))
                    .filter(accounts::Column::Id.eq(fixture.account.id))
                    .exec(&blocker)
                    .await?;
            } else {
                oauth_grants::Entity::update_many()
                    .col_expr(
                        oauth_grants::Column::RevokedAt,
                        Expr::val(chrono::Utc::now().naive_utc()),
                    )
                    .filter(oauth_grants::Column::Id.eq(fixture.authorization.id))
                    .exec(&blocker)
                    .await?;
            }
            blocker.commit().await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        };
        let (response, revoked) = tokio::join!(read, revoke);
        revoked?;
        assert_eq!(response.status_code(), 401, "{}", response.text());
        let audit = audits(&fixture, response.header("x-request-id").to_str()?).await;
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].event, "personal_history_denied");
        assert_eq!(
            audit[0].audit_context["actor_account_id"],
            fixture.account.id
        );
        assert!(audit[0].household_id.is_none());
        drop(request);
        close_request(fixture, boot).await;
    }
    Ok(())
}

#[tokio::test]
async fn personal_history_read_audit_commits_before_waiting_revocation() -> TestResult {
    use med_tracker::models::_entities::audit_chain_heads;
    use sea_orm::sea_query::Expr;
    let (fixture, boot) = boot_request_fixture().await;
    let token = fixture.token().await;
    let blocker = fixture.admin.begin().await?;
    audit_chain_heads::Entity::find()
        .filter(audit_chain_heads::Column::ChainKey.eq("global"))
        .lock_exclusive()
        .one(&blocker)
        .await?
        .ok_or("missing audit chain")?;
    let pid = backend_pid(&blocker).await?;
    let request = server(&boot);
    let read = async {
        request
            .get(ENDPOINT)
            .add_header("authorization", format!("Bearer {token}"))
            .await
    };
    let ordering = async {
        let reader = blocked_by(&fixture, pid).await?;
        let revoke = async {
            oauth_grants::Entity::update_many()
                .col_expr(
                    oauth_grants::Column::RevokedAt,
                    Expr::val(chrono::Utc::now().naive_utc()),
                )
                .filter(oauth_grants::Column::Id.eq(fixture.authorization.id))
                .exec(&fixture.admin)
                .await
        };
        let release = async {
            blocked_by(&fixture, reader).await?;
            blocker.commit().await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        };
        let (revoked, released) = tokio::join!(revoke, release);
        released?;
        revoked?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (response, ordered) = tokio::join!(read, ordering);
    ordered?;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    let audit = audits(&fixture, response.header("x-request-id").to_str()?).await;
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].event, "personal_history_read");
    let after = request
        .get(ENDPOINT)
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(after.status_code(), 401);
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn personal_history_audit_failure_releases_no_records() -> TestResult {
    let (fixture, boot) = boot_request_fixture().await;
    let token = fixture.token().await;
    let fault = format!("history_audit_fault_{}", uuid::Uuid::new_v4().simple());
    versions::ActiveModel {
        item_type: Set("SyntheticHistoryAuditFailure".into()),
        item_id: Set(fixture.account.id),
        event: Set("fixture".into()),
        created_at: Set(Some(chrono::Utc::now().naive_utc())),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    // SeaQuery schema builders cannot create PostgreSQL trigger functions or triggers.
    // The parameterised marker limits this DDL exception to this scenario's account.
    fixture.admin.execute_unprepared(&format!("CREATE FUNCTION {fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='AccountClinicalAccess' AND NEW.event IN ('personal_history_read','personal_history_denied') AND EXISTS (SELECT 1 FROM public.versions WHERE item_type='SyntheticHistoryAuditFailure' AND item_id=NEW.item_id) THEN RAISE EXCEPTION 'Synthetic history audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER {fault} BEFORE INSERT ON public.versions FOR EACH ROW EXECUTE FUNCTION {fault}()")).await?;
    let request = server(&boot);
    let read = request
        .get(ENDPOINT)
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    let rejected = request
        .get(&format!("{ENDPOINT}?person_id=1"))
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {fault} ON public.versions; DROP FUNCTION {fault}()"
        ))
        .await?;
    for response in [read, rejected] {
        assert_eq!(response.status_code(), 500, "{}", response.text());
        let body: Value = response.json();
        assert!(body.get("data").is_none());
        assert!(
            audits(&fixture, response.header("x-request-id").to_str()?)
                .await
                .is_empty()
        );
    }
    let retry = request
        .get(ENDPOINT)
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(retry.status_code(), 200);
    assert_eq!(
        audits(&fixture, retry.header("x-request-id").to_str()?)
            .await
            .len(),
        1
    );
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}
