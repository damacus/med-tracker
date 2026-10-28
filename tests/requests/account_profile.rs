use super::*;
use med_tracker::models::_entities::{
    accounts, api_app_tokens, household_memberships, oauth_grants, versions,
};
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

async fn stored_session(
    fixture: &RequestFixture,
    token: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    Ok(fixture.session(&fixture.credential(token)).await.id)
}

async fn query_waits_for_lock(
    fixture: &RequestFixture,
    pattern: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let observed = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let row = fixture.admin.query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query ILIKE $1 AND cardinality(pg_blocking_pids(pid))>0) AS waiting",
                [pattern.into()],
            )).await?.ok_or("lock observation must return one row")?;
            if row.try_get::<bool>("", "waiting")? {
                return Ok::<_,Box<dyn std::error::Error>>(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await;
    match observed {
        Ok(result) => result.map(|()| true),
        Err(_) => Ok(false),
    }
}

async fn hold_profile_audit(
    fixture: &RequestFixture,
) -> Result<sea_orm::DatabaseTransaction, Box<dyn std::error::Error>> {
    fixture.admin.execute_unprepared("CREATE FUNCTION block_profile_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='Account' AND NEW.event='update' THEN PERFORM pg_advisory_xact_lock(714208); END IF; RETURN NEW; END $$; CREATE TRIGGER block_profile_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION block_profile_audit()").await?;
    let blocker = fixture.admin.begin().await?;
    blocker
        .execute_unprepared("SELECT pg_advisory_xact_lock(714208)")
        .await?;
    Ok(blocker)
}

async fn profile_state(fixture: &RequestFixture) -> Result<(accounts::Model, u64), sea_orm::DbErr> {
    let account = accounts::Entity::find_by_id(fixture.account.id)
        .one(&fixture.admin)
        .await?
        .unwrap();
    let audits = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("Account"))
        .filter(versions::Column::ItemId.eq(fixture.account.id))
        .filter(versions::Column::Event.eq("update"))
        .count(&fixture.admin)
        .await?;
    Ok((account, audits))
}

async fn revoke_authority(fixture: &RequestFixture, app: bool) -> Result<(), sea_orm::DbErr> {
    let now = chrono::Utc::now().naive_utc();
    if app {
        api_app_tokens::Entity::update_many()
            .col_expr(api_app_tokens::Column::RevokedAt, Expr::value(now))
            .col_expr(api_app_tokens::Column::UpdatedAt, Expr::value(now))
            .filter(api_app_tokens::Column::AccountId.eq(fixture.account.id))
            .exec(&fixture.admin)
            .await?;
    } else {
        household_memberships::Entity::update_many()
            .col_expr(
                household_memberships::Column::Status,
                Expr::value("revoked"),
            )
            .col_expr(household_memberships::Column::RevokedAt, Expr::value(now))
            .col_expr(household_memberships::Column::UpdatedAt, Expr::value(now))
            .col_expr(household_memberships::Column::PermissionsVersion, {
                use sea_orm::sea_query::ExprTrait;
                Expr::col(household_memberships::Column::PermissionsVersion).add(1)
            })
            .filter(household_memberships::Column::Id.eq(fixture.membership.id))
            .exec(&fixture.admin)
            .await?;
    }
    Ok(())
}

#[tokio::test]
async fn stored_session_revocation_cannot_overtake_a_profile_write() -> TestResult {
    let (fixture, boot) = boot_request_fixture().await;
    household_memberships::Entity::update_many()
        .col_expr(household_memberships::Column::Role, Expr::value("member"))
        .filter(household_memberships::Column::Id.eq(fixture.membership.id))
        .exec(&fixture.admin)
        .await?;
    let session_id = stored_session(&fixture, "profile-inflight").await?;
    stored_session(&fixture, "profile-revoker").await?;
    let blocker = hold_profile_audit(&fixture).await?;
    let request = server(&boot);
    let revoker = server(&boot);
    let write = async {
        request
            .patch("/api/v1/auth/profile")
            .add_header(
                "authorization",
                format!("Bearer {}", fixture.credential("profile-inflight")),
            )
            .json(&json!({"profile":{"name":"Authorised before revocation"}}))
            .await
    };
    let revoke = async {
        let reached_audit = query_waits_for_lock(&fixture, "%INSERT%versions%").await?;
        let revocation = async {
            revoker
                .delete(&format!("/api/v1/auth/sessions/{session_id}"))
                .add_header(
                    "authorization",
                    format!("Bearer {}", fixture.credential("profile-revoker")),
                )
                .await
        };
        tokio::pin!(revocation);
        let (serialised, completed) = tokio::select! {
            response = &mut revocation => (false, Some(response)),
            observed = query_waits_for_lock(&fixture, "%UPDATE%api_sessions%") => (observed?, None),
        };
        blocker.commit().await?;
        let revoked = match completed {
            Some(response) => response,
            None => revocation.await,
        };
        Ok::<_, Box<dyn std::error::Error>>((reached_audit, serialised, revoked))
    };
    let (updated, ordering) = tokio::join!(write, revoke);
    let (reached_audit, serialised, revoked) = ordering?;
    let denied = request
        .patch("/api/v1/auth/profile")
        .add_header(
            "authorization",
            format!("Bearer {}", fixture.credential("profile-inflight")),
        )
        .json(&json!({"profile":{"name":"Must never persist"}}))
        .await;
    let (account, audits) = profile_state(&fixture).await?;
    fixture
        .admin
        .execute_unprepared(
            "DROP TRIGGER block_profile_audit ON versions; DROP FUNCTION block_profile_audit()",
        )
        .await?;
    drop(request);
    drop(revoker);
    close_request(fixture, boot).await;
    assert!(
        reached_audit,
        "profile write must reach the held audit boundary"
    );
    assert!(
        serialised,
        "revocation must wait for the authorised profile transaction"
    );
    assert_eq!(updated.status_code(), 200);
    assert_eq!(revoked.status_code(), 204);
    assert_eq!(denied.status_code(), 401);
    assert_eq!(
        account.name.as_deref().unwrap(),
        "Authorised before revocation"
    );
    assert_eq!(audits, 1);
    Ok(())
}

