use super::administration_overview::{authenticity_token, select_household};
use super::*;
use base64::Engine as _;

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn setup() -> (Fixture, BootResult, TestServer) {
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    fixture.admin.execute_unprepared("UPDATE household_memberships SET role='owner' WHERE id=74001; INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73061,'Bulk Alpha',0,true,now(),now()),(73062,'Bulk Beta',0,true,now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74061,72001,73061,'member','active',now(),now(),now()),(74062,72001,73062,'member','suspended',now(),now(),now())").await.unwrap();
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&request, &fixture, &boot).await.unwrap();
    (fixture, boot, request)
}

#[tokio::test]
async fn bulk_member_preview_confirms_named_targets_without_mutation() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let negotiated = request
        .get("/households/persistence-fixture/admin/users?status=active")
        .add_header("accept-language", "cy")
        .await;
    let negotiated_html = negotiated.text();
    let bulk_form = negotiated_html
        .split("id=\"bulk-member-selection\"")
        .nth(1)
        .ok_or("bulk form missing")?
        .split("</form>")
        .next()
        .ok_or("form end missing")?;
    assert!(
        bulk_form.contains("name=\"lang\" value=\"cy\""),
        "bulk action must carry negotiated language"
    );

    let listing = request
        .get("/households/persistence-fixture/admin/users?status=active&lang=en")
        .await;
    let html = listing.text();
    let token = authenticity_token(&html).ok_or("csrf missing")?;
    let preview = request
        .post("/households/persistence-fixture/admin/users/bulk/preview")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("authenticity_token", token),
            ("action", "suspend"),
            ("membership_ids", "74061"),
            ("membership_ids", "74062"),
            ("status", "active"),
            ("lang", "en"),
        ])
        .await;
    assert_eq!(preview.status_code(), 200, "{}", preview.text());
    assert!(preview.text().contains("Bulk Alpha"));
    assert!(preview.text().contains("Bulk Beta"));
    assert!(preview.text().contains("Confirm Suspend"));
    let row = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status FROM household_memberships WHERE id=74061",
        ))
        .await?
        .ok_or("state missing")?;
    assert_eq!(row.try_get::<String>("", "status")?, "active");
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

