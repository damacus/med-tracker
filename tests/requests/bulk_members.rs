use super::administration_overview::{authenticity_token, select_household};
use super::*;
use base64::Engine as _;
use med_tracker::models::entities::{
    account_lockout, household, membership, person, security_audit_event,
};
use sea_orm::sea_query::{Expr, ExprTrait};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn setup() -> (Fixture, BootResult, TestServer) {
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    membership::Entity::update_many()
        .col_expr(membership::Column::Role, Expr::value("owner"))
        .filter(membership::Column::Id.eq(74001))
        .exec(&fixture.admin)
        .await
        .unwrap();
    add_member(&fixture, 73061, 74061, "Bulk Alpha", "active")
        .await
        .unwrap();
    add_member(&fixture, 73062, 74062, "Bulk Beta", "suspended")
        .await
        .unwrap();
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&request, &fixture, &boot).await.unwrap();
    (fixture, boot, request)
}

async fn add_member(
    fixture: &Fixture,
    person_id: i64,
    id: i64,
    name: &str,
    status: &str,
) -> TestResult {
    let now = chrono::Utc::now().naive_utc();
    person::ActiveModel {
        id: Set(person_id),
        name: Set(name.into()),
        person_type: Set(0),
        has_capacity: Set(true),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    membership::ActiveModel {
        id: Set(id),
        household_id: Set(72001),
        person_id: Set(person_id),
        role: Set("member".into()),
        status: Set(status.into()),
        permissions_version: Set(1),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    Ok(())
}
fn field(html: &str, name: &str) -> String {
    let document = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse(&format!("input[name='{name}']")).unwrap();
    document
        .select(&selector)
        .next()
        .and_then(|input| input.value().attr("value"))
        .unwrap()
        .to_owned()
}
async fn review(request: &TestServer, csrf: &str, ids: &[&str], action: &str) -> Reply {
    review_context(request, csrf, ids, action, "Bulk", None).await
}
async fn review_context(
    request: &TestServer,
    csrf: &str,
    ids: &[&str],
    action: &str,
    query: &str,
    review_id: Option<&str>,
) -> Reply {
    let mut fields = vec![
        ("authenticity_token".to_owned(), csrf.to_owned()),
        ("action".into(), action.into()),
        ("q".into(), query.into()),
        ("status".into(), "all".into()),
        ("sort".into(), "name".into()),
        ("direction".into(), "desc".into()),
        ("page".into(), "1".into()),
        ("lang".into(), "en".into()),
    ];
    if let Some(id) = review_id {
        fields.push(("review_id".into(), id.into()));
    }
    fields.extend(
        ids.iter()
            .map(|id| ("membership_ids".into(), (*id).to_owned())),
    );
    let response = request
        .post("/households/persistence-fixture/admin/users/bulk/preview")
        .add_header("sec-fetch-site", "same-origin")
        .form(&fields)
        .await;
    Reply {
        status: response.status_code().as_u16(),
        body: response.text(),
    }
}
async fn apply_review(
    request: &TestServer,
    csrf: &str,
    ids: &[&str],
    action: &str,
    review_id: &str,
) -> Reply {
    apply_context(request, csrf, ids, action, review_id, "Bulk").await
}
async fn apply_context(
    request: &TestServer,
    csrf: &str,
    ids: &[&str],
    action: &str,
    review_id: &str,
    query: &str,
) -> Reply {
    let mut fields = vec![
        ("authenticity_token".to_string(), csrf.to_string()),
        ("action".into(), action.into()),
        ("confirmation".into(), action.into()),
        ("review_id".into(), review_id.into()),
        ("q".into(), query.into()),
        ("status".into(), "all".into()),
        ("sort".into(), "name".into()),
        ("direction".into(), "desc".into()),
        ("page".into(), "1".into()),
        ("lang".into(), "en".into()),
    ];
    fields.extend(
        ids.iter()
            .map(|id| ("membership_ids".into(), (*id).to_owned())),
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
        .get("/households/persistence-fixture/admin/users?status=all&lang=en")
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
            ("status", "all"),
            ("lang", "en"),
        ])
        .await;
    assert_eq!(preview.status_code(), 200, "{}", preview.text());
    assert!(preview.text().contains("Bulk Alpha"));
    assert!(preview.text().contains("Bulk Beta"));
    assert!(preview.text().contains("Confirm Suspend"));
    assert_eq!(status(&fixture, 74061).await, "active");
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
async fn submit(request: &TestServer, csrf: &str, ids: &[&str], action: &str) -> Reply {
    let reviewed = review(request, csrf, ids, action).await;
    if reviewed.status_code() != 200 {
        return reviewed;
    }
    apply_review(
        request,
        csrf,
        ids,
        action,
        &field(reviewed.text(), "review_id"),
    )
    .await
}
async fn status(fixture: &Fixture, id: i64) -> String {
    membership::Entity::find_by_id(id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .status
}
async fn audit_count(fixture: &Fixture, outcome: &str) -> u64 {
    security_audit_event::Entity::find()
        .filter(security_audit_event::Column::HouseholdId.eq(72001))
        .filter(security_audit_event::Column::EventType.eq("household_access.membership_changed"))
        .filter(
            sea_orm::sea_query::extension::postgres::PgExpr::cast_json_field(
                Expr::col(security_audit_event::Column::Metadata),
                "outcome",
            )
            .eq(outcome),
        )
        .count(&fixture.admin)
        .await
        .unwrap()
}
#[tokio::test]
async fn bulk_member_partial_success_retains_failed_targets_and_context() -> TestResult {
    let (fixture, boot, request) = setup().await;
    add_member(&fixture, 73063, 74063, "Bulk Foreign", "active").await?;
    add_member(&fixture, 73064, 74064, "Bulk Missing", "active").await?;
    let csrf = token(&request).await;
    let reviewed = review(
        &request,
        &csrf,
        &["74061", "74062", "74063", "74064"],
        "suspend",
    )
    .await;
    assert_eq!(reviewed.status_code(), 200, "{}", reviewed.text());
    let now = chrono::Utc::now().naive_utc();
    household::ActiveModel {
        id: Set(72002),
        created_by_account_id: Set(Some(71001)),
        name: Set("Foreign".into()),
        slug: Set("foreign-bulk".into()),
        timezone: Set("UTC".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    membership::Entity::update_many()
        .col_expr(membership::Column::HouseholdId, Expr::value(72002))
        .filter(membership::Column::Id.eq(74063))
        .exec(&fixture.admin)
        .await?;
    person::Entity::update_many()
        .col_expr(person::Column::Name, Expr::value("Foreign secret identity"))
        .filter(person::Column::Id.eq(73063))
        .exec(&fixture.admin)
        .await?;
    membership::Entity::delete_by_id(74064)
        .exec(&fixture.admin)
        .await?;
    let response = apply_review(
        &request,
        &csrf,
        &["74061", "74062", "74063", "74064"],
        "suspend",
        &field(reviewed.text(), "review_id"),
    )
    .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    let html = response.text();
    assert!(html.contains("Household access changed."));
    assert!(html.contains("Membership state changed"));
    assert!(!html.contains("Foreign secret identity"));
    assert!(html.contains("value=\"74062\" checked"));
    assert!(!html.contains("value=\"74061\" checked"));
    assert!(html.contains("q=Bulk&amp;role=all&amp;status=all"));
    assert!(html.contains("direction=desc"));
    assert!(html.contains("page=1"));
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
// SeaORM 2.0.4 Schema and SeaQuery 1.0.2 SchemaStatement expose no CREATE FUNCTION,
// CREATE TRIGGER or deferred constraint-trigger builder. The next three fixtures use
// isolated synthetic PostgreSQL DDL to inject authority, audit and commit failures;
// ordinary fixture CRUD and assertions use entities/builders.
#[tokio::test]
async fn bulk_member_authority_loss_stops_remaining_after_committed_success() -> TestResult {
    let (fixture, boot, request) = setup().await;
    membership::Entity::update_many()
        .col_expr(membership::Column::Status, Expr::value("active"))
        .filter(membership::Column::Id.eq(74062))
        .exec(&fixture.admin)
        .await?;
    fixture.admin.execute_unprepared(" CREATE FUNCTION bulk_remove_actor() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$ BEGIN IF NEW.event_type='household_access.membership_changed' AND NEW.metadata->>'target_membership_id'='74061' THEN UPDATE public.household_memberships SET status='revoked',permissions_version=permissions_version+1 WHERE id=74001; END IF; RETURN NEW; END $$; CREATE TRIGGER bulk_remove_actor AFTER INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION bulk_remove_actor()").await?;
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
    membership::Entity::update_many()
        .col_expr(membership::Column::Status, Expr::value("active"))
        .filter(membership::Column::Id.eq(74062))
        .exec(&fixture.admin)
        .await?;
    fixture.admin.execute_unprepared(" CREATE FUNCTION bulk_reject_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='household_access.membership_changed' THEN RAISE EXCEPTION 'Synthetic bulk audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER bulk_reject_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION bulk_reject_audit()").await?;
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
    membership::Entity::update_many()
        .col_expr(membership::Column::Status, Expr::value("active"))
        .filter(membership::Column::Id.eq(74062))
        .exec(&fixture.admin)
        .await?;
    add_member(&fixture, 73064, 74064, "Bulk Gamma", "active").await?;
    fixture.admin.execute_unprepared(" CREATE FUNCTION bulk_reject_commit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='household_access.membership_changed' AND NEW.metadata->>'target_membership_id'='74062' THEN RAISE EXCEPTION 'Synthetic bulk commit rejection'; END IF; RETURN NEW; END $$; CREATE CONSTRAINT TRIGGER bulk_reject_commit AFTER INSERT ON security_audit_events DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION bulk_reject_commit()").await?;
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
    membership::Entity::update_many()
        .col_expr(membership::Column::Role, Expr::value("owner"))
        .col_expr(membership::Column::Status, Expr::value("active"))
        .filter(membership::Column::Id.eq(74062))
        .exec(&fixture.admin)
        .await?;
    let csrf = token(&request).await;
    let reviewed = review_context(&request, &csrf, &["74001", "74061"], "revoke", "", None).await;
    assert_eq!(reviewed.status_code(), 200, "{}", reviewed.text());
    let response = apply_context(
        &request,
        &csrf,
        &["74001", "74061"],
        "revoke",
        &field(reviewed.text(), "review_id"),
        "",
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
    membership::Entity::update_many()
        .col_expr(membership::Column::Role, Expr::value("owner"))
        .col_expr(membership::Column::Status, Expr::value("active"))
        .filter(membership::Column::Id.eq(74062))
        .exec(&fixture.admin)
        .await?;
    let mut second_request = server(&boot);
    second_request.save_cookies();
    select_household(&second_request, &fixture, &boot).await?;
    let csrf = token(&request).await;
    let second_csrf = token(&second_request).await;
    let first_review =
        review_context(&request, &csrf, &["74001", "74062"], "suspend", "", None).await;
    let second_review = review_context(
        &second_request,
        &second_csrf,
        &["74001", "74062"],
        "revoke",
        "",
        None,
    )
    .await;
    assert_eq!(first_review.status_code(), 200, "{}", first_review.text());
    assert_eq!(second_review.status_code(), 200, "{}", second_review.text());
    let first_id = field(first_review.text(), "review_id");
    let second_id = field(second_review.text(), "review_id");
    let (first, second) = tokio::join!(
        apply_context(
            &request,
            &csrf,
            &["74001", "74062"],
            "suspend",
            &first_id,
            ""
        ),
        apply_context(
            &second_request,
            &second_csrf,
            &["74001", "74062"],
            "revoke",
            &second_id,
            ""
        )
    );
    assert_eq!(first.status_code(), 200, "{}", first.text());
    assert_eq!(second.status_code(), 200, "{}", second.text());
    assert_eq!(status(&fixture, 74001).await, "active");
    assert_ne!(status(&fixture, 74062).await, "active");
    assert!(audit_count(&fixture, "rejected").await >= 2);
    let owners = membership::Entity::find()
        .filter(membership::Column::HouseholdId.eq(72001))
        .filter(membership::Column::Role.eq("owner"))
        .filter(membership::Column::Status.eq("active"))
        .count(&fixture.admin)
        .await?;
    assert_eq!(owners, 1);
    drop(second_request);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn bulk_member_all_success_has_no_empty_retry_and_preserves_escaped_identity() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let csrf = token(&request).await;
    let reviewed = review(&request, &csrf, &["74061", "74062"], "revoke").await;
    assert_eq!(reviewed.status_code(), 200);
    person::Entity::update_many()
        .col_expr(
            person::Column::Name,
            Expr::value("<script>private-name</script>"),
        )
        .filter(person::Column::Id.eq(73061))
        .exec(&fixture.admin)
        .await?;
    let response = apply_review(
        &request,
        &csrf,
        &["74061", "74062"],
        "revoke",
        &field(reviewed.text(), "review_id"),
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
    for scenario in 0..4 {
        match scenario {
            0 => {
                membership::Entity::update_many()
                    .col_expr(membership::Column::Role, Expr::value("member"))
                    .filter(membership::Column::Id.eq(74001))
                    .exec(&fixture.admin)
                    .await?;
            }
            1 => {
                membership::Entity::update_many()
                    .col_expr(membership::Column::Role, Expr::value("owner"))
                    .col_expr(membership::Column::Status, Expr::value("revoked"))
                    .col_expr(
                        membership::Column::RevokedAt,
                        Expr::value(chrono::Utc::now().naive_utc()),
                    )
                    .filter(membership::Column::Id.eq(74001))
                    .exec(&fixture.admin)
                    .await?;
            }
            2 => {
                membership::Entity::update_many()
                    .col_expr(membership::Column::Status, Expr::value("active"))
                    .col_expr(
                        membership::Column::RevokedAt,
                        Expr::value(None::<chrono::NaiveDateTime>),
                    )
                    .filter(membership::Column::Id.eq(74001))
                    .exec(&fixture.admin)
                    .await?;
                account_lockout::ActiveModel {
                    account_id: Set(71001),
                    key: Set("bulk-lock".into()),
                    deadline: Set((chrono::Utc::now() + chrono::Duration::hours(1)).naive_utc()),
                    ..Default::default()
                }
                .insert(&fixture.admin)
                .await?;
            }
            _ => {
                account_lockout::Entity::delete_by_id(71001)
                    .exec(&fixture.admin)
                    .await?;
                household::Entity::update_many()
                    .col_expr(
                        household::Column::LifecycleState,
                        Expr::value("deletion_requested"),
                    )
                    .filter(household::Column::Id.eq(72001))
                    .exec(&fixture.admin)
                    .await?;
            }
        }
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

#[tokio::test]
async fn bulk_member_anonymous_malformed_post_uses_generic_navigation_without_effects() -> TestResult
{
    let (fixture, boot, request) = setup().await;
    let anonymous = server(&boot);
    for endpoint in ["preview", ""] {
        let path = if endpoint.is_empty() {
            "/households/fabricated-bulk-household/admin/users/bulk".to_string()
        } else {
            format!("/households/fabricated-bulk-household/admin/users/bulk/{endpoint}")
        };
        let response = anonymous
            .post(&path)
            .add_header("sec-fetch-site", "same-origin")
            .form(&[
                ("action", "suspend"),
                ("q", "fabricated-search"),
                ("lang", "en"),
            ])
            .await;
        assert_eq!(response.status_code(), 422, "{}", response.text());
        let html = response.text();
        assert!(
            !html.contains("fabricated-bulk-household"),
            "anonymous error must not invent household navigation"
        );
        assert!(!html.contains("class=\"app-household\""));
        assert!(!html.contains("data-sidebar-profile"));
        assert!(!html.contains("/admin/users?"));
        assert!(!html.contains("/people"));
        assert_eq!(status(&fixture, 74061).await, "active");
        assert_eq!(status(&fixture, 74062).await, "suspended");
        assert_eq!(audit_count(&fixture, "success").await, 0);
        assert_eq!(audit_count(&fixture, "rejected").await, 0);
    }
    drop(anonymous);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn bulk_review_rejects_filtered_out_initial_target_without_effects() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let csrf = token(&request).await;
    let response = request
        .post("/households/persistence-fixture/admin/users/bulk/preview")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("authenticity_token", csrf.as_str()),
            ("action", "suspend"),
            ("membership_ids", "74062"),
            ("q", "Bulk Alpha"),
            ("lang", "en"),
        ])
        .await;
    assert_eq!(
        response.status_code(),
        422,
        "filtered-out target must not be reviewable"
    );
    assert_eq!(status(&fixture, 74061).await, "active");
    assert_eq!(status(&fixture, 74062).await, "suspended");
    assert_eq!(audit_count(&fixture, "success").await, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_review_requires_session_confirmation_before_mutation() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let response = apply_review(
        &request,
        &token(&request).await,
        &["74061"],
        "suspend",
        &uuid::Uuid::new_v4().to_string(),
    )
    .await;
    assert_eq!(
        response.status_code(),
        422,
        "direct apply without a reviewed session must fail"
    );
    assert_eq!(status(&fixture, 74061).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn bulk_review_rejects_off_page_selection_without_effects() -> TestResult {
    let (fixture, boot, request) = setup().await;
    for index in 0..25 {
        add_member(
            &fixture,
            73100 + index,
            74100 + index,
            &format!("Bulk Z{index:02}"),
            "active",
        )
        .await?;
    }
    let csrf = token(&request).await;
    let response = review(&request, &csrf, &["74061"], "revoke").await;
    assert_eq!(response.status_code(), 422, "{}", response.text());
    assert_eq!(status(&fixture, 74061).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn bulk_review_binds_session_targets_action_and_context_and_consumes_confirmation()
-> TestResult {
    let (fixture, boot, request) = setup().await;
    let csrf = token(&request).await;
    for (ids, action, query) in [
        (vec!["74061", "74062"], "revoke", "Bulk"),
        (vec!["74061"], "suspend", "Bulk"),
        (vec!["74061"], "revoke", ""),
    ] {
        let reviewed = review(&request, &csrf, &["74061"], "revoke").await;
        assert_eq!(reviewed.status_code(), 200, "{}", reviewed.text());
        let response = apply_context(
            &request,
            &csrf,
            &ids,
            action,
            &field(reviewed.text(), "review_id"),
            query,
        )
        .await;
        assert_eq!(response.status_code(), 422, "{}", response.text());
        assert_eq!(status(&fixture, 74061).await, "active");
    }
    let reviewed = review(&request, &csrf, &["74061"], "revoke").await;
    let id = field(reviewed.text(), "review_id");
    let mut other = server(&boot);
    other.save_cookies();
    select_household(&other, &fixture, &boot).await?;
    let other_csrf = token(&other).await;
    let foreign = apply_review(&other, &other_csrf, &["74061"], "revoke", &id).await;
    assert_eq!(foreign.status_code(), 422, "{}", foreign.text());
    assert_eq!(status(&fixture, 74061).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 0);
    let success = apply_review(&request, &csrf, &["74061"], "revoke", &id).await;
    assert_eq!(success.status_code(), 200, "{}", success.text());
    let replay = apply_review(&request, &csrf, &["74061"], "revoke", &id).await;
    assert_eq!(replay.status_code(), 422, "{}", replay.text());
    assert_eq!(status(&fixture, 74061).await, "revoked");
    assert_eq!(audit_count(&fixture, "success").await, 1);
    assert_eq!(audit_count(&fixture, "rejected").await, 0);
    drop(other);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn bulk_review_retry_is_failed_only_despite_shifted_page() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let csrf = token(&request).await;
    let reviewed = review(&request, &csrf, &["74061", "74062"], "suspend").await;
    assert_eq!(reviewed.status_code(), 200, "{}", reviewed.text());
    let results = apply_review(
        &request,
        &csrf,
        &["74061", "74062"],
        "suspend",
        &field(reviewed.text(), "review_id"),
    )
    .await;
    assert_eq!(results.status_code(), 200, "{}", results.text());
    let retry_id = field(results.text(), "review_id");
    for index in 0..25 {
        add_member(
            &fixture,
            73100 + index,
            74100 + index,
            &format!("Bulk Z{index:02}"),
            "active",
        )
        .await?;
    }
    let retry = review_context(
        &request,
        &csrf,
        &["74062"],
        "suspend",
        "Bulk",
        Some(&retry_id),
    )
    .await;
    assert_eq!(retry.status_code(), 200, "{}", retry.text());
    membership::Entity::update_many()
        .col_expr(membership::Column::Status, Expr::value("active"))
        .filter(membership::Column::Id.eq(74062))
        .exec(&fixture.admin)
        .await?;
    let retried = apply_review(
        &request,
        &csrf,
        &["74062"],
        "suspend",
        &field(retry.text(), "review_id"),
    )
    .await;
    assert_eq!(retried.status_code(), 200, "{}", retried.text());
    assert_eq!(status(&fixture, 74062).await, "suspended");
    assert_eq!(audit_count(&fixture, "success").await, 2);
    assert_eq!(audit_count(&fixture, "rejected").await, 1);
    assert!(!retried.text().contains("Review selected failures again"));
    let reviewed = review(&request, &csrf, &["74124"], "suspend").await;
    assert_eq!(reviewed.status_code(), 200);
    membership::Entity::update_many()
        .col_expr(membership::Column::Status, Expr::value("suspended"))
        .filter(membership::Column::Id.eq(74124))
        .exec(&fixture.admin)
        .await?;
    let results = apply_review(
        &request,
        &csrf,
        &["74124"],
        "suspend",
        &field(reviewed.text(), "review_id"),
    )
    .await;
    let denied = review_context(
        &request,
        &csrf,
        &["74061", "74124"],
        "suspend",
        "Bulk",
        Some(&field(results.text(), "review_id")),
    )
    .await;
    assert_eq!(denied.status_code(), 422, "{}", denied.text());
    assert_eq!(audit_count(&fixture, "success").await, 2);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn bulk_review_return_to_members_cancels_pending_confirmation() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let csrf = token(&request).await;
    let reviewed = review(&request, &csrf, &["74061"], "revoke").await;
    assert_eq!(reviewed.status_code(), 200);
    request
        .get("/households/persistence-fixture/admin/users?status=all&lang=en")
        .await;
    let cancelled = apply_review(
        &request,
        &csrf,
        &["74061"],
        "revoke",
        &field(reviewed.text(), "review_id"),
    )
    .await;
    assert_eq!(cancelled.status_code(), 422, "{}", cancelled.text());
    assert_eq!(status(&fixture, 74061).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}