#[tokio::test]
async fn stored_credential_authority_revocation_waits_for_profile_commit() -> TestResult {
    for table in ["api_app_tokens", "household_memberships"] {
        let (fixture, boot) = boot_request_fixture().await;
        let token = fixture.credential("profile-authority-race");
        if table == "api_app_tokens" {
            fixture.app_token(&token).await;
        } else {
            fixture.session(&token).await;
        }
        let blocker = hold_profile_audit(&fixture).await?;
        let request = server(&boot);
        let write = async {
            request
                .patch("/api/v1/auth/profile")
                .add_header("authorization", format!("Bearer {token}"))
                .json(&json!({"profile":{"name":"Before authority revocation"}}))
                .await
        };
        let revoke = async {
            let reached_audit = query_waits_for_lock(&fixture, "%INSERT%versions%").await?;
            let revocation = revoke_authority(&fixture, table == "api_app_tokens");
            tokio::pin!(revocation);
            let pattern = format!("%UPDATE%{table}%");
            let (serialised, completed) = tokio::select! {
                result = &mut revocation => (false, Some(result)),
                observed = query_waits_for_lock(&fixture, &pattern) => (observed?, None),
            };
            blocker.commit().await?;
            match completed {
                Some(result) => {
                    result?;
                }
                None => {
                    revocation.await?;
                }
            }
            Ok::<_, Box<dyn std::error::Error>>((reached_audit, serialised))
        };
        let (updated, ordering) = tokio::join!(write, revoke);
        let (reached_audit, serialised) = ordering?;
        let denied = request
            .patch("/api/v1/auth/profile")
            .add_header("authorization", format!("Bearer {token}"))
            .json(&json!({"profile":{"name":"Must never persist"}}))
            .await;
        let (account, audits) = profile_state(&fixture).await?;
        fixture
            .admin
            .execute_unprepared(
                "DROP TRIGGER block_profile_audit ON versions; DROP FUNCTION block_profile_audit()",
            )
            .await?;
        drop(request);
        close_request(fixture, boot).await;
        assert!(
            reached_audit,
            "{table}: profile must reach the held audit boundary"
        );
        assert!(
            serialised,
            "{table}: revocation must wait for profile commit"
        );
        assert_eq!(updated.status_code(), 200, "{table}");
        assert_eq!(denied.status_code(), 401, "{table}");
        assert_eq!(
            account.name.as_deref().unwrap(),
            "Before authority revocation"
        );
        assert_eq!(audits, 1);
    }
    Ok(())
}

