use super::*;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

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
    let (fixture, boot) = boot_fixture().await;
    let digest = hex::encode(Sha256::digest(b"profile-session"));
    fixture.admin.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO api_sessions(account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(71001,74001,$1,'profile-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),now(),now())",
        [digest.into()],
    )).await?;
    let request = server(&boot);
    let anonymous = request.get("/api/v1/auth/profile").await;
    let before = request
        .get("/api/v1/auth/profile")
        .add_header("authorization", "Bearer profile-session")
        .await;
    let update = request
        .patch("/api/v1/auth/profile")
        .add_header("authorization", "Bearer profile-session")
        .add_header("x-request-id", "client-profile-request")
        .json(&json!({"profile":{"name":"  Account name  "}}))
        .await;
    let after = request
        .get("/api/v1/auth/profile")
        .add_header("authorization", "Bearer profile-session")
        .await;
    let audit = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT audit_context->>'actor_account_id' AS actor,audit_context->>'server_trace_id' AS trace,household_id,request_id FROM versions WHERE item_type='Account' AND item_id=71001 AND event='update' ORDER BY id DESC LIMIT 1"
    )).await?;
    drop(request);
    close(fixture, boot).await;

    assert_eq!(anonymous.status_code(), 401);
    assert_eq!(anonymous.header("cache-control"), "no-store");
    assert_eq!(before.status_code(), 200);
    assert_eq!(before.header("cache-control"), "no-store");
    let before: Value = before.json();
    assert_profile_schema("get", 200, &before);
    assert_eq!(before["data"]["account_id"], "71001");
    assert_eq!(before["data"]["name"], Value::Null);
    assert_eq!(update.status_code(), 200);
    assert_eq!(update.header("cache-control"), "no-store");
    assert_eq!(after.status_code(), 200);
    let after: Value = after.json();
    assert_profile_schema("get", 200, &after);
    assert_profile_schema("patch", 200, &update.json());
    assert_eq!(after["data"]["name"], "Account name");
    assert_eq!(after["data"]["email"], "persistence@example.test");
    let audit = audit.ok_or("profile update must record an account audit")?;
    assert_eq!(audit.try_get::<String>("", "actor")?, "71001");
    let trace = audit.try_get::<String>("", "trace")?;
    assert!(uuid::Uuid::parse_str(&trace).is_ok());
    assert_ne!(trace, "client-profile-request");
    assert_eq!(audit.try_get::<Option<i64>>("", "household_id")?, None);
    assert_eq!(
        audit.try_get::<String>("", "request_id")?,
        update.header("x-request-id").to_str()?
    );
    Ok(())
}

#[tokio::test]
async fn mobile_account_profile_remains_available_after_household_revocation() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let token = super::account::mobile_token(&fixture).await;
    fixture.admin.execute_unprepared(
        "UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE account_id=71001"
    ).await?;
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
        .get("/api/v1/households/72001/people")
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    drop(request);
    close(fixture, boot).await;
    assert_eq!(profile.status_code(), 200);
    assert_eq!(updated.status_code(), 200);
    assert_eq!(updated.header("cache-control"), "no-store");
    assert_eq!(clinical.status_code(), 403);
    Ok(())
}

#[tokio::test]
async fn account_profile_rejects_invalid_protected_and_revoked_writes() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let token = super::account::mobile_token(&fixture).await;
    fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,name,created_at,updated_at) VALUES(71002,'foreign-profile@example.test','Foreign account',now(),now())").await?;
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
        json!({"name":"Changed", "account_id":"71002"}),
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
    fixture
        .admin
        .execute_unprepared("UPDATE oauth_grants SET revoked_at=now() WHERE account_id=71001")
        .await?;
    let revoked = request
        .patch("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .json(&json!({"profile":{"name":"Revoked write"}}))
        .await;
    assert_eq!(revoked.status_code(), 401);
    assert_eq!(revoked.header("cache-control"), "no-store");
    let state = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT name FROM accounts WHERE id=71001) AS own_name,(SELECT email FROM accounts WHERE id=71001) AS own_email,(SELECT name FROM accounts WHERE id=71002) AS foreign_name,(SELECT count(*) FROM versions WHERE item_type='Account' AND event='update') AS audits"
    )).await?.ok_or("profile accounts must remain queryable")?;
    drop(request);
    close(fixture, boot).await;
    assert_eq!(state.try_get::<Option<String>>("", "own_name")?, None);
    assert_eq!(
        state.try_get::<String>("", "own_email")?,
        "persistence@example.test"
    );
    assert_eq!(
        state.try_get::<String>("", "foreign_name")?,
        "Foreign account"
    );
    assert_eq!(state.try_get::<i64>("", "audits")?, 0);
    Ok(())
}

#[tokio::test]
async fn account_profile_audit_failure_rolls_back_the_name_change() -> TestResult {
    let (fixture, boot) = boot_fixture().await;
    let token = super::account::mobile_token(&fixture).await;
    fixture.admin.execute_unprepared("CREATE FUNCTION reject_profile_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='Account' AND NEW.event='update' THEN RAISE EXCEPTION 'Synthetic profile audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_profile_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_profile_audit()").await?;
    let request = server(&boot);
    let failed = request
        .patch("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .json(&json!({"profile":{"name":"Uncommitted name"}}))
        .await;
    let refreshed = request
        .get("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    let audits = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS total FROM versions WHERE item_type='Account' AND event='update'",
        ))
        .await?
        .ok_or("audit count must return one row")?
        .try_get::<i64>("", "total")?;
    drop(request);
    close(fixture, boot).await;
    assert_eq!(failed.status_code(), 500);
    assert_eq!(failed.header("cache-control"), "no-store");
    let failed_body: Value = failed.json();
    assert_profile_schema("patch", 500, &failed_body);
    assert_eq!(failed_body["error"]["message"], "Internal server error");
    assert_eq!(refreshed.status_code(), 200);
    let refreshed: Value = refreshed.json();
    assert_eq!(refreshed["data"]["name"], Value::Null);
    assert_eq!(audits, 0);
    Ok(())
}
