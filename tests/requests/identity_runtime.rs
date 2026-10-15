use super::*;

#[tokio::test]
async fn household_notification_form_offers_explicit_child_opt_in() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=EXCLUDED.recovery_saved_at").await.unwrap();
    let service = boot
        .app_context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let origin = url::Url::parse(&service.context().config.base_url)
        .unwrap()
        .origin()
        .ascii_serialization();
    let mut request = server(&boot);
    request.save_cookies();
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", origin.clone())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(signed_in.status_code(), 200);
    let cookie = signed_in
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(';').next())
        .collect::<Vec<_>>()
        .join("; ");
    let profile = request
        .get("/households/persistence-fixture/profile")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(profile.status_code(), 200);
    assert!(profile.text().contains("profile-notification-form"));
    assert!(
        profile.text().contains("name=\"managed_73002\""),
        "child receives an explicit subscription control"
    );
    assert!(!profile.text().contains("Included automatically"));
    let csrf = profile
        .text()
        .split("name=\"authenticity_token\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_owned();
    let denied = request
        .post("/households/persistence-fixture/profile/notifications")
        .add_header("cookie", cookie.clone())
        .add_header("sec-fetch-site", "same-origin")
        .add_header("origin", origin.clone())
        .form(&[
            ("setting", "notifications"),
            ("enabled", "true"),
            ("missed_dose_enabled", "true"),
            ("managed_73002", "true"),
            ("authenticity_token", "invalid"),
        ])
        .await;
    assert_eq!(denied.status_code(), 403);
    let saved = request
        .post("/households/persistence-fixture/profile/notifications")
        .add_header("cookie", cookie.clone())
        .add_header("sec-fetch-site", "same-origin")
        .add_header("origin", origin.clone())
        .form(&[
            ("setting", "notifications"),
            ("enabled", "true"),
            ("missed_dose_enabled", "true"),
            ("managed_73002", "true"),
            ("morning_time", "08:00"),
            ("afternoon_time", "14:00"),
            ("evening_time", "18:00"),
            ("night_time", "22:00"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(saved.status_code(), 303);
    let state=fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT enabled,(SELECT count(*) FROM security_audit_events WHERE event_type='notification_preferences.managed_people.updated' AND actor_account_id=71001) AS audits FROM household_notification_subscriptions WHERE account_id=71001 AND household_id=72001 AND person_id=73002")).await.unwrap().unwrap();
    assert!(state.try_get::<bool>("", "enabled").unwrap());
    assert_eq!(state.try_get::<i64>("", "audits").unwrap(), 1);
    let refreshed = request
        .get("/households/persistence-fixture/profile")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(refreshed.status_code(), 200);
    assert!(
        refreshed
            .text()
            .split("name=\"managed_73002\"")
            .nth(1)
            .unwrap()
            .split('>')
            .next()
            .unwrap()
            .contains("checked")
    );
    let removed = request
        .post("/households/persistence-fixture/profile/notifications")
        .add_header("cookie", cookie)
        .add_header("sec-fetch-site", "same-origin")
        .add_header("origin", origin)
        .form(&[
            ("setting", "notifications"),
            ("enabled", "true"),
            ("missed_dose_enabled", "true"),
            ("morning_time", "08:00"),
            ("afternoon_time", "14:00"),
            ("evening_time", "18:00"),
            ("night_time", "22:00"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(removed.status_code(), 303);
    let state=fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT enabled FROM household_notification_subscriptions WHERE account_id=71001 AND household_id=72001 AND person_id=73002")).await.unwrap().unwrap();
    assert!(!state.try_get::<bool>("", "enabled").unwrap());
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn same_person_has_two_current_households_and_revocation_is_live() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared(
        "INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(92902,'Second participation','runtime-second','UTC',now(),now());
         INSERT INTO household_memberships(id,person_id,household_id,role,status,joined_at,created_at,updated_at)
         SELECT 92903,person_id,92902,'member','active',now(),now(),now() FROM accounts WHERE id=71001"
    ).await.unwrap();
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO api_sessions(account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,device_name,created_at,updated_at) VALUES(71001,74001,$1,'identity-first-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),'First',now(),now()),(71001,92903,$2,'identity-second-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),'Second',now(),now())",
        [hex::encode(Sha256::digest(b"identity-first-session")).into(),hex::encode(Sha256::digest(b"identity-second-session")).into()]
    )).await.unwrap();
    let request = server(&boot);
    let first = request
        .get("/api/v1/households/72001/me")
        .add_header("authorization", "Bearer identity-first-session")
        .await;
    assert_eq!(first.status_code(), 200, "{}", first.text());
    let second = request
        .get("/api/v1/households/92902/me")
        .add_header("authorization", "Bearer identity-second-session")
        .await;
    assert_eq!(second.status_code(), 200, "{}", second.text());
    let first: Value = first.json();
    let second: Value = second.json();
    assert_eq!(first["data"]["person"]["id"], 73001);
    assert_eq!(second["data"]["person"]["id"], 73001);
    fixture.admin.execute_unprepared("UPDATE household_memberships SET revoked_at=now(),permissions_version=permissions_version+1 WHERE id=92903").await.unwrap();
    let denied = request
        .get("/api/v1/households/92902/me")
        .add_header("authorization", "Bearer identity-second-session")
        .await;
    assert_eq!(denied.status_code(), 401);
    let retained = request
        .get("/api/v1/households/72001/me")
        .add_header("authorization", "Bearer identity-first-session")
        .await;
    assert_eq!(retained.status_code(), 200);
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn manager_creates_and_renames_accountless_clinical_participant() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO api_sessions(account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(71001,74001,$1,'clinical-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),now(),now())",
        [hex::encode(Sha256::digest(b"clinical-manager-session")).into()])).await.unwrap();
    let diagnostic = med_tracker::models::access::begin(
        &fixture.runtime,
        &med_tracker::models::access::HouseholdScope {
            actor: med_tracker::models::access::Actor { account_id: 71001 },
            household_id: 72001,
            request_id: "clinical-diagnostic".into(),
        },
        None,
    )
    .await
    .unwrap();
    let lock = diagnostic
        .transaction()
        .execute_unprepared("SELECT id FROM households WHERE id=72001 FOR UPDATE")
        .await
        .expect_err("runtime must not acquire broad UPDATE privilege");
    assert!(
        lock.to_string()
            .contains("permission denied for table households"),
        "{lock}"
    );
    diagnostic.rollback().await.unwrap();
    let request = server(&boot);
    let created = request.post("/api/v1/households/72001/people")
        .add_header("authorization", "Bearer clinical-manager-session")
        .json(&json!({"person":{"name":"Child","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false}})).await;
    assert_eq!(created.status_code(), 201, "{}", created.text());
    let body: Value = created.json();
    let id = body["data"]["id"].as_i64().unwrap();
    let edited = request
        .patch(&format!("/api/v1/households/72001/people/{id}"))
        .add_header("authorization", "Bearer clinical-manager-session")
        .json(&json!({"person":{"name":"Renamed child"}}))
        .await;
    assert_eq!(edited.status_code(), 200, "{}", edited.text());
    let persisted = fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT clinical_name,(SELECT count(*) FROM accounts WHERE person_id=$1) AS accounts,(SELECT count(*) FROM versions WHERE item_type='Person' AND item_id=$1 AND event='update') AS edits FROM people WHERE id=$1",[id.into()])).await.unwrap().unwrap();
    assert_eq!(
        persisted.try_get::<String>("", "clinical_name").unwrap(),
        "Renamed child"
    );
    assert_eq!(persisted.try_get::<i64>("", "accounts").unwrap(), 0);
    assert_eq!(persisted.try_get::<i64>("", "edits").unwrap(), 1);
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn manager_lists_and_suspends_accountless_participation() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO people(id,clinical_name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(93101,'Clinical participant',DATE '2020-01-01',1,false,now(),now());INSERT INTO household_memberships(id,person_id,household_id,role,status,joined_at,created_at,updated_at) VALUES(93102,93101,72001,'member','active',now(),now(),now())").await.unwrap();
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO api_sessions(account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(71001,74001,$1,'membership-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),now(),now())",
        [hex::encode(Sha256::digest(b"membership-manager-session")).into()])).await.unwrap();
    let request = server(&boot);
    let listed = request
        .get("/api/v1/households/72001/admin/memberships")
        .add_header("authorization", "Bearer membership-manager-session")
        .await;
    assert_eq!(listed.status_code(), 200, "{}", listed.text());
    let body: Value = listed.json();
    let participant = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["person_id"] == 93101)
        .unwrap();
    assert_eq!(participant["account_id"], Value::Null);
    assert_eq!(participant["person_name"], "Clinical participant");
    let changed = request
        .patch("/api/v1/households/72001/admin/memberships/93102")
        .add_header("authorization", "Bearer membership-manager-session")
        .json(&json!({"household_membership":{"status":"suspended"}}))
        .await;
    assert_eq!(changed.status_code(), 200, "{}", changed.text());
    let body: Value = changed.json();
    assert_eq!(body["data"]["status"], "suspended");
    assert_eq!(body["data"]["permissions_version"], 2);
    let removed = request
        .delete("/api/v1/households/72001/admin/memberships/93102")
        .add_header("authorization", "Bearer membership-manager-session")
        .await;
    assert_eq!(removed.status_code(), 204, "{}", removed.text());
    let row = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,permissions_version FROM household_memberships WHERE id=93102",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "revoked");
    assert_eq!(row.try_get::<i32>("", "permissions_version").unwrap(), 3);
    let settings = request
        .patch("/api/v1/households/72001/admin/settings")
        .add_header("authorization", "Bearer membership-manager-session")
        .json(&json!({"household":{"name":"Updated household","timezone":"Europe/London"}}))
        .await;
    assert_eq!(settings.status_code(), 200, "{}", settings.text());
    let home = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT name,timezone,(SELECT count(*) FROM versions WHERE item_type='Household' AND item_id=72001) AS versions FROM households WHERE id=72001")).await.unwrap().unwrap();
    assert_eq!(
        home.try_get::<String>("", "name").unwrap(),
        "Updated household"
    );
    assert_eq!(
        home.try_get::<String>("", "timezone").unwrap(),
        "Europe/London"
    );
    assert_eq!(home.try_get::<i64>("", "versions").unwrap(), 1);
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn account_profile_survives_zero_memberships_but_not_revoked_or_locked_login() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=EXCLUDED.recovery_saved_at").await.unwrap();
    let service = boot
        .app_context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let origin = url::Url::parse(&service.context().config.base_url)
        .unwrap()
        .origin()
        .ascii_serialization();
    let mut request = server(&boot);
    request.save_cookies();
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", origin)
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(signed_in.status_code(), 200, "{}", signed_in.text());
    let audit = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT whodunnit,audit_context FROM versions WHERE item_type='AuthenticationToken' AND item_id=71001 AND event='auth_token/session/created' ORDER BY id DESC LIMIT 1")).await.unwrap().expect("real login records account-owned authentication audit");
    assert_eq!(audit.try_get::<String>("", "whodunnit").unwrap(), "73001");
    let context: Value = audit.try_get("", "audit_context").unwrap();
    assert_eq!(context["actor_account_id"], 71001);
    assert_eq!(context["actor_person_id"], 73001);
    fixture.admin.execute_unprepared("UPDATE household_memberships SET revoked_at=now(),permissions_version=permissions_version+1 WHERE person_id=73001").await.unwrap();
    let cookie = signed_in
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(';').next())
        .collect::<Vec<_>>()
        .join("; ");
    assert!(!cookie.is_empty());
    let session = request
        .get("/api/auth/get-session")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(session.status_code(), 200, "{}", session.text());
    let session_body: Value = session.json();
    assert_eq!(session_body["user"]["id"], "71001");
    let profile = request
        .get("/profile")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(
        profile.status_code(),
        200,
        "location={:?} body={}",
        profile.headers().get("location"),
        profile.text()
    );
    assert!(profile.text().contains("persistence@example.test"));
    assert_eq!(profile.header("cache-control"), "no-store");
    assert!(profile.text().contains("profile-timezone-form"));
    assert!(profile.text().contains("profile-avatar-controls"));
    assert!(profile.text().contains("profile-security-flow"));
    let denied = request
        .post("/profile")
        .add_header("cookie", cookie.clone())
        .form(&[("setting", "time_zone"), ("time_zone", "Europe/Paris")])
        .await;
    assert_eq!(denied.status_code(), 403);
    let token = profile
        .text()
        .split("name=\"authenticity_token\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_owned();
    let saved = request
        .post("/profile")
        .add_header("cookie", cookie.clone())
        .form(&[
            ("setting", "time_zone"),
            ("time_zone", "Europe/Paris"),
            ("authenticity_token", token.as_str()),
        ])
        .await;
    assert_eq!(saved.status_code(), 303, "{}", saved.text());
    let stored = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT preferences->>'time_zone' AS zone,(SELECT count(*) FROM versions WHERE item_type='Account' AND item_id=71001 AND event='update') AS versions FROM accounts WHERE id=71001")).await.unwrap().unwrap();
    assert_eq!(
        stored.try_get::<String>("", "zone").unwrap(),
        "Europe/Paris"
    );
    assert_eq!(stored.try_get::<i64>("", "versions").unwrap(), 1);
    let renamed = request
        .post("/profile")
        .add_header("cookie", cookie.clone())
        .form(&[
            ("setting", "name"),
            ("name", "Updated account name"),
            ("authenticity_token", token.as_str()),
        ])
        .await;
    assert_eq!(renamed.status_code(), 303, "{}", renamed.text());
    let refreshed = request
        .get("/profile")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(refreshed.status_code(), 200);
    assert!(refreshed.text().contains("Updated account name"));
    let stored = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT a.name,p.clinical_name,(SELECT count(*) FROM versions WHERE item_type='Account' AND item_id=71001 AND object_changes::jsonb ? 'name') AS versions FROM accounts a JOIN people p ON p.id=a.person_id WHERE a.id=71001")).await.unwrap().unwrap();
    assert_eq!(
        stored.try_get::<String>("", "name").unwrap(),
        "Updated account name"
    );
    assert!(
        stored
            .try_get::<Option<String>>("", "clinical_name")
            .unwrap()
            .is_none()
    );
    assert_eq!(stored.try_get::<i64>("", "versions").unwrap(), 1);
    let invalid = request
        .post("/profile")
        .add_header("cookie", cookie.clone())
        .form(&[
            ("setting", "name"),
            ("name", " "),
            ("authenticity_token", token.as_str()),
        ])
        .await;
    assert_eq!(invalid.status_code(), 422);
    fixture.admin.execute_unprepared("INSERT INTO account_lockouts(account_id,deadline,key) VALUES(71001,now()+interval '1 hour','profile-locked')").await.unwrap();
    let locked = request
        .get("/profile")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(locked.status_code(), 303);
    assert_eq!(locked.header("location"), "/login");
    fixture.admin.execute_unprepared("DELETE FROM account_lockouts WHERE account_id=71001;UPDATE identity_sessions SET active=false WHERE account_id=71001").await.unwrap();
    let revoked = request.get("/profile").add_header("cookie", cookie).await;
    assert_eq!(revoked.status_code(), 303);
    assert_eq!(revoked.header("location"), "/login");
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn auth_organization_reads_use_account_backed_participation() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(71003,73003,'Pending dependent adult','pending@example.test',1,now(),now())").await.unwrap();
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=EXCLUDED.recovery_saved_at").await.unwrap();
    fixture.admin.execute_unprepared("INSERT INTO people(id,clinical_name,person_type,has_capacity,created_at,updated_at) VALUES(73904,NULL,0,true,now(),now()); INSERT INTO accounts(id,person_id,name,email,status,preferences,created_at,updated_at) VALUES(71904,73904,'Visible peer','private-peer@example.test',2,'{\"private_marker\":\"never-share\"}',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74904,72001,73904,'owner','active',now(),now(),now()); UPDATE household_memberships SET role='owner' WHERE id IN(74902,74903); INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(92920,'Foreign household','auth-foreign','UTC',now(),now())").await.unwrap();
    let mut request = server(&boot);
    request.save_cookies();
    let service = boot
        .app_context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let origin = url::Url::parse(&service.context().config.base_url)
        .unwrap()
        .origin()
        .ascii_serialization();
    for route in ["list-members", "list-invitations"] {
        let denied = request
            .get(&format!(
                "/api/auth/organization/{route}?organizationId=72001"
            ))
            .await;
        assert_eq!(denied.status_code(), 401);
    }
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", origin)
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(signed_in.status_code(), 200);
    let cookie = signed_in
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(';').next())
        .collect::<Vec<_>>()
        .join("; ");
    let purpose = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT purpose FROM identity_sessions WHERE account_id=71001 AND active ORDER BY created_at DESC LIMIT 1")).await.unwrap().unwrap();
    assert_eq!(
        purpose.try_get::<String>("", "purpose").unwrap(),
        "authenticated"
    );
    let selected = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        request
            .post("/api/auth/organization/set-active")
            .add_header("cookie", cookie.clone())
            .add_header(
                "origin",
                url::Url::parse(&service.context().config.base_url)
                    .unwrap()
                    .origin()
                    .ascii_serialization(),
            )
            .json(&json!({"organizationId":"72001"}))
            .await
    })
    .await
    .expect("household selection must finish without locking its own session");
    assert_eq!(selected.status_code(), 200, "{}", selected.text());
    let members = request
        .get("/api/auth/organization/list-members?organizationId=72001")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(members.status_code(), 200, "{}", members.text());
    let members: Value = members.json();
    assert_eq!(members["members"].as_array().unwrap().len(), 2);
    assert_eq!(members["total"], 2);
    let invitations = request
        .get("/api/auth/organization/list-invitations?organizationId=72001")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(invitations.status_code(), 200, "{}", invitations.text());
    assert_eq!(invitations.json::<Value>(), json!([]));
    assert!(!members.to_string().contains("persistence@example.test"));
    assert!(members.to_string().contains("Visible peer"));
    assert!(!members.to_string().contains("private-peer@example.test"));
    assert!(!members.to_string().contains("never-share"));
    let counted = request
        .get("/api/auth/organization/list-members?organizationId=72001&limit=0")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(counted.status_code(), 200);
    let counted: Value = counted.json();
    assert_eq!(counted["members"], json!([]));
    assert_eq!(counted["total"], 2);
    let owners = request.get("/api/auth/organization/list-members?organizationId=72001&limit=0&filterField=role&filterValue=owner").add_header("cookie",cookie.clone()).await;
    assert_eq!(owners.status_code(), 200);
    assert_eq!(owners.json::<Value>()["total"], 1);
    let by_slug = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        request
            .get("/api/auth/organization/list-members?organizationSlug=persistence-fixture")
            .add_header("cookie", cookie.clone())
            .await
    })
    .await
    .expect("slug roster must not lock its own account transaction");
    assert_eq!(by_slug.status_code(), 200);
    assert_eq!(by_slug.json::<Value>()["total"], 2);
    fixture.admin.execute_unprepared("INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,accepted_at,created_at,updated_at) VALUES(94901,72001,'pending-history@example.test','member','history-pending',74904,now()+interval '1 day',NULL,now(),now()),(94902,72001,'accepted-history@example.test','member','history-accepted',74904,now()-interval '1 day',now(),now(),now())").await.unwrap();
    for change in [
        "UPDATE household_memberships SET status='suspended' WHERE id=74904",
        "UPDATE household_memberships SET status='active',revoked_at=now() WHERE id=74904",
        "UPDATE household_memberships SET revoked_at=NULL WHERE id=74904; UPDATE accounts SET status=3 WHERE id=71904",
    ] {
        fixture.admin.execute_unprepared(change).await.unwrap();
        let history = request
            .get("/api/auth/organization/list-invitations?organizationId=72001")
            .add_header("cookie", cookie.clone())
            .await;
        assert_eq!(history.status_code(), 200, "{}", history.text());
        let history: Value = history.json();
        let invitations = history.as_array().unwrap();
        assert_eq!(invitations.len(), 2);
        assert!(
            invitations
                .iter()
                .all(|invitation| invitation["inviterId"] == "71904")
        );
        assert!(!history.to_string().contains("private-peer@example.test"));
        assert!(!history.to_string().contains("never-share"));
    }
    for route in ["list-members", "list-invitations"] {
        let denied = request
            .get(&format!(
                "/api/auth/organization/{route}?organizationId=92920"
            ))
            .add_header("cookie", cookie.clone())
            .await;
        assert!(matches!(denied.status_code().as_u16(), 403 | 404));
    }
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    for route in ["list-members", "list-invitations"] {
        let denied = request
            .get(&format!(
                "/api/auth/organization/{route}?organizationId=72001"
            ))
            .add_header("cookie", cookie.clone())
            .await;
        assert!(matches!(denied.status_code().as_u16(), 403 | 404));
        assert!(!denied.text().contains("Visible peer"));
        assert!(!denied.text().contains("private-peer@example.test"));
    }
    fixture.admin.execute_unprepared("UPDATE household_memberships SET role='administrator',status='suspended' WHERE id=74001").await.unwrap();
    for route in ["list-members", "list-invitations"] {
        let denied = request
            .get(&format!(
                "/api/auth/organization/{route}?organizationId=72001"
            ))
            .add_header("cookie", cookie.clone())
            .await;
        assert!(matches!(denied.status_code().as_u16(), 403 | 404));
        assert!(!denied.text().contains("Visible peer"));
        assert!(!denied.text().contains("private-peer@example.test"));
    }
    fixture
        .admin
        .execute_unprepared(
            "UPDATE household_memberships SET status='active',revoked_at=now() WHERE id=74001",
        )
        .await
        .unwrap();
    for route in ["list-members", "list-invitations"] {
        let denied = request
            .get(&format!(
                "/api/auth/organization/{route}?organizationId=72001"
            ))
            .add_header("cookie", cookie.clone())
            .await;
        assert!(matches!(denied.status_code().as_u16(), 403 | 404));
        assert!(!denied.text().contains("Visible peer"));
        assert!(!denied.text().contains("private-peer@example.test"));
    }
    let unchanged = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS selected FROM identity_sessions WHERE account_id=71001 AND active AND active_household_id=72001")).await.unwrap().unwrap();
    assert_eq!(unchanged.try_get::<i64>("", "selected").unwrap(), 1);
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn household_selection_is_per_session_and_rejects_stale_scope_and_invalid_targets() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=EXCLUDED.recovery_saved_at; INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(92921,'Second device household','runtime-device-second','UTC',now(),now()),(92920,'Foreign household','runtime-device-foreign','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(92922,92921,73001,'administrator','active',now(),now(),now())").await.unwrap();
    let request = server(&boot);
    let service = boot
        .app_context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let origin = url::Url::parse(&service.context().config.base_url)
        .unwrap()
        .origin()
        .ascii_serialization();
    let mut cookies = Vec::new();
    let mut sessions = Vec::new();
    for device in ["device-one", "device-two"] {
        let login = request
            .post("/api/auth/sign-in/email")
            .add_header("origin", origin.clone())
            .add_header("user-agent", device)
            .json(&json!({"email":"persistence@example.test","password":"password"}))
            .await;
        assert_eq!(login.status_code(), 200);
        let cookie = login
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .filter_map(|value| value.split(';').next())
            .collect::<Vec<_>>()
            .join("; ");
        let session = request
            .get("/api/auth/get-session")
            .add_header("cookie", cookie.clone())
            .await;
        assert_eq!(session.status_code(), 200);
        sessions.push(
            session.json::<Value>()["session"]["id"]
                .as_str()
                .unwrap()
                .to_owned(),
        );
        cookies.push(cookie);
    }
    assert_ne!(sessions[0], sessions[1]);
    for (cookie, household) in cookies.iter().zip(["72001", "92921"]) {
        let selected = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            request
                .post("/api/auth/organization/set-active")
                .add_header("origin", origin.clone())
                .add_header("cookie", cookie.clone())
                .json(&json!({"organizationId":household}))
                .await
        })
        .await
        .expect("selection must finish");
        assert_eq!(selected.status_code(), 200);
    }
    let untrusted = request
        .post("/api/auth/organization/set-active")
        .add_header("origin", "https://untrusted.example.test")
        .add_header("cookie", cookies[0].clone())
        .json(&json!({"organizationId":"92921"}))
        .await;
    assert_eq!(untrusted.status_code(), 403);
    for target in ["92920", "99999"] {
        let denied = request
            .post("/api/auth/organization/set-active")
            .add_header("origin", origin.clone())
            .add_header("cookie", cookies[0].clone())
            .json(&json!({"organizationId":target}))
            .await;
        assert!(matches!(denied.status_code().as_u16(), 403 | 404));
    }
    for (cookie, household) in cookies.iter().zip(["72001", "92921"]) {
        let session = request
            .get("/api/auth/get-session")
            .add_header("cookie", cookie.clone())
            .await;
        assert_eq!(session.status_code(), 200);
        assert_eq!(
            session.json::<Value>()["session"]["activeOrganizationId"],
            household
        );
    }
    for route in ["list-members", "list-invitations"] {
        let mismatch = request
            .get(&format!(
                "/api/auth/organization/{route}?organizationId=92921"
            ))
            .add_header("cookie", cookies[0].clone())
            .await;
        assert_eq!(mismatch.status_code(), 403);
        assert!(!mismatch.text().contains("Synthetic adult"));
    }
    fixture.admin.execute_unprepared("UPDATE household_memberships SET revoked_at=now(),permissions_version=permissions_version+1 WHERE id=92922").await.unwrap();
    let revoked = request
        .post("/api/auth/organization/set-active")
        .add_header("origin", origin.clone())
        .add_header("cookie", cookies[1].clone())
        .json(&json!({"organizationId":"92921"}))
        .await;
    assert!(matches!(revoked.status_code().as_u16(), 403 | 404));
    let denied = request
        .get("/api/auth/organization/list-members?organizationId=92921")
        .add_header("cookie", cookies[1].clone())
        .await;
    assert!(matches!(denied.status_code().as_u16(), 403 | 404));
    let retained = request
        .get("/api/auth/organization/list-members?organizationId=72001")
        .add_header("cookie", cookies[0].clone())
        .await;
    assert_eq!(retained.status_code(), 200);
    for state in [
        "expires_at=now()-interval '1 second'",
        "expires_at=now()+interval '1 hour',purpose='enrolment'",
        "purpose='authenticated',active=false",
    ] {
        fixture
            .admin
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                format!("UPDATE identity_sessions SET {state} WHERE id=$1"),
                [sessions[1].clone().into()],
            ))
            .await
            .unwrap();
        let denied = request
            .post("/api/auth/organization/set-active")
            .add_header("origin", origin.clone())
            .add_header("cookie", cookies[1].clone())
            .json(&json!({"organizationId":"72001"}))
            .await;
        assert_eq!(denied.status_code(), 401);
    }
    let selection = fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT active_household_id FROM identity_sessions WHERE id=$1",
            [sessions[0].clone().into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        selection.try_get::<i64>("", "active_household_id").unwrap(),
        72001
    );
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn stale_household_person_form_keeps_its_original_scope_after_selection_changes() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=EXCLUDED.recovery_saved_at; INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(92921,'Second form household','runtime-form-second','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(92922,92921,73001,'administrator','active',now(),now(),now())").await.unwrap();
    let mut request = server(&boot);
    request.save_cookies();
    let service = boot
        .app_context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let origin = url::Url::parse(&service.context().config.base_url)
        .unwrap()
        .origin()
        .ascii_serialization();
    let login = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", origin.clone())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(login.status_code(), 200);
    let cookie = login
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(';').next())
        .collect::<Vec<_>>()
        .join("; ");
    let selected = request
        .post("/api/auth/organization/set-active")
        .add_header("origin", origin.clone())
        .add_header("cookie", cookie.clone())
        .json(&json!({"organizationId":"72001"}))
        .await;
    assert_eq!(selected.status_code(), 200);
    let form = request
        .get("/households/persistence-fixture/people/new")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(form.status_code(), 200);
    assert!(
        form.text()
            .contains("action=\"/households/persistence-fixture/people\"")
    );
    assert!(!form.text().contains("name=\"email\""));
    let csrf = form
        .text()
        .split("name=\"authenticity_token\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_owned();
    let switched = request
        .post("/api/auth/organization/set-active")
        .add_header("origin", origin.clone())
        .add_header("cookie", cookie.clone())
        .json(&json!({"organizationId":"92921"}))
        .await;
    assert_eq!(switched.status_code(), 200);
    let created = request
        .post("/households/persistence-fixture/people")
        .add_header("cookie", cookie.clone())
        .form(&[
            ("name", "Stale form child"),
            ("date_of_birth", "2020-01-01"),
            ("person_type", "minor"),
            ("has_capacity", "false"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(created.status_code(), 303);
    let persisted = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT p.id,(SELECT count(*) FROM accounts WHERE person_id=p.id) AS accounts,(SELECT count(*) FROM household_memberships WHERE person_id=p.id AND household_id=72001) AS original,(SELECT count(*) FROM household_memberships WHERE person_id=p.id AND household_id=92921) AS second,(SELECT count(*) FROM versions WHERE item_type='Person' AND item_id=p.id AND household_id=72001) AS original_versions,(SELECT count(*) FROM versions WHERE item_type='Person' AND item_id=p.id AND household_id=92921) AS second_versions,(SELECT count(*) FROM api_change_events WHERE record_type='Person' AND record_id=p.id AND household_id=92921) AS second_changes FROM people p WHERE clinical_name='Stale form child'")).await.unwrap().unwrap();
    assert_eq!(persisted.try_get::<i64>("", "accounts").unwrap(), 0);
    assert_eq!(persisted.try_get::<i64>("", "original").unwrap(), 1);
    assert_eq!(persisted.try_get::<i64>("", "second").unwrap(), 0);
    assert_eq!(
        persisted.try_get::<i64>("", "original_versions").unwrap(),
        1
    );
    assert_eq!(persisted.try_get::<i64>("", "second_versions").unwrap(), 0);
    assert_eq!(persisted.try_get::<i64>("", "second_changes").unwrap(), 0);
    fixture.admin.execute_unprepared("UPDATE household_memberships SET revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001").await.unwrap();
    let denied = request
        .post("/households/persistence-fixture/people")
        .add_header("cookie", cookie)
        .form(&[
            ("name", "Rejected stale child"),
            ("date_of_birth", "2020-01-01"),
            ("person_type", "minor"),
            ("has_capacity", "false"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert!(matches!(denied.status_code().as_u16(), 403 | 404));
    let absent = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM people WHERE clinical_name='Rejected stale child'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(absent.try_get::<i64>("", "count").unwrap(), 0);
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn real_signup_atomically_creates_identity_household_and_verification_outbox() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("DELETE FROM app_settings; INSERT INTO app_settings(invite_only,created_at,updated_at) VALUES(false,now(),now())").await.unwrap();
    let service = boot
        .app_context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let origin = url::Url::parse(&service.context().config.base_url)
        .unwrap()
        .origin()
        .ascii_serialization();
    let request = server(&boot);
    let registered = request.post("/api/auth/onboarding/signup").add_header("origin",origin.clone())
        .json(&json!({"name":"Runtime owner","email":"runtime-signup@example.test","date_of_birth":"1990-01-01","credential":"password","password":"synthetic-signup-1!"})).await;
    assert_eq!(registered.status_code(), 200, "{}", registered.text());
    let created = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT a.id,a.person_id,a.name,a.password_hash,a.status,p.person_type,p.has_capacity,p.clinical_name,(SELECT count(*) FROM household_memberships m WHERE m.person_id=p.id AND m.role='owner' AND m.status='active' AND m.revoked_at IS NULL) AS memberships,(SELECT count(*) FROM households h WHERE h.created_by_account_id=a.id) AS households,(SELECT count(*) FROM identity_provider_accounts c WHERE c.account_id=a.id AND c.provider_id='credential' AND c.password IS NOT NULL) AS credentials,(SELECT count(*) FROM identity_onboarding o WHERE o.account_id=a.id) AS onboarding,(SELECT count(*) FROM identity_sessions s WHERE s.account_id=a.id) AS sessions,(SELECT count(*) FROM pg_loco_queue q WHERE q.task_data->>'to'=a.email) AS messages,(SELECT count(*) FROM security_audit_events e WHERE e.actor_account_id=a.id) AS audits,(SELECT count(*) FROM api_change_events e WHERE e.account_id=a.id) AS changes FROM accounts a JOIN people p ON p.id=a.person_id WHERE a.email='runtime-signup@example.test'")).await.unwrap().unwrap();
    assert_eq!(
        created.try_get::<String>("", "name").unwrap(),
        "Runtime owner"
    );
    assert!(
        created
            .try_get::<Option<String>>("", "password_hash")
            .unwrap()
            .is_none()
    );
    assert_eq!(created.try_get::<i32>("", "status").unwrap(), 1);
    assert_eq!(created.try_get::<i32>("", "person_type").unwrap(), 0);
    assert!(created.try_get::<bool>("", "has_capacity").unwrap());
    assert!(
        created
            .try_get::<Option<String>>("", "clinical_name")
            .unwrap()
            .is_none()
    );
    for field in [
        "memberships",
        "households",
        "credentials",
        "onboarding",
        "messages",
        "audits",
        "changes",
    ] {
        assert_eq!(created.try_get::<i64>("", field).unwrap(), 1, "{field}");
    }
    assert_eq!(created.try_get::<i64>("", "sessions").unwrap(), 0);
    let snapshot = "SELECT (SELECT count(*) FROM accounts) AS accounts,(SELECT count(*) FROM people) AS people,(SELECT count(*) FROM households) AS households,(SELECT count(*) FROM household_memberships) AS memberships,(SELECT count(*) FROM identity_provider_accounts) AS providers,(SELECT count(*) FROM identity_onboarding) AS onboarding,(SELECT count(*) FROM identity_verifications) AS verifications,(SELECT count(*) FROM pg_loco_queue) AS messages,(SELECT count(*) FROM versions) AS versions,(SELECT count(*) FROM security_audit_events) AS audits,(SELECT count(*) FROM api_change_events) AS changes";
    let before = fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot))
        .await
        .unwrap()
        .unwrap();
    fixture.admin.execute_unprepared("CREATE SEQUENCE runtime_signup_queue_attempt; GRANT USAGE ON SEQUENCE runtime_signup_queue_attempt TO med_tracker_app; CREATE FUNCTION reject_runtime_signup_queue() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('runtime_signup_queue_attempt'); RAISE EXCEPTION 'synthetic registration outbox failure'; END $$; CREATE TRIGGER reject_runtime_signup_queue BEFORE INSERT ON pg_loco_queue FOR EACH ROW EXECUTE FUNCTION reject_runtime_signup_queue()").await.unwrap();
    let failed = request.post("/api/auth/onboarding/signup").add_header("origin",origin)
        .json(&json!({"name":"Rolled back owner","email":"runtime-rollback@example.test","date_of_birth":"1990-01-01","credential":"password","password":"synthetic-signup-1!"})).await;
    assert_eq!(failed.status_code(), 500);
    let attempt = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT is_called FROM runtime_signup_queue_attempt",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(attempt.try_get::<bool>("", "is_called").unwrap());
    let after = fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot))
        .await
        .unwrap()
        .unwrap();
    for field in [
        "accounts",
        "people",
        "households",
        "memberships",
        "providers",
        "onboarding",
        "verifications",
        "messages",
        "versions",
        "audits",
        "changes",
    ] {
        assert_eq!(
            after.try_get::<i64>("", field).unwrap(),
            before.try_get::<i64>("", field).unwrap(),
            "rollback {field}"
        );
    }
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn home_household_selector_persists_selection_with_csrf_and_live_membership() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=EXCLUDED.recovery_saved_at").await.unwrap();
    let mut request = server(&boot);
    request.save_cookies();
    let service = boot
        .app_context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let origin = url::Url::parse(&service.context().config.base_url)
        .unwrap()
        .origin()
        .ascii_serialization();
    let login = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", origin.clone())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(login.status_code(), 200);
    let cookie = login
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(char::from(59)).next())
        .collect::<Vec<_>>()
        .join("; ");
    let home = request.get("/").add_header("cookie", cookie.clone()).await;
    assert_eq!(home.status_code(), 200);
    assert!(home.text().contains("Your households"));
    assert!(home.text().contains("value=\"72001\""));
    assert!(home.text().contains("action=\"/households/select\""));
    assert!(home.text().contains("href=\"/profile\""));
    assert!(home.text().contains("href=\"/account/security\""));
    assert!(!home.text().contains("/multifactor-manage"));
    let csrf = home
        .text()
        .split("name=\"authenticity_token\" value=\"")
        .nth(1)
        .unwrap()
        .split(char::from(34))
        .next()
        .unwrap()
        .to_owned();
    let denied = request
        .post("/households/select")
        .add_header("cookie", cookie.clone())
        .add_header("sec-fetch-site", "same-origin")
        .add_header("origin", origin.clone())
        .form(&[("household_id", "72001"), ("authenticity_token", "invalid")])
        .await;
    assert_eq!(denied.status_code(), 403);
    let cross_site = request
        .post("/households/select")
        .add_header("cookie", cookie.clone())
        .add_header("sec-fetch-site", "cross-site")
        .add_header("origin", "https://untrusted.invalid")
        .form(&[
            ("household_id", "72001"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(cross_site.status_code(), 403);
    let unknown = request
        .post("/households/select")
        .add_header("cookie", cookie.clone())
        .add_header("sec-fetch-site", "same-origin")
        .add_header("origin", origin.clone())
        .form(&[
            ("household_id", "999999"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(unknown.status_code(), 404);
    let unchanged = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS count FROM identity_sessions WHERE account_id=71001 AND active_household_id IS NOT NULL")).await.unwrap().unwrap();
    assert_eq!(unchanged.try_get::<i64>("", "count").unwrap(), 0);
    let selected = request
        .post("/households/select")
        .add_header("cookie", cookie.clone())
        .add_header("sec-fetch-site", "same-origin")
        .add_header("origin", origin.clone())
        .form(&[
            ("household_id", "72001"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(selected.status_code(), 303);
    assert_eq!(
        selected.headers().get("location").unwrap(),
        "/households/persistence-fixture/medications"
    );
    let session = request
        .get("/api/auth/get-session")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(session.status_code(), 200);
    let selected_count = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS count FROM identity_sessions WHERE account_id=71001 AND active_household_id=72001")).await.unwrap().unwrap();
    assert_eq!(selected_count.try_get::<i64>("", "count").unwrap(), 1);
    fixture.admin.execute_unprepared("UPDATE household_memberships SET revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001").await.unwrap();
    let revoked = request
        .post("/households/select")
        .add_header("cookie", cookie.clone())
        .add_header("sec-fetch-site", "same-origin")
        .add_header("origin", origin)
        .form(&[
            ("household_id", "72001"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert!(matches!(revoked.status_code().as_u16(), 403 | 404));
    let empty = request.get("/").add_header("cookie", cookie.clone()).await;
    assert_eq!(empty.status_code(), 200);
    assert!(empty.text().contains("href=\"/profile\""));
    assert!(!empty.text().contains("action=\"/households/select\""));
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn split_cookie_headers_preserve_browser_identity_and_household_selection() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=EXCLUDED.recovery_saved_at").await.unwrap();
    let mut request = server(&boot);
    request.save_cookies();
    let service = boot
        .app_context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let origin = url::Url::parse(&service.context().config.base_url)
        .unwrap()
        .origin()
        .ascii_serialization();
    let login = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", origin.clone())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(login.status_code(), 200);
    let cookie = login
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(char::from(59)).next())
        .collect::<Vec<_>>()
        .join("; ");
    let home = request
        .get("/")
        .clear_cookies()
        .add_header("cookie", cookie.clone())
        .add_header("cookie", "unrelated_fixture=value")
        .await;
    assert_eq!(home.status_code(), 200);
    assert!(home.text().contains("Your households"));
    let csrf = home
        .text()
        .split("name=\"authenticity_token\" value=\"")
        .nth(1)
        .unwrap()
        .split(char::from(34))
        .next()
        .unwrap()
        .to_owned();
    let selected = request
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .add_header("origin", origin)
        .add_header("cookie", cookie.clone())
        .add_header("cookie", "unrelated_fixture=value")
        .form(&[
            ("household_id", "72001"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(selected.status_code(), 303);
    let session = request
        .get("/api/auth/get-session")
        .clear_cookies()
        .add_header("cookie", cookie)
        .add_header("cookie", "unrelated_fixture=value")
        .await;
    assert_eq!(session.status_code(), 200);
    let body: Value = session.json();
    assert_eq!(body["user"]["id"], "71001");
    let stored = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS count FROM identity_sessions WHERE account_id=71001 AND active_household_id=72001")).await.unwrap().unwrap();
    assert_eq!(stored.try_get::<i64>("", "count").unwrap(), 1);
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn invitation_browser_form_offers_membership_without_inferred_guardian_authority() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=EXCLUDED.recovery_saved_at").await.unwrap();
    let mut request = server(&boot);
    request.save_cookies();
    let service = boot
        .app_context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let origin = url::Url::parse(&service.context().config.base_url)
        .unwrap()
        .origin()
        .ascii_serialization();
    let login = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", origin)
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(login.status_code(), 200);
    let form = request
        .get("/households/persistence-fixture/admin/invitations")
        .await;
    assert_eq!(form.status_code(), 200);
    assert!(form.text().contains("id=\"invitation-form\""));
    for obsolete in ["relationship_type", "access_level", "dependent_ids"] {
        assert!(
            !form.text().contains(obsolete),
            "obsolete invitation authority field: {obsolete}"
        );
    }
    let csrf = form
        .text()
        .split("name=\"authenticity_token\" value=\"")
        .nth(1)
        .unwrap()
        .split(char::from(34))
        .next()
        .unwrap()
        .to_owned();
    let denied = request
        .post("/households/persistence-fixture/admin/invitations")
        .form(&[
            ("email", "invited-browser@example.test"),
            ("membership_role", "member"),
            ("authenticity_token", "invalid"),
        ])
        .await;
    assert_eq!(denied.status_code(), 403);
    let created = request
        .post("/households/persistence-fixture/admin/invitations")
        .form(&[
            ("email", "invited-browser@example.test"),
            ("membership_role", "member"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(created.status_code(), 303);
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM household_invitations WHERE email='invited-browser@example.test') AS invitations,(SELECT count(*) FROM pg_loco_queue WHERE task_data->>'to'='invited-browser@example.test') AS messages,(SELECT count(*) FROM household_invitation_grants) AS grants,(SELECT count(*) FROM carer_relationships) AS relationships")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "invitations").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "messages").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "grants").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "relationships").unwrap(), 0);
    let refreshed = request
        .get("/households/persistence-fixture/admin/invitations")
        .await;
    assert_eq!(refreshed.status_code(), 200);
    assert!(refreshed.text().contains("invited-browser@example.test"));
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn invitation_browser_acceptance_keeps_identity_and_rolls_back_change_failure() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=EXCLUDED.recovery_saved_at; INSERT INTO people(id,person_type,has_capacity,created_at,updated_at) VALUES(94101,0,true,now(),now()); INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(94102,94101,'Browser inviter','browser-inviter@example.test',2,now(),now()); INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(94103,'Browser invited household','browser-invited','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(94104,94103,94101,'owner','active',now(),now(),now())").await.unwrap();
    let invitation = "synthetic-browser-invitation";
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(94105,94103,'persistence@example.test','member',$1,94104,now()+interval '1 day',now(),now())",[hex::encode(Sha256::digest(invitation.as_bytes())).into()])).await.unwrap();
    let mut request = server(&boot);
    request.save_cookies();
    let service = boot
        .app_context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let origin = url::Url::parse(&service.context().config.base_url)
        .unwrap()
        .origin()
        .ascii_serialization();
    let login = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", origin)
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(login.status_code(), 200);
    let preview = request
        .get(&format!("/invitations/accept?token={invitation}"))
        .await;
    assert_eq!(preview.status_code(), 200);
    assert!(preview.text().contains("Browser invited household"));
    let csrf = preview
        .text()
        .split("name=\"authenticity_token\" value=\"")
        .nth(1)
        .unwrap()
        .split(char::from(34))
        .next()
        .unwrap()
        .to_owned();
    let form = [("token", invitation), ("authenticity_token", csrf.as_str())];
    let snapshot = "SELECT (SELECT count(*) FROM accounts) AS accounts,(SELECT count(*) FROM people) AS people,(SELECT count(*) FROM household_memberships) AS memberships,(SELECT count(*) FROM household_invitations WHERE accepted_at IS NOT NULL) AS accepted,(SELECT count(*) FROM security_audit_events) AS audits,(SELECT count(*) FROM api_change_events) AS changes,(SELECT count(*) FROM pg_loco_queue) AS messages";
    let before = fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot))
        .await
        .unwrap()
        .unwrap();
    for mutation in [
        "UPDATE household_invitations SET email='another-account@example.test' WHERE id=94105",
        "UPDATE household_invitations SET expires_at=now()-interval '1 second' WHERE id=94105",
        "UPDATE household_invitations SET revoked_at=now() WHERE id=94105",
        "UPDATE household_memberships SET role='member' WHERE id=94104",
        "UPDATE household_memberships SET revoked_at=now() WHERE id=94104",
        "INSERT INTO account_lockouts(account_id,deadline,key) VALUES(94102,now()+interval '1 hour','browser-inviter-lockout')",
        "UPDATE people SET person_type=2,has_capacity=false WHERE id=73001",
    ] {
        fixture.admin.execute_unprepared(mutation).await.unwrap();
        let denied = request.post("/invitations/accept").form(&form).await;
        assert_eq!(denied.status_code(), 404, "{mutation}");
        let unchanged = fixture
            .admin
            .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot))
            .await
            .unwrap()
            .unwrap();
        for field in [
            "accounts",
            "people",
            "memberships",
            "accepted",
            "audits",
            "changes",
            "messages",
        ] {
            assert_eq!(
                unchanged.try_get::<i64>("", field).unwrap(),
                before.try_get::<i64>("", field).unwrap(),
                "denial {field}"
            );
        }
        fixture.admin.execute_unprepared("UPDATE household_invitations SET email='persistence@example.test',expires_at=now()+interval '1 day',revoked_at=NULL WHERE id=94105; UPDATE household_memberships SET role='owner',revoked_at=NULL WHERE id=94104; DELETE FROM account_lockouts WHERE account_id=94102; UPDATE people SET person_type=0,has_capacity=true WHERE id=73001").await.unwrap();
    }
    fixture.admin.execute_unprepared("CREATE SEQUENCE browser_invite_change_reached; GRANT USAGE ON SEQUENCE browser_invite_change_reached TO med_tracker_app,med_tracker_owner; CREATE FUNCTION reject_browser_invite_change() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('public.browser_invite_change_reached'); RAISE EXCEPTION 'Synthetic invitation change failure' USING ERRCODE='42501'; END $$; CREATE TRIGGER reject_browser_invite_change BEFORE INSERT ON api_change_events FOR EACH ROW EXECUTE FUNCTION reject_browser_invite_change()").await.unwrap();
    let failed = request.post("/invitations/accept").form(&form).await;
    assert_eq!(failed.status_code(), 503);
    let reached = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT is_called FROM browser_invite_change_reached",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(reached.try_get::<bool>("", "is_called").unwrap());
    let after = fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot))
        .await
        .unwrap()
        .unwrap();
    for field in [
        "accounts",
        "people",
        "memberships",
        "accepted",
        "audits",
        "changes",
        "messages",
    ] {
        assert_eq!(
            after.try_get::<i64>("", field).unwrap(),
            before.try_get::<i64>("", field).unwrap(),
            "rollback {field}"
        );
    }
    fixture.admin.execute_unprepared("DROP TRIGGER reject_browser_invite_change ON api_change_events; DROP FUNCTION reject_browser_invite_change()").await.unwrap();
    fixture.admin.execute_unprepared("REVOKE EXECUTE ON FUNCTION med_tracker.accept_household_invitation(text,text) FROM med_tracker_app").await.unwrap();
    let unavailable = request.post("/invitations/accept").form(&form).await;
    assert_eq!(unavailable.status_code(), 503);
    let unchanged = fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot))
        .await
        .unwrap()
        .unwrap();
    for field in [
        "accounts",
        "people",
        "memberships",
        "accepted",
        "audits",
        "changes",
        "messages",
    ] {
        assert_eq!(
            unchanged.try_get::<i64>("", field).unwrap(),
            before.try_get::<i64>("", field).unwrap(),
            "missing execute {field}"
        );
    }
    fixture.admin.execute_unprepared("GRANT EXECUTE ON FUNCTION med_tracker.accept_household_invitation(text,text) TO med_tracker_app").await.unwrap();
    fixture.admin.execute_unprepared("CREATE FUNCTION pause_concurrent_household() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.name='Concurrent own household' THEN PERFORM pg_advisory_xact_lock(94601); END IF; RETURN NEW; END $$; CREATE TRIGGER pause_concurrent_household BEFORE INSERT ON households FOR EACH ROW EXECUTE FUNCTION pause_concurrent_household()").await.unwrap();
    let household_barrier = fixture.admin.begin().await.unwrap();
    let blocker = household_barrier
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_backend_pid() AS pid,id FROM households WHERE id=94103 FOR UPDATE",
        ))
        .await
        .unwrap()
        .unwrap();
    let blocker_pid = blocker.try_get::<i32>("", "pid").unwrap();
    let creation_barrier = fixture.admin.begin().await.unwrap();
    creation_barrier
        .execute_unprepared("SELECT pg_advisory_xact_lock(94601)")
        .await
        .unwrap();
    let start_creation = tokio::sync::Notify::new();
    let acceptance = request.post("/invitations/accept").form(&form);
    let creation = async {
        start_creation.notified().await;
        request
            .post("/households")
            .add_header("sec-fetch-site", "same-origin")
            .form(&[
                ("name", "Concurrent own household"),
                ("timezone", "UTC"),
                ("authenticity_token", csrf.as_str()),
            ])
            .await
    };
    let runtime_role = url::Url::parse(&fixture.runtime_uri)
        .unwrap()
        .username()
        .to_owned();
    let release = async {
        let acceptance_waited = tokio::time::timeout(std::time::Duration::from_secs(3),async {
            loop {
                let row = fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                    "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND usename=$1 AND $2::int=ANY(pg_blocking_pids(pid)) AND query LIKE '%accept_household_invitation%') AS waiting",
                    [runtime_role.clone().into(),blocker_pid.into()])).await.unwrap().unwrap();
                if row.try_get::<bool>("","waiting").unwrap() { break; }
                tokio::task::yield_now().await;
            }
        }).await.is_ok();
        start_creation.notify_one();
        let overlapped = tokio::time::timeout(std::time::Duration::from_secs(3),async {
            loop {
                let row = fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                    "SELECT count(DISTINCT pid) AS waiting FROM pg_stat_activity WHERE datname=current_database() AND usename=$1 AND wait_event_type='Lock' AND cardinality(pg_blocking_pids(pid))>0",
                    [runtime_role.clone().into()])).await.unwrap().unwrap();
                if row.try_get::<i64>("","waiting").unwrap() >= 2 { break; }
                tokio::task::yield_now().await;
            }
        }).await.is_ok();
        household_barrier.commit().await.unwrap();
        creation_barrier.commit().await.unwrap();
        (acceptance_waited, overlapped)
    };
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        tokio::join!(acceptance, creation, release)
    })
    .await;
    fixture.admin.execute_unprepared("DROP TRIGGER pause_concurrent_household ON households; DROP FUNCTION pause_concurrent_household()").await.unwrap();
    let (accepted, created, observed) =
        outcome.expect("overlapping household mutations must finish within the bounded deadline");
    assert_eq!(
        observed,
        (true, true),
        "both requests must reach observed database lock barriers"
    );
    assert_eq!(accepted.status_code(), 303, "{}", accepted.text());
    assert_eq!(created.status_code(), 303, "{}", created.text());
    let selected = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT h.slug,(SELECT count(*) FROM household_memberships WHERE household_id=h.id AND person_id=73001 AND role='owner' AND status='active') AS participation,(SELECT count(*) FROM security_audit_events WHERE household_id=h.id AND event_type='household.created') AS audits,(SELECT count(*) FROM api_change_events WHERE household_id=h.id AND record_type='Household') AS changes,(SELECT count(*) FROM identity_sessions WHERE account_id=71001 AND active_household_id=h.id) AS selected FROM households h WHERE name='Concurrent own household'")).await.unwrap().unwrap();
    for field in ["participation", "audits", "changes", "selected"] {
        assert_eq!(
            selected.try_get::<i64>("", field).unwrap(),
            1,
            "concurrent {field}"
        );
    }
    assert_eq!(
        created.headers().get("location").unwrap(),
        format!(
            "/households/{}/medications",
            selected.try_get::<String>("", "slug").unwrap()
        )
        .as_str()
    );
    for _ in 0..2 {
        let accepted = request.post("/invitations/accept").form(&form).await;
        assert_eq!(accepted.status_code(), 303);
        assert_eq!(
            accepted.headers().get("location").unwrap(),
            "/households/browser-invited/medications"
        );
    }
    let final_state = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT person_id,(SELECT count(*) FROM people) AS people,(SELECT count(*) FROM household_memberships WHERE person_id=73001 AND household_id=94103 AND status='active' AND revoked_at IS NULL) AS memberships,(SELECT count(*) FROM security_audit_events WHERE household_id=94103 AND event_type='household_invitation.accepted') AS audits,(SELECT count(*) FROM api_change_events WHERE household_id=94103 AND record_type='HouseholdMembership') AS changes FROM accounts WHERE id=71001")).await.unwrap().unwrap();
    assert_eq!(final_state.try_get::<i64>("", "person_id").unwrap(), 73001);
    assert_eq!(
        final_state.try_get::<i64>("", "people").unwrap(),
        before.try_get::<i64>("", "people").unwrap()
    );
    for field in ["memberships", "audits", "changes"] {
        assert_eq!(final_state.try_get::<i64>("", field).unwrap(), 1, "{field}");
    }
    fixture.admin.execute_unprepared("UPDATE household_memberships SET revoked_at=now(),permissions_version=permissions_version+1 WHERE household_id=94103 AND person_id=73001").await.unwrap();
    let replay = request.post("/invitations/accept").form(&form).await;
    assert_eq!(replay.status_code(), 404);
    fixture.admin.execute_unprepared("INSERT INTO account_lockouts(account_id,deadline,key) VALUES(71001,now()+interval '1 hour','invitation-recipient-lockout')").await.unwrap();
    let locked = request.post("/invitations/accept").form(&form).await;
    assert_eq!(locked.status_code(), 303);
    assert_eq!(locked.headers().get("location").unwrap(), "/login");
    let retained = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS count FROM household_memberships WHERE person_id=73001 AND household_id=94103 AND revoked_at IS NOT NULL")).await.unwrap().unwrap();
    assert_eq!(retained.try_get::<i64>("", "count").unwrap(), 1);
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn active_account_without_membership_creates_and_selects_own_household_atomically() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=EXCLUDED.recovery_saved_at; UPDATE household_memberships SET revoked_at=now(),permissions_version=permissions_version+1 WHERE person_id=73001").await.unwrap();
    let mut request = server(&boot);
    request.save_cookies();
    let service = boot
        .app_context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let origin = url::Url::parse(&service.context().config.base_url)
        .unwrap()
        .origin()
        .ascii_serialization();
    let other_device = server(&boot);
    let other_login = other_device
        .post("/api/auth/sign-in/email")
        .add_header("origin", origin.clone())
        .add_header("user-agent", "Household second device")
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(other_login.status_code(), 200);
    let other_session = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT id FROM identity_sessions WHERE account_id=71001 ORDER BY created_at DESC LIMIT 1")).await.unwrap().unwrap().try_get::<String>("","id").unwrap();
    let login = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", origin)
        .add_header("user-agent", "Household creating device")
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(login.status_code(), 200);
    let page = request.get("/households/new").await;
    assert_eq!(page.status_code(), 200);
    assert!(page.text().contains("Create household"));
    let csrf = page
        .text()
        .split("name=\"authenticity_token\" value=\"")
        .nth(1)
        .unwrap()
        .split(char::from(34))
        .next()
        .unwrap()
        .to_owned();
    let denied = request
        .post("/households")
        .form(&[
            ("name", "My new household"),
            ("timezone", "Europe/London"),
            ("authenticity_token", "invalid"),
        ])
        .await;
    assert_eq!(denied.status_code(), 403);
    let invalid = request
        .post("/households")
        .form(&[
            ("name", ""),
            ("timezone", "Invalid/Zone"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(invalid.status_code(), 422);
    let snapshot = "SELECT (SELECT count(*) FROM accounts) AS accounts,(SELECT count(*) FROM people) AS people,(SELECT count(*) FROM households) AS households,(SELECT count(*) FROM household_memberships) AS memberships,(SELECT count(*) FROM security_audit_events) AS audits,(SELECT count(*) FROM api_change_events) AS changes,(SELECT count(*) FROM identity_sessions WHERE active_household_id IS NOT NULL) AS selections";
    let before = fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot))
        .await
        .unwrap()
        .unwrap();
    fixture
        .admin
        .execute_unprepared("UPDATE people SET person_type=2,has_capacity=false WHERE id=73001")
        .await
        .unwrap();
    let incapable = request
        .post("/households")
        .form(&[
            ("name", "Denied household"),
            ("timezone", "UTC"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(incapable.status_code(), 403);
    fixture
        .admin
        .execute_unprepared("UPDATE people SET person_type=0,has_capacity=true WHERE id=73001")
        .await
        .unwrap();
    fixture.admin.execute_unprepared("CREATE SEQUENCE own_household_change_reached; GRANT USAGE ON SEQUENCE own_household_change_reached TO med_tracker_owner; CREATE FUNCTION reject_own_household_change() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('public.own_household_change_reached'); RAISE EXCEPTION 'Synthetic own household change failure' USING ERRCODE='42501'; END $$; CREATE TRIGGER reject_own_household_change BEFORE INSERT ON api_change_events FOR EACH ROW EXECUTE FUNCTION reject_own_household_change()").await.unwrap();
    let failed = request
        .post("/households")
        .form(&[
            ("name", "My new household"),
            ("timezone", "Europe/London"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(failed.status_code(), 503);
    let reached = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT is_called FROM own_household_change_reached",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(reached.try_get::<bool>("", "is_called").unwrap());
    let after = fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot))
        .await
        .unwrap()
        .unwrap();
    for field in [
        "accounts",
        "people",
        "households",
        "memberships",
        "audits",
        "changes",
        "selections",
    ] {
        assert_eq!(
            after.try_get::<i64>("", field).unwrap(),
            before.try_get::<i64>("", field).unwrap(),
            "rollback {field}"
        );
    }
    fixture.admin.execute_unprepared("DROP TRIGGER reject_own_household_change ON api_change_events; DROP FUNCTION reject_own_household_change()").await.unwrap();
    fixture.admin.execute_unprepared("REVOKE EXECUTE ON FUNCTION med_tracker.create_household(text,text,text,text) FROM med_tracker_app").await.unwrap();
    let unavailable = request
        .post("/households")
        .form(&[
            ("name", "My new household"),
            ("timezone", "Europe/London"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(unavailable.status_code(), 503);
    let unchanged = fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot))
        .await
        .unwrap()
        .unwrap();
    for field in [
        "accounts",
        "people",
        "households",
        "memberships",
        "audits",
        "changes",
        "selections",
    ] {
        assert_eq!(
            unchanged.try_get::<i64>("", field).unwrap(),
            before.try_get::<i64>("", field).unwrap(),
            "missing execute {field}"
        );
    }
    fixture.admin.execute_unprepared("GRANT EXECUTE ON FUNCTION med_tracker.create_household(text,text,text,text) TO med_tracker_app").await.unwrap();
    let created = request
        .post("/households")
        .form(&[
            ("name", "My new household"),
            ("timezone", "Europe/London"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(created.status_code(), 303);
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT h.id,h.slug,h.timezone,m.person_id,m.role,(SELECT count(*) FROM accounts) AS accounts,(SELECT count(*) FROM people) AS people,(SELECT count(*) FROM security_audit_events WHERE household_id=h.id) AS audits,(SELECT count(*) FROM api_change_events WHERE household_id=h.id) AS changes,(SELECT count(*) FROM identity_sessions WHERE account_id=71001 AND active_household_id=h.id) AS selections FROM households h JOIN household_memberships m ON m.household_id=h.id WHERE h.name='My new household' AND h.created_by_account_id=71001")).await.unwrap().unwrap();
    let slug = row.try_get::<String>("", "slug").unwrap();
    assert_eq!(
        created.headers().get("location").unwrap(),
        format!("/households/{slug}/medications").as_str()
    );
    assert_eq!(row.try_get::<i64>("", "person_id").unwrap(), 73001);
    assert_eq!(row.try_get::<String>("", "role").unwrap(), "owner");
    assert_eq!(
        row.try_get::<String>("", "timezone").unwrap(),
        "Europe/London"
    );
    for field in ["accounts", "people"] {
        assert_eq!(
            row.try_get::<i64>("", field).unwrap(),
            before.try_get::<i64>("", field).unwrap()
        );
    }
    for field in ["audits", "changes", "selections"] {
        assert_eq!(row.try_get::<i64>("", field).unwrap(), 1, "{field}");
    }
    let other_selection = fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT active_household_id FROM identity_sessions WHERE id=$1",
            [other_session.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(
        other_selection
            .try_get::<Option<i64>>("", "active_household_id")
            .unwrap()
            .is_none()
    );
    let home = request.get("/").await;
    assert_eq!(home.status_code(), 200);
    assert!(home.text().contains("My new household"));
    assert_eq!(
        request
            .get(&format!("/households/{slug}/medications"))
            .await
            .status_code(),
        200
    );
    fixture.admin.execute_unprepared("INSERT INTO account_lockouts(account_id,deadline,key) VALUES(71001,now()+interval '1 hour','household-creator-lockout')").await.unwrap();
    let locked = request
        .post("/households")
        .form(&[
            ("name", "Locked household"),
            ("timezone", "UTC"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(locked.status_code(), 303);
    assert_eq!(locked.headers().get("location").unwrap(), "/login");
    let absent = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS count FROM households WHERE name IN('Denied household','Locked household')")).await.unwrap().unwrap();
    assert_eq!(absent.try_get::<i64>("", "count").unwrap(), 0);
    drop(other_device);
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn smart_consent_uses_selected_household_and_rechecks_participation() {
    async fn expire_pending(
        session: axum_session::Session<axum_session_sqlx::SessionPgPool>,
        axum::extract::State(ctx): axum::extract::State<loco_rs::app::AppContext>,
    ) -> axum::http::StatusCode {
        let Some(mut pending) = session.get::<serde_json::Value>("oauth_consent") else {
            return axum::http::StatusCode::BAD_REQUEST;
        };
        let clock = ctx
            .db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT clock_timestamp()-interval '1 minute' AS expired_at",
            ))
            .await
            .unwrap()
            .unwrap();
        let expired_at: chrono::DateTime<chrono::Utc> = clock.try_get("", "expired_at").unwrap();
        pending["approval"]["expires_at"] = json!(expired_at);
        session.set("oauth_consent", pending);
        session.set_store(true);
        axum::http::StatusCode::NO_CONTENT
    }
    async fn save_pending(
        session: axum_session::Session<axum_session_sqlx::SessionPgPool>,
    ) -> axum::http::StatusCode {
        let pending = session.get::<serde_json::Value>("oauth_consent").unwrap();
        session.set("fixture_pending_consent", pending);
        session.set_store(true);
        axum::http::StatusCode::NO_CONTENT
    }
    async fn restore_pending(
        session: axum_session::Session<axum_session_sqlx::SessionPgPool>,
        axum::extract::State(ctx): axum::extract::State<loco_rs::app::AppContext>,
    ) -> axum::http::StatusCode {
        let pending = session
            .get::<serde_json::Value>("fixture_pending_consent")
            .unwrap();
        let expires_at = pending["approval"]["expires_at"]
            .as_str()
            .unwrap()
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap();
        let clock = ctx
            .db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT clock_timestamp() < $1 AS valid",
                [expires_at.into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert!(clock.try_get::<bool>("", "valid").unwrap());
        session.set("oauth_consent", pending);
        session.set_store(true);
        axum::http::StatusCode::NO_CONTENT
    }
    let (fixture, mut boot) = boot_fixture().await;
    let fixture_routes = med_tracker::models::identity::browser::route_layers(
        &boot.app_context,
        loco_rs::controller::Routes::new()
            .post("/fixture/expire-consent", expire_pending)
            .post("/fixture/save-consent", save_pending)
            .post("/fixture/restore-consent", restore_pending),
    );
    let mut fixture_router = axum::Router::<loco_rs::app::AppContext>::new();
    for handler in fixture_routes.handlers {
        fixture_router = fixture_router.route(&handler.uri, handler.method);
    }
    boot.router = Some(
        boot.router
            .take()
            .unwrap()
            .merge(fixture_router.with_state(boot.app_context.clone())),
    );
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=EXCLUDED.recovery_saved_at; INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(72002,'Selected SMART household','selected-smart','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74002,72002,73001,'member','active',now(),now(),now()); INSERT INTO oauth_applications(id,account_id,client_id,client_kind,name,redirect_uri,scopes,token_endpoint_auth_method,created_at,updated_at) VALUES(75002,71001,'smart-selection','integration','SMART selection client','https://example.test/callback','patient/*.read','none',now(),now())").await.unwrap();
    let service = boot
        .app_context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let origin = url::Url::parse(&service.context().config.base_url)
        .unwrap()
        .origin()
        .ascii_serialization();
    let mut request = server(&boot);
    request.save_cookies();
    let login = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", origin.clone())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(login.status_code(), 200);
    let cookie = login
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(';').next())
        .collect::<Vec<_>>()
        .join("; ");
    let selected = request
        .post("/api/auth/organization/set-active")
        .add_header("cookie", cookie.clone())
        .add_header("origin", origin.clone())
        .json(&json!({"organizationId":"72002"}))
        .await;
    assert_eq!(selected.status_code(), 200);
    let query = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs([
            ("client_id", "smart-selection"),
            ("redirect_uri", "https://example.test/callback"),
            ("response_type", "code"),
            ("scope", "patient/*.read"),
            ("state", "selection-regression"),
            (
                "code_challenge",
                "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
            ),
            ("code_challenge_method", "S256"),
        ])
        .finish();
    let url = format!("/authorize?{query}");
    let consent = request.get(&url).add_header("cookie", cookie.clone()).await;
    assert_eq!(consent.status_code(), 200, "{}", consent.text());
    assert!(
        consent
            .text()
            .contains("Household: Selected SMART household")
    );
    assert!(consent.text().contains("Person: Synthetic adult"));
    assert!(!consent.text().contains("Household: Synthetic household"));
    for stale in [false, true] {
        let mut page = request.get(&url).add_header("cookie", cookie.clone()).await;
        assert_eq!(page.status_code(), 200);
        let mut csrf = page
            .text()
            .split("name=\"authenticity_token\" value=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .to_owned();
        let mut consent_id = page
            .text()
            .split("name=\"consent_request_id\" value=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .to_owned();
        if stale {
            let changed = request
                .post("/api/auth/organization/set-active")
                .add_header("cookie", cookie.clone())
                .add_header("origin", origin.clone())
                .json(&json!({"organizationId":"72001"}))
                .await;
            assert_eq!(changed.status_code(), 200);
            let second_tab = request.get(&url).add_header("cookie", cookie.clone()).await;
            assert_eq!(second_tab.status_code(), 200);
            assert!(second_tab.text().contains("Household: Synthetic household"));
        }
        if !stale {
            let expired = request
                .post("/fixture/expire-consent")
                .add_header("cookie", cookie.clone())
                .add_header("origin", origin.clone())
                .add_header("sec-fetch-site", "same-origin")
                .await;
            assert_eq!(expired.status_code(), 204);
            let expired_approval = request
                .post("/authorize")
                .add_header("cookie", cookie.clone())
                .add_header("origin", origin.clone())
                .add_header("sec-fetch-site", "same-origin")
                .form(&[
                    ("client_id", "smart-selection"),
                    ("redirect_uri", "https://example.test/callback"),
                    ("response_type", "code"),
                    ("state", "selection-regression"),
                    (
                        "code_challenge",
                        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
                    ),
                    ("code_challenge_method", "S256"),
                    ("scope[]", "patient/*.read"),
                    ("authenticity_token", csrf.as_str()),
                    ("consent_request_id", consent_id.as_str()),
                ])
                .await;
            assert_eq!(expired_approval.status_code(), 400);
            page = request.get(&url).add_header("cookie", cookie.clone()).await;
            assert_eq!(page.status_code(), 200);
            csrf = page
                .text()
                .split("name=\"authenticity_token\" value=\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap()
                .to_owned();
            consent_id = page
                .text()
                .split("name=\"consent_request_id\" value=\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap()
                .to_owned();
        }
        let form = [
            ("client_id", "smart-selection"),
            ("redirect_uri", "https://example.test/callback"),
            ("response_type", "code"),
            ("state", "selection-regression"),
            (
                "code_challenge",
                "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
            ),
            ("code_challenge_method", "S256"),
            ("scope[]", "patient/*.read"),
            ("authenticity_token", csrf.as_str()),
            ("consent_request_id", consent_id.as_str()),
        ];
        if !stale {
            fixture.admin.execute_unprepared("CREATE SEQUENCE public.consent_audit_probe; CREATE FUNCTION public.reject_consent_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='smart_oauth.consent_granted' THEN PERFORM nextval('public.consent_audit_probe'); RAISE EXCEPTION 'injected audit failure' USING ERRCODE='42501'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_consent_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION public.reject_consent_audit(); GRANT USAGE ON SEQUENCE public.consent_audit_probe TO med_tracker_app,med_tracker_owner").await.unwrap();
            let failed = request
                .post("/authorize")
                .add_header("cookie", cookie.clone())
                .add_header("origin", origin.clone())
                .add_header("sec-fetch-site", "same-origin")
                .form(&form)
                .await;
            assert_eq!(failed.status_code(), 503);
            let rollback = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
                "SELECT (SELECT is_called FROM public.consent_audit_probe) AS reached,(SELECT count(*) FROM oauth_grants WHERE oauth_application_id=75002) AS grants,(SELECT count(*) FROM security_audit_events WHERE event_type='smart_oauth.consent_granted') AS audits")).await.unwrap().unwrap();
            assert!(rollback.try_get::<bool>("", "reached").unwrap());
            assert_eq!(rollback.try_get::<i64>("", "grants").unwrap(), 0);
            assert_eq!(rollback.try_get::<i64>("", "audits").unwrap(), 0);
            fixture.admin.execute_unprepared("DROP TRIGGER reject_consent_audit ON security_audit_events; DROP FUNCTION public.reject_consent_audit(); DROP SEQUENCE public.consent_audit_probe").await.unwrap();
        }
        if !stale {
            let saved = request
                .post("/fixture/save-consent")
                .add_header("cookie", cookie.clone())
                .add_header("origin", origin.clone())
                .add_header("sec-fetch-site", "same-origin")
                .await;
            assert_eq!(saved.status_code(), 204);
        }
        let first = request
            .post("/authorize")
            .add_header("cookie", cookie.clone())
            .add_header("origin", origin.clone())
            .add_header("sec-fetch-site", "same-origin")
            .form(&form);
        let second = request
            .post("/authorize")
            .add_header("cookie", cookie.clone())
            .add_header("origin", origin.clone())
            .add_header("sec-fetch-site", "same-origin")
            .form(&form);
        let (approved, replayed) =
            tokio::time::timeout(std::time::Duration::from_secs(15), async {
                tokio::join!(first, second)
            })
            .await
            .expect("concurrent consent requests finish");
        let mut statuses = [
            approved.status_code().as_u16(),
            replayed.status_code().as_u16(),
        ];
        statuses.sort();
        assert_eq!(statuses, if stale { [400, 400] } else { [200, 400] });
        if !stale {
            fixture.admin.execute_unprepared("UPDATE oauth_grants SET code=NULL,revoked_at=now() WHERE oauth_application_id=75002").await.unwrap();
            let restored_pending = request
                .post("/fixture/restore-consent")
                .add_header("cookie", cookie.clone())
                .add_header("origin", origin.clone())
                .add_header("sec-fetch-site", "same-origin")
                .await;
            assert_eq!(restored_pending.status_code(), 204);
            let retired_replay = request
                .post("/authorize")
                .add_header("cookie", cookie.clone())
                .add_header("origin", origin.clone())
                .add_header("sec-fetch-site", "same-origin")
                .form(&form)
                .await;
            assert_eq!(retired_replay.status_code(), 400);
        }
        let grants = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS total,count(*) FILTER(WHERE household_membership_id=74002 AND person_id=73001) AS selected,(SELECT count(*) FROM security_audit_events WHERE event_type='smart_oauth.consent_granted' AND household_id=72002) AS audits FROM oauth_grants WHERE oauth_application_id=75002")).await.unwrap().unwrap();
        assert_eq!(grants.try_get::<i64>("", "total").unwrap(), 1);
        assert_eq!(grants.try_get::<i64>("", "selected").unwrap(), 1);
        assert_eq!(grants.try_get::<i64>("", "audits").unwrap(), 1);
    }
    let restored = request
        .post("/api/auth/organization/set-active")
        .add_header("cookie", cookie.clone())
        .add_header("origin", origin.clone())
        .json(&json!({"organizationId":"72002"}))
        .await;
    assert_eq!(restored.status_code(), 200);
    fixture.admin.execute_unprepared("UPDATE household_memberships SET revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74002").await.unwrap();
    let revoked = request.get(&url).add_header("cookie", cookie.clone()).await;
    assert_eq!(revoked.status_code(), 400);
    assert!(!revoked.text().contains("Synthetic household"));
    fixture
        .admin
        .execute_unprepared(
            "UPDATE identity_sessions SET active_household_id=NULL WHERE account_id=71001",
        )
        .await
        .unwrap();
    let unselected = request.get(&url).add_header("cookie", cookie.clone()).await;
    assert_eq!(unselected.status_code(), 400);
    assert!(!unselected.text().contains("Synthetic household"));
    fixture.admin.execute_unprepared("UPDATE household_memberships SET revoked_at=now(),permissions_version=permissions_version+1 WHERE person_id=73001 AND revoked_at IS NULL").await.unwrap();
    let native_query = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs([
            ("client_id", "native"),
            ("redirect_uri", "io.damacus.medtracker:/oauth2redirect"),
            ("response_type", "code"),
            ("scope", "medtracker offline_access"),
            ("state", "native-no-household"),
            (
                "code_challenge",
                "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
            ),
            ("code_challenge_method", "S256"),
        ])
        .finish();
    let native_page = request
        .get(&format!("/authorize?{native_query}"))
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(native_page.status_code(), 200);
    assert!(!native_page.text().contains("Household:"));
    let csrf = native_page
        .text()
        .split("name=\"authenticity_token\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_owned();
    let consent_id = native_page
        .text()
        .split("name=\"consent_request_id\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_owned();
    let native_approval = request
        .post("/authorize")
        .add_header("cookie", cookie)
        .add_header("origin", origin)
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("client_id", "native"),
            ("redirect_uri", "io.damacus.medtracker:/oauth2redirect"),
            ("response_type", "code"),
            ("state", "native-no-household"),
            (
                "code_challenge",
                "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
            ),
            ("code_challenge_method", "S256"),
            ("scope[]", "medtracker"),
            ("scope[]", "offline_access"),
            ("authenticity_token", csrf.as_str()),
            ("consent_request_id", consent_id.as_str()),
        ])
        .await;
    assert_eq!(native_approval.status_code(), 200);
    let native = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT account_id,household_membership_id,person_id,permissions_version FROM oauth_grants WHERE oauth_application_id=75001 AND id<>76001 ORDER BY id DESC LIMIT 1")).await.unwrap().unwrap();
    assert_eq!(native.try_get::<i64>("", "account_id").unwrap(), 71001);
    assert_eq!(
        native
            .try_get::<Option<i64>>("", "household_membership_id")
            .unwrap(),
        None
    );
    assert_eq!(
        native.try_get::<Option<i64>>("", "person_id").unwrap(),
        None
    );
    assert_eq!(
        native
            .try_get::<Option<i32>>("", "permissions_version")
            .unwrap(),
        None
    );
    drop(request);
    close(fixture, boot).await;
}
