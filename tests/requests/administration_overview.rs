use super::*;
use base64::Engine as _;
use sea_orm::TransactionTrait;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn authenticity_token(html: &str) -> Option<&str> {
    html.split("name=\"authenticity_token\" value=\"")
        .nth(1)?
        .split('"')
        .next()
}

fn listed_membership_ids(html: &str) -> Vec<i64> {
    html.split("<table")
        .nth(1)
        .and_then(|table| table.split("</table>").next())
        .unwrap_or_default()
        .split("/admin/memberships/")
        .skip(1)
        .filter_map(|link| link.split("/edit").next()?.parse().ok())
        .collect()
}

async fn select_household(
    request: &TestServer,
    fixture: &Fixture,
    boot: &BootResult,
) -> TestResult {
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=now()").await?;
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(signed_in.status_code(), 200, "{}", signed_in.text());
    let chooser = request.get("/households/choose").await;
    let chooser_html = chooser.text();
    let token = authenticity_token(&chooser_html).ok_or("household form token missing")?;
    let selected = request
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("household_id", "72001"), ("authenticity_token", token)])
        .await;
    assert_eq!(selected.status_code(), 303, "{}", selected.text());
    Ok(())
}

#[tokio::test]
async fn member_administration_lists_inactive_identity_and_changes_membership_state() -> TestResult
{
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    fixture.admin.execute_unprepared("INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73006,'Hidden person name',0,true,now(),now()),(73007,'Other hidden person',0,true,now(),now()); INSERT INTO accounts(id,person_id,name,email,password_hash,status,created_at,updated_at) VALUES(71006,73006,'Inactive member','inactive@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()),(71007,73007,NULL,'fallback@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74006,72001,73006,'member','suspended',now(),now(),now()),(74007,72001,73007,'member','suspended',now(),now(),now())").await?;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&request, &fixture, &boot).await?;
    let list = request
        .get("/households/persistence-fixture/admin/users?status=suspended")
        .await;
    let html = list.text();
    assert_eq!(list.status_code(), 200, "{html}");
    assert!(html.contains("Inactive member"), "{html}");
    assert!(html.contains("inactive@example.test"), "{html}");
    assert!(html.contains("fallback@example.test"), "{html}");
    assert!(!html.contains("Hidden person name"), "{html}");
    fixture.admin.execute_unprepared("INSERT INTO users(id,person_id,email_address,active,created_at,updated_at) VALUES(74006,73007,'legacy-collision@example.test',true,now(),now())").await?;
    let legacy_collision = request
        .get("/households/persistence-fixture/admin/users/74006/edit")
        .await;
    assert!(
        legacy_collision
            .text()
            .contains("Edit household role for fallback@example.test"),
        "{}",
        legacy_collision.text()
    );
    let member_editor = request.get("/households/persistence-fixture/admin/memberships/74006/edit?status=suspended&sort=email&direction=desc&page=1&lang=es").await;
    assert_eq!(member_editor.status_code(), 200, "{}", member_editor.text());
    assert!(
        member_editor
            .text()
            .contains("Edit household role for Inactive member")
    );
    assert!(member_editor.text().contains("/admin/users?q=&amp;role=all&amp;status=suspended&amp;sort=email&amp;direction=desc&amp;page=1&amp;lang=es"));
    let editor_csrf = authenticity_token(&member_editor.text())
        .ok_or("editor token missing")?
        .to_owned();
    let stateful_save = request
        .post("/households/persistence-fixture/admin/memberships/74006/membership_role")
        .form(&[
            ("role", "member"),
            ("role_filter", "all"),
            ("status", "suspended"),
            ("sort", "email"),
            ("direction", "desc"),
            ("page", "1"),
            ("lang", "es"),
            ("authenticity_token", editor_csrf.as_str()),
        ])
        .await;
    assert_eq!(stateful_save.status_code(), 303, "{}", stateful_save.text());
    assert_eq!(
        stateful_save.header("location").to_str()?,
        "/households/persistence-fixture/admin/users?q=&role=all&status=suspended&sort=email&direction=desc&page=1&lang=es"
    );
    assert!(html.contains("suspended"), "{html}");
    assert!(html.contains("Restore"), "{html}");
    let search = request.get("/households/persistence-fixture/admin/users?q=INACTIVE%40EXAMPLE.TEST&role=member&status=suspended&sort=email&direction=desc").await;
    assert_eq!(search.status_code(), 200, "{}", search.text());
    assert!(search.text().contains("inactive@example.test"));
    assert!(!search.text().contains("fallback@example.test"));
    assert_eq!(
        request
            .get("/households/persistence-fixture/admin/users?sort=name%3BDROP%20TABLE%20accounts")
            .await
            .status_code(),
        422
    );
    let role = request
        .get("/households/persistence-fixture/admin/memberships/74006/edit")
        .await;
    assert_eq!(role.status_code(), 200, "{}", role.text());
    assert!(
        role.text()
            .contains("Edit household role for Inactive member")
    );
    assert_eq!(
        request
            .get("/households/persistence-fixture/admin/users/77001/edit")
            .await
            .status_code(),
        200
    );
    let csrf = authenticity_token(&html).ok_or("member form token missing")?;
    let edited_role = request
        .post("/households/persistence-fixture/admin/memberships/74006/membership_role")
        .form(&[("role", "administrator"), ("authenticity_token", csrf)])
        .await;
    assert_eq!(edited_role.status_code(), 303, "{}", edited_role.text());
    let restored_role = request
        .post("/households/persistence-fixture/admin/memberships/74006/membership_role")
        .form(&[("role", "member"), ("authenticity_token", csrf)])
        .await;
    assert_eq!(restored_role.status_code(), 303, "{}", restored_role.text());
    let missing_csrf = request
        .post("/households/persistence-fixture/admin/users/74006/restore")
        .form(&[("confirmation", "restored")])
        .await;
    assert_eq!(missing_csrf.status_code(), 403);
    let foreign = request
        .post("/households/persistence-fixture/admin/users/74002/revoke")
        .form(&[("confirmation", "revoked"), ("authenticity_token", csrf)])
        .await;
    assert_eq!(foreign.status_code(), 404, "{}", foreign.text());
    let restored = request
        .post("/households/persistence-fixture/admin/users/74006/restore")
        .form(&[
            ("confirmation", "restored"),
            ("authenticity_token", csrf),
            ("status", "suspended"),
        ])
        .await;
    assert_eq!(restored.status_code(), 303, "{}", restored.text());
    assert!(
        restored
            .header("location")
            .to_str()?
            .contains("notice=restored")
    );
    let active = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,permissions_version FROM household_memberships WHERE id=74006",
        ))
        .await?
        .ok_or("membership missing")?;
    assert_eq!(active.try_get::<String>("", "status")?, "active");
    assert_eq!(active.try_get::<i32>("", "permissions_version")?, 4);
    let active_list = request
        .get("/households/persistence-fixture/admin/users")
        .await;
    let active_html = active_list.text();
    let next_csrf = authenticity_token(&active_html).ok_or("active form token missing")?;
    let suspended = request
        .post("/households/persistence-fixture/admin/users/74006/suspend")
        .form(&[
            ("confirmation", "suspended"),
            ("authenticity_token", next_csrf),
        ])
        .await;
    assert_eq!(suspended.status_code(), 303, "{}", suspended.text());
    assert_eq!(
        suspended.header("location").to_str()?,
        "/households/persistence-fixture/admin/users?notice=suspended"
    );
    let suspended_notice = request.get(suspended.header("location").to_str()?).await;
    assert_eq!(
        suspended_notice.status_code(),
        200,
        "{}",
        suspended_notice.text()
    );
    assert!(
        suspended_notice.text().contains("Membership Suspended."),
        "{}",
        suspended_notice.text()
    );
    let revoked = request
        .post("/households/persistence-fixture/admin/users/74006/revoke")
        .form(&[
            ("confirmation", "revoked"),
            ("authenticity_token", next_csrf),
        ])
        .await;
    assert_eq!(revoked.status_code(), 303, "{}", revoked.text());
    let forbidden_suspend = request
        .post("/households/persistence-fixture/admin/users/74006/suspend")
        .form(&[
            ("confirmation", "suspended"),
            ("authenticity_token", next_csrf),
        ])
        .await;
    assert_eq!(
        forbidden_suspend.status_code(),
        422,
        "{}",
        forbidden_suspend.text()
    );
    let restored_again = request
        .post("/households/persistence-fixture/admin/users/74006/restore")
        .form(&[
            ("confirmation", "restored"),
            ("authenticity_token", next_csrf),
        ])
        .await;
    assert_eq!(
        restored_again.status_code(),
        303,
        "{}",
        restored_again.text()
    );
    let blocker = fixture.admin.begin().await?;
    let blocker_pid: i32 = blocker
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_backend_pid() AS pid",
        ))
        .await?
        .ok_or("blocker pid missing")?
        .try_get("", "pid")?;
    blocker
        .execute_unprepared("SELECT id FROM household_memberships WHERE id=74006 FOR UPDATE")
        .await?;
    let stale_suspend = async {
        request
            .post("/households/persistence-fixture/admin/users/74006/suspend")
            .form(&[
                ("confirmation", "suspended"),
                ("authenticity_token", next_csrf),
            ])
            .await
    };
    let revoke_under_lock = async {
        let observed = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let row = fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres, "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE '%household_memberships%' AND $1::int=ANY(pg_blocking_pids(pid))) AS waiting", [blocker_pid.into()])).await?.ok_or("lock observation missing")?;
                if row.try_get::<bool>("", "waiting")? { break Ok::<_, Box<dyn std::error::Error>>(true); }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }).await.unwrap_or(Ok(false))?;
        blocker
            .execute_unprepared(
                "UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE id=74006",
            )
            .await?;
        blocker.commit().await?;
        Ok::<_, Box<dyn std::error::Error>>(observed)
    };
    let (stale_result, observed) = tokio::join!(stale_suspend, revoke_under_lock);
    assert!(
        observed?,
        "browser mutation must wait on the membership row lock"
    );
    assert_eq!(stale_result.status_code(), 422, "{}", stale_result.text());
    let raced_state = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status FROM household_memberships WHERE id=74006",
        ))
        .await?
        .ok_or("raced membership missing")?;
    assert_eq!(raced_state.try_get::<String>("", "status")?, "revoked");
    let audit = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*) AS total FROM security_audit_events WHERE household_id=72001 AND metadata->>'target_membership_id'='74006' AND metadata->>'outcome'='success'")).await?.ok_or("audit missing")?;
    assert!(audit.try_get::<i64>("", "total")? >= 4);
    let account = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status FROM accounts WHERE id=71006",
        ))
        .await?
        .ok_or("account missing")?;
    assert_eq!(account.try_get::<i32>("", "status")?, 2);
    fixture.admin.execute_unprepared("INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) SELECT 73100+n,'Bulk member',0,true,now(),now() FROM generate_series(1,27) n; INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) SELECT 74100+n,72001,73100+n,'member','active',now(),now(),now() FROM generate_series(1,27) n").await?;
    let first_page = request.get("/households/persistence-fixture/admin/users?role=member&status=active&sort=name&direction=asc&page=1&lang=es").await;
    let first_html = first_page.text();
    assert_eq!(first_page.status_code(), 200, "{first_html}");
    assert_eq!(first_html.matches("Bulk member</th>").count(), 25);
    assert_eq!(
        listed_membership_ids(&first_html),
        (74101..=74125).collect::<Vec<_>>()
    );
    assert!(first_html.contains("page=2&amp;lang=es"), "{first_html}");
    let second_page = request.get("/households/persistence-fixture/admin/users?role=member&status=active&sort=name&direction=asc&page=2&lang=es").await;
    assert_eq!(second_page.status_code(), 200, "{}", second_page.text());
    assert_eq!(second_page.text().matches("Bulk member</th>").count(), 2);
    assert_eq!(
        listed_membership_ids(&second_page.text()),
        vec![74126, 74127, 74981, 74980]
    );
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='owner' WHERE id=74001")
        .await?;
    let owner_page = request
        .get("/households/persistence-fixture/admin/users")
        .await;
    let owner_html = owner_page.text();
    let owner_csrf = authenticity_token(&owner_html).ok_or("owner form token missing")?;
    let last_owner = request
        .post("/households/persistence-fixture/admin/users/74001/suspend")
        .form(&[
            ("confirmation", "suspended"),
            ("authenticity_token", owner_csrf),
        ])
        .await;
    assert_eq!(last_owner.status_code(), 422, "{}", last_owner.text());
    assert!(
        last_owner
            .text()
            .contains("Last active owner cannot be removed")
    );
    assert!(last_owner.text().contains("href=\"#administration-form\""));
    assert_eq!(
        last_owner
            .text()
            .matches("id=\"administration-form\"")
            .count(),
        1
    );
    assert!(
        last_owner
            .text()
            .contains("id=\"administration-form\" tabindex=\"-1\"")
    );
    let retained = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status FROM household_memberships WHERE id=74001",
        ))
        .await?
        .ok_or("owner missing")?;
    assert_eq!(retained.try_get::<String>("", "status")?, "active");
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await?;
    let stale_manager = request
        .post("/households/persistence-fixture/admin/users/74006/suspend")
        .form(&[
            ("confirmation", "suspended"),
            ("authenticity_token", owner_csrf),
        ])
        .await;
    assert_eq!(stale_manager.status_code(), 403, "{}", stale_manager.text());
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn member_administration_orders_and_filters_exact_memberships() -> TestResult {
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    fixture.admin.execute_unprepared("INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES (73201,'Hidden Alpha',0,true,now(),now()),(73202,'Hidden Bravo',0,true,now(),now()),(73203,'Hidden Charlie',0,true,now(),now()),(73204,'Hidden Delta',0,true,now(),now()); INSERT INTO accounts(id,person_id,name,email,password_hash,status,created_at,updated_at) VALUES (71201,73201,'Sortprobe Alpha','zeta-sortprobe@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()),(71202,73202,'Sortprobe Bravo','delta-sortprobe@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()),(71203,73203,'Sortprobe Charlie','alpha-sortprobe@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()),(71204,73204,'Sortprobe Delta','beta-sortprobe@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES (74201,72001,73201,'owner','active',now(),now(),now()),(74202,72001,73202,'administrator','suspended',now(),now(),now()),(74203,72001,73203,'member','revoked',now(),now(),now()),(74204,72001,73204,'member','active',now(),now(),now())").await?;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&request, &fixture, &boot).await?;
    let path = "/households/persistence-fixture/admin/users?q=sortprobe";
    for (field, ascending, descending) in [
        (
            "name",
            vec![74201, 74202, 74203, 74204],
            vec![74204, 74203, 74202, 74201],
        ),
        (
            "email",
            vec![74203, 74204, 74202, 74201],
            vec![74201, 74202, 74204, 74203],
        ),
        (
            "role",
            vec![74202, 74203, 74204, 74201],
            vec![74201, 74203, 74204, 74202],
        ),
        (
            "status",
            vec![74201, 74204, 74203, 74202],
            vec![74202, 74203, 74201, 74204],
        ),
    ] {
        for (direction, expected) in [("asc", ascending), ("desc", descending)] {
            let response = request
                .get(&format!("{path}&sort={field}&direction={direction}"))
                .await;
            assert_eq!(response.status_code(), 200, "{}", response.text());
            assert_eq!(
                listed_membership_ids(&response.text()),
                expected,
                "{field} {direction}"
            );
        }
    }
    for (filter, expected) in [
        ("role=member", vec![74203, 74204]),
        ("status=active", vec![74201, 74204]),
        ("status=suspended", vec![74202]),
        ("status=revoked", vec![74203]),
    ] {
        let response = request.get(&format!("{path}&{filter}")).await;
        assert_eq!(response.status_code(), 200, "{}", response.text());
        assert_eq!(
            listed_membership_ids(&response.text()),
            expected,
            "{filter}"
        );
    }
    let canonical_name = request
        .get("/households/persistence-fixture/admin/users?q=BRAVO")
        .await;
    assert_eq!(
        canonical_name.status_code(),
        200,
        "{}",
        canonical_name.text()
    );
    assert_eq!(listed_membership_ids(&canonical_name.text()), vec![74202]);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn administration_overview_shows_only_current_household_status() -> TestResult {
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    fixture.admin.execute_unprepared(
        "INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(87001,72001,'pending-admin@example.test','member','overview-pending',74001,now()+interval '7 days',now(),now()),(87002,72001,'expired-admin@example.test','member','overview-expired',74001,now()-interval '1 day',now(),now()); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71001,'Other household','other-household','UTC',now(),now()); INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73004,'Other person',0,true,now(),now()),(73005,'Foreign account person',0,true,now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,joined_at,created_at,updated_at) VALUES(74002,72002,73001,'owner',now(),now(),now()),(74004,72002,73004,'member',now(),now(),now()); INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(87003,72002,'other-admin@example.test','member','overview-other',74002,now()+interval '7 days',now(),now()); INSERT INTO accounts(id,person_id,email,password_hash,status,created_at,updated_at) VALUES(71002,73005,'foreign-admin@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72003,71002,'Foreign household','foreign-admin-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,joined_at,created_at,updated_at) VALUES(74003,72003,73005,'owner',now(),now(),now())"
    ).await?;
    let identity = fixture.admin.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT a.person_id,count(m.id) AS households FROM accounts a JOIN household_memberships m ON m.person_id=a.person_id WHERE a.id=71001 AND m.household_id IN(72001,72002) GROUP BY a.person_id",
    )).await?.ok_or("shared canonical Person fixture missing")?;
    let shared_person = identity.try_get::<i64>("", "person_id")?;
    let shared_households = identity.try_get::<i64>("", "households")?;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&request, &fixture, &boot).await?;
    let overview = request.get("/households/persistence-fixture/admin").await;
    let overview_status = overview.status_code();
    let overview_cache = overview.header("cache-control").to_owned();
    let html = overview.text();
    let selected_other = request.get("/households/other-household/admin").await;
    let other_status = selected_other.status_code();
    let other_html = selected_other.text();
    let foreign = request
        .get("/households/foreign-admin-household/admin")
        .await;
    let foreign_status = foreign.status_code().as_u16();
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await?;
    let denied = request.get("/households/persistence-fixture/admin").await;
    let denied_status = denied.status_code();
    fixture.admin.execute_unprepared("UPDATE household_memberships SET role='administrator',status='revoked',revoked_at=now() WHERE id=74001").await?;
    let revoked = request.get("/households/persistence-fixture/admin").await;
    let revoked_status = revoked.status_code().as_u16();
    drop(request);
    close(fixture, boot).await;
    assert_eq!(overview_status, 200, "{html}");
    assert_eq!(overview_cache, "no-store");
    assert!(html.contains("Synthetic household administration"));
    assert!(html.contains("3 active members"));
    assert!(html.contains("3 people"));
    assert!(html.contains("1 pending invitation"));
    assert!(!html.contains("Other household"));
    assert!(!html.contains("other-admin@example.test"));
    assert_eq!(other_status, 200, "{other_html}");
    assert!(other_html.contains("Other household administration"));
    assert!(other_html.contains("2 active members"));
    assert!(other_html.contains("2 people"));
    assert!(other_html.contains("1 pending invitation"));
    assert!(!other_html.contains("Synthetic household"));
    assert_eq!(shared_person, 73001);
    assert_eq!(shared_households, 2);
    assert!([403, 404].contains(&foreign_status));
    assert_eq!(denied_status, 403);
    assert!([403, 404].contains(&revoked_status));
    Ok(())
}