#[tokio::test]
async fn stored_session_revoked_while_waiting_for_account_lock_cannot_update_profile() -> TestResult
{
    let (fixture, boot) = boot_request_fixture().await;
    let session_id = stored_session(&fixture, "profile-waiting").await?;
    stored_session(&fixture, "profile-revoker").await?;
    let blocker = fixture.admin.begin().await?;
    accounts::Entity::find_by_id(fixture.account.id)
        .lock(sea_orm::sea_query::LockType::NoKeyUpdate)
        .one(&blocker)
        .await?;
    let request = server(&boot);
    let revoker = server(&boot);
    let write = async {
        request
            .patch("/api/v1/auth/profile")
            .add_header(
                "authorization",
                format!("Bearer {}", fixture.credential("profile-waiting")),
            )
            .json(&json!({"profile":{"name":"Must never persist"}}))
            .await
    };
    let revoke = async {
        let reached_account =
            query_waits_for_lock(&fixture, "%accounts%FOR NO KEY UPDATE%").await?;
        let revoked = revoker
            .delete(&format!("/api/v1/auth/sessions/{session_id}"))
            .add_header(
                "authorization",
                format!("Bearer {}", fixture.credential("profile-revoker")),
            )
            .await;
        blocker.commit().await?;
        Ok::<_, Box<dyn std::error::Error>>((reached_account, revoked))
    };
    let (denied, ordering) = tokio::join!(write, revoke);
    let (reached_account, revoked) = ordering?;
    let (account, audits) = profile_state(&fixture).await?;
    drop(request);
    drop(revoker);
    close_request(fixture, boot).await;
    assert!(
        reached_account,
        "profile authentication must precede the held account lock"
    );
    assert_eq!(revoked.status_code(), 204);
    assert_eq!(denied.status_code(), 401);
    assert_eq!(account.name, None);
    assert_eq!(audits, 0);
    Ok(())
}

fn assert_profile_schema(method: &str, status: u16, body: &Value) {
    let document = contract::contract();
    let raw = &document["paths"]["/auth/profile"][method]["responses"][status.to_string()];
    let response = if raw.get("$ref").is_some() {
        contract::resolve(document, raw)
    } else {
        raw
    };
    contract::assert_value(
        document,
        &response["content"]["application/json"]["schema"],
        body,
        &format!("{method} /auth/profile {status}"),
    );
}

#[tokio::test]
async fn account_profile_name_is_account_owned_and_audited() -> TestResult {
    let (fixture, boot) = boot_request_fixture().await;
    fixture
        .session(&fixture.credential("profile-session"))
        .await;
    let account_id = fixture.account.id.to_string();
    let email = fixture.account.email.clone();
    let request = server(&boot);
    let anonymous = request.get("/api/v1/auth/profile").await;
    let before = request
        .get("/api/v1/auth/profile")
        .add_header(
            "authorization",
            format!("Bearer {}", fixture.credential("profile-session")),
        )
        .await;
    let update = request
        .patch("/api/v1/auth/profile")
        .add_header(
            "authorization",
            format!("Bearer {}", fixture.credential("profile-session")),
        )
        .add_header("x-request-id", "client-profile-request")
        .json(&json!({"profile":{"name":"  Account name  "}}))
        .await;
    let after = request
        .get("/api/v1/auth/profile")
        .add_header(
            "authorization",
            format!("Bearer {}", fixture.credential("profile-session")),
        )
        .await;
    let audit = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("Account"))
        .filter(versions::Column::ItemId.eq(fixture.account.id))
        .filter(versions::Column::Event.eq("update"))
        .order_by_desc(versions::Column::Id)
        .one(&fixture.admin)
        .await?;
    drop(request);
    close_request(fixture, boot).await;

    assert_eq!(anonymous.status_code(), 401);
    assert_eq!(anonymous.header("cache-control"), "no-store");
    assert_eq!(before.status_code(), 200);
    assert_eq!(before.header("cache-control"), "no-store");
    let before: Value = before.json();
    assert_profile_schema("get", 200, &before);
    assert_eq!(before["data"]["account_id"], account_id);
    assert_eq!(before["data"]["name"], Value::Null);
    assert_eq!(update.status_code(), 200);
    assert_eq!(update.header("cache-control"), "no-store");
    assert_eq!(after.status_code(), 200);
    let after: Value = after.json();
    assert_profile_schema("get", 200, &after);
    assert_profile_schema("patch", 200, &update.json());
    assert_eq!(after["data"]["name"], "Account name");
    assert_eq!(after["data"]["email"], email);
    let audit = audit.ok_or("profile update must record an account audit")?;
    assert_eq!(
        audit.audit_context["actor_account_id"]
            .to_string()
            .trim_matches('"'),
        account_id
    );
    let trace = audit.audit_context["server_trace_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(uuid::Uuid::parse_str(&trace).is_ok());
    assert_ne!(trace, "client-profile-request");
    assert_eq!(audit.household_id, None);
    assert_eq!(
        audit.request_id.as_deref().unwrap(),
        update.header("x-request-id").to_str()?
    );
    Ok(())
}