async fn token(request: &TestServer) -> String {
    let page = request
        .get("/households/persistence-fixture/admin/users?lang=en")
        .await;
    authenticity_token(&page.text()).unwrap().to_string()
}
struct Reply {
    status: u16,
    body: String,
}
impl Reply {
    fn status_code(&self) -> u16 {
        self.status
    }
    fn text(&self) -> &str {
        &self.body
    }
}
async fn submit(request: &TestServer, token: &str, ids: &[&str], action: &str) -> Reply {
    let mut fields = vec![
        ("authenticity_token".to_string(), token.to_string()),
        ("action".into(), action.into()),
        ("confirmation".into(), action.into()),
        ("q".into(), "Bulk".into()),
        ("status".into(), "active".into()),
        ("sort".into(), "name".into()),
        ("direction".into(), "desc".into()),
        ("page".into(), "2".into()),
        ("lang".into(), "en".into()),
    ];
    fields.extend(
        ids.iter()
            .map(|id| ("membership_ids".into(), (*id).to_string())),
    );
    let response = request
        .post("/households/persistence-fixture/admin/users/bulk")
        .add_header("sec-fetch-site", "same-origin")
        .form(&fields)
        .await;
    Reply {
        status: response.status_code().as_u16(),
        body: response.text(),
    }
}
async fn status(fixture: &Fixture, id: i64) -> String {
    fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status FROM household_memberships WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "status")
        .unwrap()
}
async fn audit_count(fixture: &Fixture, outcome: &str) -> i64 {
    fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT count(*) AS count FROM security_audit_events WHERE event_type='household_access.membership_changed' AND metadata->>'outcome'=$1",[outcome.into()])).await.unwrap().unwrap().try_get("","count").unwrap()
}
#[tokio::test]
async fn bulk_member_partial_success_retains_failed_targets_and_context() -> TestResult {
    let (fixture, boot, request) = setup().await;
    fixture.admin.execute_unprepared("INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71001,'Foreign','foreign-bulk','UTC',now(),now()); INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73063,'Foreign secret identity',0,true,now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74063,72002,73063,'member','active',now(),now(),now())").await?;
    let response = submit(
        &request,
        &token(&request).await,
        &["74061", "74062", "74063", "99999"],
        "suspend",
    )
    .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    let html = response.text();
    assert!(html.contains("Household access changed."));
    assert!(html.contains("Membership state changed"));
    assert!(!html.contains("Foreign secret identity"));
    assert!(html.contains("value=\"74062\" checked"));
    assert!(!html.contains("value=\"74061\" checked"));
    assert!(html.contains("q=Bulk&amp;role=all&amp;status=active"));
    assert!(html.contains("direction=desc"));
    assert!(html.contains("page=2"));
    assert!(html.contains("lang=en"));
    assert_eq!(status(&fixture, 74061).await, "suspended");
    assert_eq!(status(&fixture, 74062).await, "suspended");
    assert_eq!(status(&fixture, 74063).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 1);
    assert_eq!(audit_count(&fixture, "rejected").await, 1);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_member_invalid_selection_and_csrf_never_mutate() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let csrf = token(&request).await;
    for ids in [
        vec![],
        vec!["74061", "74061"],
        vec!["74061", "bad"],
        vec!["0"],
        vec!["-1"],
        vec!["74061"; 26],
    ] {
        let response = submit(&request, &csrf, &ids, "suspend").await;
        assert_eq!(response.status_code(), 422, "{}", response.text());
    }
    assert_eq!(
        submit(&request, &csrf, &["74061"], "restore")
            .await
            .status_code(),
        422
    );
    assert_eq!(
        submit(&request, "invalid", &["74061"], "suspend")
            .await
            .status_code(),
        403
    );
    assert_eq!(status(&fixture, 74061).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 0);
    assert_eq!(audit_count(&fixture, "rejected").await, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_member_authority_loss_stops_remaining_after_committed_success() -> TestResult {
    let (fixture, boot, request) = setup().await;
    fixture.admin.execute_unprepared("UPDATE household_memberships SET status='active' WHERE id=74062; CREATE FUNCTION bulk_remove_actor() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$ BEGIN IF NEW.event_type='household_access.membership_changed' AND NEW.metadata->>'target_membership_id'='74061' THEN UPDATE public.household_memberships SET status='revoked',permissions_version=permissions_version+1 WHERE id=74001; END IF; RETURN NEW; END $$; CREATE TRIGGER bulk_remove_actor AFTER INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION bulk_remove_actor()").await?;
    let response = submit(
        &request,
        &token(&request).await,
        &["74061", "74062"],
        "suspend",
    )
    .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    assert!(response.text().contains("Household access changed."));
    assert!(
        response
            .text()
            .contains("Your management authority is no longer available")
    );
    assert!(!response.text().contains("Back to members"));
    assert!(!response.text().contains("Review selected failures again"));
    assert_eq!(status(&fixture, 74061).await, "suspended");
    assert_eq!(status(&fixture, 74062).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 1);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_member_audit_failure_rolls_back_without_claiming_success() -> TestResult {
    let (fixture, boot, request) = setup().await;
    fixture.admin.execute_unprepared("UPDATE household_memberships SET status='active' WHERE id=74062; CREATE FUNCTION bulk_reject_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='household_access.membership_changed' THEN RAISE EXCEPTION 'Synthetic bulk audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER bulk_reject_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION bulk_reject_audit()").await?;
    let response = submit(
        &request,
        &token(&request).await,
        &["74061", "74062"],
        "revoke",
    )
    .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    assert!(
        response
            .text()
            .contains("The result could not be confirmed")
    );
    assert!(!response.text().contains("Household access changed."));
    assert_eq!(status(&fixture, 74061).await, "active");
    assert_eq!(status(&fixture, 74062).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_member_commit_failure_is_unconfirmed_and_rolls_back() -> TestResult {
    let (fixture, boot, request) = setup().await;
    fixture.admin.execute_unprepared("UPDATE household_memberships SET status='active' WHERE id=74062; INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73064,'Bulk Gamma',0,true,now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74064,72001,73064,'member','active',now(),now(),now()); CREATE FUNCTION bulk_reject_commit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='household_access.membership_changed' AND NEW.metadata->>'target_membership_id'='74062' THEN RAISE EXCEPTION 'Synthetic bulk commit rejection'; END IF; RETURN NEW; END $$; CREATE CONSTRAINT TRIGGER bulk_reject_commit AFTER INSERT ON security_audit_events DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION bulk_reject_commit()").await?;
    let response = submit(
        &request,
        &token(&request).await,
        &["74061", "74062", "74064"],
        "revoke",
    )
    .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    assert!(
        response
            .text()
            .contains("The result could not be confirmed")
    );
    assert!(
        response
            .text()
            .contains("Not attempted because processing stopped")
    );
    assert_eq!(
        response.text().matches("Household access changed.").count(),
        1
    );
    assert!(!response.text().contains("Synthetic bulk"));
    assert_eq!(status(&fixture, 74061).await, "revoked");
    assert_eq!(status(&fixture, 74062).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 1);
    assert_eq!(status(&fixture, 74064).await, "active");
    assert!(response.text().contains("value=\"74062\" checked"));
    assert!(response.text().contains("value=\"74064\" checked"));
    assert!(!response.text().contains("value=\"74061\" checked"));
    drop(request);
    close(fixture, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_member_actor_is_last_and_self_removal_renders_no_directory() -> TestResult {
    let (fixture, boot, request) = setup().await;
    fixture
        .admin
        .execute_unprepared(
            "UPDATE household_memberships SET role='owner',status='active' WHERE id=74062",
        )
        .await?;
    let response = submit(
        &request,
        &token(&request).await,
        &["74001", "74061"],
        "revoke",
    )
    .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    assert_eq!(status(&fixture, 74061).await, "revoked");
    assert_eq!(status(&fixture, 74001).await, "revoked");
    assert_eq!(status(&fixture, 74062).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 2);
    assert!(!response.text().contains("Back to members"));
    assert!(!response.text().contains("Review selected failures again"));
    drop(request);
    close(fixture, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_member_concurrent_batches_preserve_last_owner_and_rejection_audit() -> TestResult {
    let (fixture, boot, request) = setup().await;
    fixture
        .admin
        .execute_unprepared(
            "UPDATE household_memberships SET role='owner',status='active' WHERE id=74062",
        )
        .await?;
    let csrf = token(&request).await;
    let (first, second) = tokio::join!(
        submit(&request, &csrf, &["74001", "74062"], "suspend"),
        submit(&request, &csrf, &["74001", "74062"], "revoke")
    );
    assert_eq!(first.status_code(), 200, "{}", first.text());
    assert_eq!(second.status_code(), 200, "{}", second.text());
    assert_eq!(status(&fixture, 74001).await, "active");
    assert_ne!(status(&fixture, 74062).await, "active");
    assert!(audit_count(&fixture, "rejected").await >= 2);
    let owners=fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS count FROM household_memberships WHERE household_id=72001 AND role='owner' AND status='active'")).await?.ok_or("missing owner count")?;
    assert_eq!(owners.try_get::<i64>("", "count")?, 1);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn bulk_member_all_success_has_no_empty_retry_and_preserves_escaped_identity() -> TestResult {
    let (fixture, boot, request) = setup().await;
    fixture
        .admin
        .execute_unprepared("UPDATE people SET name='<script>private-name</script>' WHERE id=73061")
        .await?;
    let response = submit(
        &request,
        &token(&request).await,
        &["74061", "74062"],
        "revoke",
    )
    .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    assert!(
        response
            .text()
            .contains("&lt;script&gt;private-name&lt;/script&gt;")
    );
    assert!(!response.text().contains("<script>private-name</script>"));
    assert!(!response.text().contains("Review selected failures again"));
    assert_eq!(status(&fixture, 74061).await, "revoked");
    assert_eq!(status(&fixture, 74062).await, "revoked");
    assert_eq!(audit_count(&fixture, "success").await, 2);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_member_rejects_large_unique_selection_and_missing_confirmation() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let csrf = token(&request).await;
    let owned = (1..=26)
        .map(|id| (74060 + id).to_string())
        .collect::<Vec<_>>();
    let ids = owned.iter().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(
        submit(&request, &csrf, &ids, "suspend").await.status_code(),
        422
    );
    let response = request
        .post("/households/persistence-fixture/admin/users/bulk")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("authenticity_token", csrf.as_str()),
            ("action", "suspend"),
            ("membership_ids", "74061"),
        ])
        .await;
    assert_eq!(response.status_code(), 422);
    assert_eq!(status(&fixture, 74061).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 0);
    assert_eq!(audit_count(&fixture, "rejected").await, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_member_denies_ordinary_revoked_locked_and_inactive_household_actors() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let csrf = token(&request).await;
    for change in [
        "UPDATE household_memberships SET role='member' WHERE id=74001",
        "UPDATE household_memberships SET role='owner',status='revoked',revoked_at=now() WHERE id=74001",
        "UPDATE household_memberships SET status='active',revoked_at=NULL WHERE id=74001; INSERT INTO account_lockouts(account_id,key,deadline) VALUES(71001,'bulk-lock',now()+interval '1 hour')",
        "DELETE FROM account_lockouts WHERE account_id=71001; UPDATE households SET lifecycle_state='deletion_requested' WHERE id=72001",
    ] {
        fixture.admin.execute_unprepared(change).await?;
        assert_eq!(
            submit(&request, &csrf, &["74061"], "suspend")
                .await
                .status_code(),
            403
        );
        assert_eq!(status(&fixture, 74061).await, "active");
    }
    assert_eq!(audit_count(&fixture, "success").await, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}