#[tokio::test]
async fn administration_overview_shows_five_recent_scoped_safe_events() -> TestResult {
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let mut request = server(&boot);
    request.save_cookies();
    let result = async {
        select_household(&request, &fixture, &boot).await?;
        fixture
            .admin
            .execute_unprepared("DELETE FROM security_audit_events WHERE household_id=72001")
            .await?;
        let empty = request.get("/households/persistence-fixture/admin").await;
        let empty_status = empty.status_code();
        let empty_html = empty.text();
        fixture.admin.execute_unprepared(
            "INSERT INTO security_audit_events(id,household_id,actor_account_id,actor_membership_id,event_type,request_id,metadata,audit_context,created_at,updated_at) SELECT 953000+number,72001,71001,74001,'household_membership.role_updated','recent-primary-' || number,'{\"target_membership_id\":74001,\"new_role\":\"administrator\",\"secret\":\"private-clinical-note\"}'::jsonb,'{\"session_reference\":\"private-session\"}'::jsonb,'2030-01-15 12:00:00','2030-01-15 12:00:00' FROM generate_series(1,6) AS number; INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71001,'Other household','other-household','UTC',now(),now()); INSERT INTO security_audit_events(id,household_id,event_type,request_id,metadata,audit_context,created_at,updated_at) VALUES(953007,72002,'household_membership.role_updated','recent-foreign','{\"secret\":\"foreign-clinical-note\"}'::jsonb,'{}'::jsonb,'2030-01-16 12:00:00','2030-01-16 12:00:00')"
        ).await?;
        let populated = request.get("/households/persistence-fixture/admin").await;
        let populated_status = populated.status_code();
        let populated_cache = populated.header("cache-control").to_owned();
        let populated_html = populated.text();
        Ok::<_, Box<dyn std::error::Error>>((empty_status, empty_html, populated_status, populated_cache, populated_html))
    }.await;
    drop(request);
    close(fixture, boot).await;
    let (empty_status, empty_html, populated_status, populated_cache, populated_html) = result?;
    assert_eq!(empty_status, 200, "{empty_html}");
    assert!(empty_html.contains("Recent activity"));
    assert!(empty_html.contains("No recent administrative activity"));
    assert_eq!(populated_status, 200, "{populated_html}");
    assert_eq!(populated_cache, "no-store");
    assert!(populated_html.contains("Recent activity"));
    assert_eq!(populated_html.matches("View audit event").count(), 5);
    assert!(populated_html.contains("/admin/audit_logs/953006"));
    assert!(!populated_html.contains("/admin/audit_logs/953001"));
    assert!(!populated_html.contains("/admin/audit_logs/953007"));
    assert!(populated_html.contains("Account #71001, membership #74001"));
    assert!(populated_html.contains("Household role changed"));
    assert!(!populated_html.contains("private-clinical-note"));
    assert!(!populated_html.contains("private-session"));
    assert!(!populated_html.contains("foreign-clinical-note"));
    Ok(())
}