#[tokio::test]
async fn mobile_account_profile_remains_available_after_household_revocation() -> TestResult {
    let (fixture, boot) = boot_request_fixture().await;
    let token = fixture.token().await;
    revoke_authority(&fixture, false).await?;
    let request = server(&boot);
    let profile = request
        .get("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    let updated = request
        .patch("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .json(&json!({"profile":{"name":"Still my account"}}))
        .await;
    let clinical = request
        .get(&format!(
            "/api/v1/households/{}/people",
            fixture.household.id
        ))
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    drop(request);
    close_request(fixture, boot).await;
    assert_eq!(profile.status_code(), 200);
    assert_eq!(updated.status_code(), 200);
    assert_eq!(updated.header("cache-control"), "no-store");
    assert_eq!(clinical.status_code(), 403);
    Ok(())
}

#[tokio::test]
async fn account_profile_rejects_invalid_protected_and_revoked_writes() -> TestResult {
    let (fixture, boot) = boot_request_fixture().await;
    let token = fixture.token().await;
    let foreign = Scenario::new().await;
    accounts::Entity::update_many()
        .col_expr(accounts::Column::Name, Expr::value("Foreign account"))
        .filter(accounts::Column::Id.eq(foreign.account.id))
        .exec(&fixture.admin)
        .await?;
    let request = server(&boot);
    for name in [String::new(), "   ".to_owned(), "a".repeat(101)] {
        let response = request
            .patch("/api/v1/auth/profile")
            .add_header("authorization", format!("Bearer {token}"))
            .json(&json!({"profile":{"name":name}}))
            .await;
        assert_eq!(response.status_code(), 422);
        assert_eq!(response.header("cache-control"), "no-store");
    }
    for profile in [
        json!({"name":"Changed", "email":"changed@example.test"}),
        json!({"name":"Changed", "account_id":foreign.account.id.to_string()}),
    ] {
        let response = request
            .patch("/api/v1/auth/profile")
            .add_header("authorization", format!("Bearer {token}"))
            .json(&json!({"profile":profile}))
            .await;
        assert_eq!(response.status_code(), 400);
        assert_eq!(response.header("cache-control"), "no-store");
    }
    let anonymous = request
        .patch("/api/v1/auth/profile")
        .json(&json!({"profile":{"name":false}}))
        .await;
    assert_eq!(anonymous.status_code(), 401);
    assert_eq!(anonymous.header("cache-control"), "no-store");
    oauth_grants::Entity::update_many()
        .col_expr(
            oauth_grants::Column::RevokedAt,
            Expr::value(chrono::Utc::now().naive_utc()),
        )
        .filter(oauth_grants::Column::AccountId.eq(fixture.account.id))
        .exec(&fixture.admin)
        .await?;
    let revoked = request
        .patch("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .json(&json!({"profile":{"name":"Revoked write"}}))
        .await;
    assert_eq!(revoked.status_code(), 401);
    assert_eq!(revoked.header("cache-control"), "no-store");
    let (own, audits) = profile_state(&fixture).await?;
    let foreign_account = accounts::Entity::find_by_id(foreign.account.id)
        .one(&fixture.admin)
        .await?
        .unwrap();
    let email = fixture.account.email.clone();
    foreign.close().await;
    drop(request);
    close_request(fixture, boot).await;
    assert_eq!(own.name, None);
    assert_eq!(own.email, email);
    assert_eq!(foreign_account.name.as_deref().unwrap(), "Foreign account");
    assert_eq!(audits, 0);
    Ok(())
}

#[tokio::test]
async fn account_profile_audit_failure_rolls_back_the_name_change() -> TestResult {
    let (fixture, boot) = boot_request_fixture().await;
    let token = fixture.token().await;
    fixture.admin.execute_unprepared("CREATE FUNCTION reject_profile_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='Account' AND NEW.event='update' THEN RAISE EXCEPTION 'Synthetic profile audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_profile_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_profile_audit()").await?;
    let request = server(&boot);
    let failed = request
        .patch("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .json(&json!({"profile":{"name":"Uncommitted name","time_zone":"Europe/Paris","gravatar_enabled":true}}))
        .await;
    let refreshed = request
        .get("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    let (_, audits) = profile_state(&fixture).await?;
    fixture
        .admin
        .execute_unprepared(
            "DROP TRIGGER reject_profile_audit ON versions; DROP FUNCTION reject_profile_audit()",
        )
        .await?;
    drop(request);
    close_request(fixture, boot).await;
    assert_eq!(failed.status_code(), 500);
    assert_eq!(failed.header("cache-control"), "no-store");
    let failed_body: Value = failed.json();
    assert_profile_schema("patch", 500, &failed_body);
    assert_eq!(failed_body["error"]["message"], "Internal server error");
    assert_eq!(refreshed.status_code(), 200);
    let refreshed: Value = refreshed.json();
    assert_eq!(refreshed["data"]["name"], Value::Null);
    assert_eq!(refreshed["data"]["time_zone"], "Etc/UTC");
    assert_eq!(refreshed["data"]["gravatar_enabled"], false);
    assert_eq!(audits, 0);
    Ok(())
}