#[tokio::test]
async fn navigation_exposes_platform_tools_only_while_platform_authority_is_active() -> TestResult {
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&request, &fixture, &boot).await?;
    let path = "/households/persistence-fixture/profile/navigation";
    let ordinary = request.get(path).await;
    let ordinary_status = ordinary.status_code();
    let ordinary_body = ordinary.text();
    fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71001,'active',now(),now())").await?;
    let elevated = request.get(path).await;
    let elevated_status = elevated.status_code();
    let elevated_body = elevated.text();
    fixture
        .admin
        .execute_unprepared("UPDATE platform_admins SET status='disabled' WHERE account_id=71001")
        .await?;
    let revoked = request.get(path).await;
    let revoked_status = revoked.status_code();
    let revoked_body = revoked.text();
    drop(request);
    close(fixture, boot).await;
    assert_eq!(ordinary_status, 200, "{ordinary_body}");
    assert!(!ordinary_body.contains("/platform/users"));
    assert_eq!(elevated_status, 200, "{elevated_body}");
    assert!(elevated_body.contains("/platform/users"));
    assert_eq!(revoked_status, 200, "{revoked_body}");
    assert!(!revoked_body.contains("/platform/users"));
    Ok(())
}

#[tokio::test]
async fn administration_overview_locales_preserve_language_and_household_navigation() -> TestResult
{
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&request, &fixture, &boot).await?;
    let mut results = Vec::new();
    for language in ["en", "cy", "ga", "pt", "es"] {
        let response = request
            .get(&format!(
                "/households/persistence-fixture/admin?lang={language}"
            ))
            .add_header("accept-language", "en")
            .await;
        results.push((language, response.status_code(), response.text()));
    }
    let header = request
        .get("/households/persistence-fixture/admin")
        .add_header("accept-language", "fr,es;q=0.9,en;q=0.1")
        .await
        .text();
    let invalid = request
        .get("/households/persistence-fixture/admin?lang=es&lang=en")
        .await
        .status_code();
    drop(request);
    close(fixture, boot).await;
    for (language, status, html) in results {
        assert_eq!(status, 200, "{language}: {html}");
        assert!(
            html.contains(&format!("<html lang=\"{language}\"")),
            "{language}: missing document language"
        );
        assert!(
            html.contains(&format!("/admin/users?lang={language}")),
            "{language}: member link lost language"
        );
        assert!(
            html.contains(&format!("/admin/invitations?lang={language}")),
            "{language}: invitation link lost language"
        );
        assert!(html.contains("Synthetic household"));
        assert!(
            html.contains("/admin/audit_logs"),
            "audit navigation is available in this delivery"
        );
    }
    assert!(header.contains("<html lang=\"es\""));
    assert_eq!(invalid, 422);
    Ok(())
}

#[tokio::test]
async fn administration_locale_selection_survives_shared_links_and_forms() -> TestResult {
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&request, &fixture, &boot).await?;
    let mut results = Vec::new();
    for language in ["en", "cy", "ga", "pt", "es"] {
        for path in [
            "users",
            "users/77001/edit",
            "household/edit",
            "carer_relationships",
            "carer_relationships/new",
            "invitations",
        ] {
            let response = request
                .get(&format!(
                    "/households/persistence-fixture/admin/{path}?lang={language}"
                ))
                .add_header("accept-language", "cy")
                .await;
            results.push((language, path, response.status_code(), response.text()));
        }
    }
    drop(request);
    close(fixture, boot).await;
    for (language, path, status, html) in results {
        assert_eq!(status, 200, "{language} {path}: {html}");
        assert!(
            html.contains(&format!("<html lang=\"{language}\"")),
            "{language} {path}"
        );
        for link in html
            .split("href=\"")
            .skip(1)
            .filter_map(|value| value.split('"').next())
        {
            if link.starts_with("/households/persistence-fixture/admin") {
                assert!(
                    link.ends_with(&format!("?lang={language}")),
                    "{language} {path}: {link}"
                );
            }
        }
        if html.contains("method=\"post\"") {
            assert!(
                html.contains(&format!("name=\"lang\" value=\"{language}\"")),
                "{language} {path}: form lost language"
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn administration_locale_invalid_links_return_safe_translated_html() -> TestResult {
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&request, &fixture, &boot).await?;
    let mut results = Vec::new();
    for (language, message) in [
        ("en", "This administration link is invalid."),
        ("cy", "ddolen weinyddu hon yn annilys."),
        ("ga", "Tá an nasc riaracháin seo neamhbhailí."),
        ("pt", "Esta ligação de administração é inválida."),
        ("es", "Este enlace de administración no es válido."),
    ] {
        for path in [
            "admin?lang=es&lang=en",
            "admin?lang=private-sentinel",
            "admin/invitations?lang=es&lang=en",
        ] {
            let response = request
                .get(&format!("/households/persistence-fixture/{path}"))
                .add_header("accept-language", language)
                .await;
            results.push((
                language,
                message,
                response.status_code(),
                response.header("content-type").to_owned(),
                response.text(),
            ));
        }
    }
    drop(request);
    close(fixture, boot).await;
    for (language, message, status, content_type, html) in results {
        assert_eq!(status, 422);
        assert!(content_type.to_str()?.starts_with("text/html"));
        assert!(html.contains(&format!("<html lang=\"{language}\"")));
        assert!(
            html.contains(message),
            "{language}: missing invalid-link recovery copy"
        );
        assert!(html.contains("class=\"btn btn-neutral mt-4\" href=\"/\""));
        assert!(!html.contains("private-sentinel"));
    }
    Ok(())
}
