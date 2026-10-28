use super::administration_overview::authenticity_token;
use super::*;
use base64::Engine as _;
use med_tracker::models::entities::{
    account_lockout, household, membership, person, security_audit_event,
};
use sea_orm::sea_query::Expr;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct BulkFixture {
    inner: RequestFixture,
    alpha: membership::Model,
    beta: membership::Model,
}
impl std::ops::Deref for BulkFixture {
    type Target = RequestFixture;
    fn deref(&self) -> &RequestFixture {
        &self.inner
    }
}

async fn setup() -> (BulkFixture, BootResult, TestServer) {
    let (inner, boot) = boot_request_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    membership::Entity::update_many()
        .col_expr(membership::Column::Role, Expr::value("owner"))
        .filter(membership::Column::Id.eq(inner.membership.id))
        .exec(&inner.admin)
        .await
        .unwrap();
    let alpha = add_member(&inner, "Bulk Alpha", "active").await.unwrap();
    let beta = add_member(&inner, "Bulk Beta", "suspended").await.unwrap();
    let fixture = BulkFixture { inner, alpha, beta };
    let mut request = server(&boot);
    request.save_cookies();
    select_request_household(&mut request, &fixture, &boot)
        .await
        .unwrap();
    (fixture, boot, request)
}

async fn add_member(
    fixture: &RequestFixture,
    name: &str,
    status: &str,
) -> Result<membership::Model, sea_orm::DbErr> {
    let now = chrono::Utc::now().naive_utc();
    let person = person::ActiveModel {
        name: Set(name.into()),
        person_type: Set(0),
        has_capacity: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    membership::ActiveModel {
        household_id: Set(fixture.household.id),
        person_id: Set(person.id),
        role: Set("member".into()),
        status: Set(status.into()),
        permissions_version: Set(1),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
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
async fn review(
    fixture: &BulkFixture,
    request: &TestServer,
    csrf: &str,
    ids: &[&str],
    action: &str,
) -> Reply {
    review_context(fixture, request, csrf, ids, action, "Bulk", None).await
}
async fn review_context(
    fixture: &BulkFixture,
    request: &TestServer,
    csrf: &str,
    ids: &[&str],
    action: &str,
    query: &str,
    review_id: Option<&str>,
) -> Reply {
    let slug = fixture.household.slug.clone();
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
        .post(&format!("/households/{slug}/admin/users/bulk/preview"))
        .add_header("sec-fetch-site", "same-origin")
        .form(&fields)
        .await;
    Reply {
        status: response.status_code().as_u16(),
        body: response.text(),
    }
}
async fn apply_review(
    fixture: &BulkFixture,
    request: &TestServer,
    csrf: &str,
    ids: &[&str],
    action: &str,
    review_id: &str,
) -> Reply {
    apply_context(fixture, request, csrf, ids, action, review_id, "Bulk").await
}
async fn apply_context(
    fixture: &BulkFixture,
    request: &TestServer,
    csrf: &str,
    ids: &[&str],
    action: &str,
    review_id: &str,
    query: &str,
) -> Reply {
    let slug = fixture.household.slug.clone();
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
        .post(&format!("/households/{slug}/admin/users/bulk"))
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
    let slug = fixture.household.slug.clone();
    let alpha_id = fixture.alpha.id;
    let alpha = fixture.alpha.id.to_string();
    let beta = fixture.beta.id.to_string();
    let negotiated = request
        .get(&format!("/households/{slug}/admin/users?status=active"))
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
        .get(&format!(
            "/households/{slug}/admin/users?status=all&lang=en"
        ))
        .await;
    let html = listing.text();
    let token = authenticity_token(&html).ok_or("csrf missing")?;
    let preview = request
        .post(&format!("/households/{slug}/admin/users/bulk/preview"))
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("authenticity_token", token),
            ("action", "suspend"),
            ("membership_ids", alpha.as_str()),
            ("membership_ids", beta.as_str()),
            ("status", "all"),
            ("lang", "en"),
        ])
        .await;
    assert_eq!(preview.status_code(), 200, "{}", preview.text());
    assert!(preview.text().contains("Bulk Alpha"));
    assert!(preview.text().contains("Bulk Beta"));
    assert!(preview.text().contains("Confirm Suspend"));
    assert_eq!(status(&fixture, alpha_id).await, "active");
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}

async fn token(fixture: &BulkFixture, request: &TestServer) -> String {
    let slug = fixture.household.slug.clone();
    let page = request
        .get(&format!("/households/{slug}/admin/users?lang=en"))
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
async fn submit(
    fixture: &BulkFixture,
    request: &TestServer,
    csrf: &str,
    ids: &[&str],
    action: &str,
) -> Reply {
    let reviewed = review(fixture, request, csrf, ids, action).await;
    if reviewed.status_code() != 200 {
        return reviewed;
    }
    apply_review(
        fixture,
        request,
        csrf,
        ids,
        action,
        &field(reviewed.text(), "review_id"),
    )
    .await
}
async fn status(fixture: &BulkFixture, id: i64) -> String {
    membership::Entity::find_by_id(id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .status
}
async fn audit_count(fixture: &BulkFixture, outcome: &str) -> u64 {
    use sea_orm::sea_query::ExprTrait;
    security_audit_event::Entity::find()
        .filter(security_audit_event::Column::HouseholdId.eq(fixture.household.id))
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

    let alpha_id = fixture.alpha.id;
    let beta_id = fixture.beta.id;
    let alpha = fixture.alpha.id.to_string();
    let beta = fixture.beta.id.to_string();
    let moved = add_member(&fixture, "Bulk Foreign", "active").await?;
    let moved_text = moved.id.to_string();
    let _moved_id = moved.id;
    let extra = add_member(&fixture, "Bulk Missing", "active").await?;
    let extra_id = extra.id;
    let extra_text = extra_id.to_string();
    let csrf = token(&fixture, &request).await;
    let reviewed = review(
        &fixture,
        &request,
        &csrf,
        &[
            alpha.as_str(),
            beta.as_str(),
            moved_text.as_str(),
            extra_text.as_str(),
        ],
        "suspend",
    )
    .await;
    assert_eq!(reviewed.status_code(), 200, "{}", reviewed.text());
    let now = chrono::Utc::now().naive_utc();
    let foreign_household = household::ActiveModel {
        created_by_account_id: Set(Some(fixture.account.id)),
        name: Set("Foreign".into()),
        slug: Set(format!("foreign-bulk-{}", uuid::Uuid::new_v4())),
        timezone: Set("UTC".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    membership::Entity::update_many()
        .col_expr(
            membership::Column::HouseholdId,
            Expr::value(foreign_household.id),
        )
        .filter(membership::Column::Id.eq(moved.id))
        .exec(&fixture.admin)
        .await?;
    person::Entity::update_many()
        .col_expr(person::Column::Name, Expr::value("Foreign secret identity"))
        .filter(person::Column::Id.eq(moved.person_id))
        .exec(&fixture.admin)
        .await?;
    membership::Entity::delete_by_id(extra.id)
        .exec(&fixture.admin)
        .await?;
    let response = apply_review(
        &fixture,
        &request,
        &csrf,
        &[
            alpha.as_str(),
            beta.as_str(),
            moved_text.as_str(),
            extra_text.as_str(),
        ],
        "suspend",
        &field(reviewed.text(), "review_id"),
    )
    .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    let html = response.text();
    assert!(html.contains("Household access changed."));
    assert!(html.contains("Membership state changed"));
    assert!(!html.contains("Foreign secret identity"));
    assert!(html.contains(&format!("value=\"{beta_id}\" checked")));
    assert!(!html.contains(&format!("value=\"{alpha_id}\" checked")));
    assert!(html.contains("q=Bulk&amp;role=all&amp;status=all"));
    assert!(html.contains("direction=desc"));
    assert!(html.contains("page=1"));
    assert!(html.contains("lang=en"));
    assert_eq!(status(&fixture, alpha_id).await, "suspended");
    assert_eq!(status(&fixture, beta_id).await, "suspended");
    assert_eq!(status(&fixture, moved.id).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 1);
    assert_eq!(audit_count(&fixture, "rejected").await, 1);
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_member_invalid_selection_and_csrf_never_mutate() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let alpha_id = fixture.alpha.id;
    let alpha = fixture.alpha.id.to_string();
    let csrf = token(&fixture, &request).await;
    for ids in [
        vec![],
        vec![alpha.as_str(), alpha.as_str()],
        vec![alpha.as_str(), "bad"],
        vec!["0"],
        vec!["-1"],
        vec![alpha.as_str(); 26],
    ] {
        let response = submit(&fixture, &request, &csrf, &ids, "suspend").await;
        assert_eq!(response.status_code(), 422, "{}", response.text());
    }
    assert_eq!(
        submit(&fixture, &request, &csrf, &[alpha.as_str()], "restore")
            .await
            .status_code(),
        422
    );
    assert_eq!(
        submit(&fixture, &request, "invalid", &[alpha.as_str()], "suspend")
            .await
            .status_code(),
        403
    );
    assert_eq!(status(&fixture, alpha_id).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 0);
    assert_eq!(audit_count(&fixture, "rejected").await, 0);
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}
// SeaORM 2.0.4 Schema and SeaQuery 1.0.2 SchemaStatement expose no CREATE FUNCTION,
// CREATE TRIGGER or deferred constraint-trigger builder. The next three fixtures use
// isolated synthetic PostgreSQL DDL to inject authority, audit and commit failures;
// ordinary fixture CRUD and assertions use entities/builders.
#[tokio::test]
async fn bulk_member_authority_loss_stops_remaining_after_committed_success() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let alpha_id = fixture.alpha.id;
    let beta_id = fixture.beta.id;
    let actor_id = fixture.membership.id;
    let alpha = fixture.alpha.id.to_string();
    let beta = fixture.beta.id.to_string();
    membership::Entity::update_many()
        .col_expr(membership::Column::Status, Expr::value("active"))
        .filter(membership::Column::Id.eq(beta_id))
        .exec(&fixture.admin)
        .await?;
    fixture.admin.execute_unprepared(&format!(" CREATE FUNCTION bulk_remove_actor() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$ BEGIN IF NEW.event_type='household_access.membership_changed' AND NEW.metadata->>'target_membership_id'='{alpha_id}' THEN UPDATE public.household_memberships SET status='revoked',permissions_version=permissions_version+1 WHERE id={actor_id}; END IF; RETURN NEW; END $$; CREATE TRIGGER bulk_remove_actor AFTER INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION bulk_remove_actor()")).await?;
    let response = submit(
        &fixture,
        &request,
        &token(&fixture, &request).await,
        &[alpha.as_str(), beta.as_str()],
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
    assert_eq!(status(&fixture, alpha_id).await, "suspended");
    assert_eq!(status(&fixture, beta_id).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 1);
    fixture.admin.execute_unprepared("DROP TRIGGER bulk_remove_actor ON security_audit_events; DROP FUNCTION bulk_remove_actor()").await?;
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_member_audit_failure_rolls_back_without_claiming_success() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let alpha_id = fixture.alpha.id;
    let beta_id = fixture.beta.id;
    let alpha = fixture.alpha.id.to_string();
    let beta = fixture.beta.id.to_string();
    membership::Entity::update_many()
        .col_expr(membership::Column::Status, Expr::value("active"))
        .filter(membership::Column::Id.eq(beta_id))
        .exec(&fixture.admin)
        .await?;
    fixture.admin.execute_unprepared(" CREATE FUNCTION bulk_reject_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='household_access.membership_changed' THEN RAISE EXCEPTION 'Synthetic bulk audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER bulk_reject_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION bulk_reject_audit()").await?;
    let response = submit(
        &fixture,
        &request,
        &token(&fixture, &request).await,
        &[alpha.as_str(), beta.as_str()],
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
    assert_eq!(status(&fixture, alpha_id).await, "active");
    assert_eq!(status(&fixture, beta_id).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 0);
    fixture.admin.execute_unprepared("DROP TRIGGER bulk_reject_audit ON security_audit_events; DROP FUNCTION bulk_reject_audit()").await?;
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_member_commit_failure_is_unconfirmed_and_rolls_back() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let alpha_id = fixture.alpha.id;
    let beta_id = fixture.beta.id;
    let alpha = fixture.alpha.id.to_string();
    let beta = fixture.beta.id.to_string();
    membership::Entity::update_many()
        .col_expr(membership::Column::Status, Expr::value("active"))
        .filter(membership::Column::Id.eq(beta_id))
        .exec(&fixture.admin)
        .await?;
    let extra = add_member(&fixture, "Bulk Gamma", "active").await?;
    let extra_id = extra.id;
    let extra_text = extra_id.to_string();
    fixture.admin.execute_unprepared(&format!(" CREATE FUNCTION bulk_reject_commit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='household_access.membership_changed' AND NEW.metadata->>'target_membership_id'='{beta_id}' THEN RAISE EXCEPTION 'Synthetic bulk commit rejection'; END IF; RETURN NEW; END $$; CREATE CONSTRAINT TRIGGER bulk_reject_commit AFTER INSERT ON security_audit_events DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION bulk_reject_commit()")).await?;
    let response = submit(
        &fixture,
        &request,
        &token(&fixture, &request).await,
        &[alpha.as_str(), beta.as_str(), extra_text.as_str()],
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
    assert_eq!(status(&fixture, alpha_id).await, "revoked");
    assert_eq!(status(&fixture, beta_id).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 1);
    assert_eq!(status(&fixture, extra.id).await, "active");
    assert!(
        response
            .text()
            .contains(&format!("value=\"{beta_id}\" checked"))
    );
    assert!(
        response
            .text()
            .contains(&format!("value=\"{extra_id}\" checked"))
    );
    assert!(
        !response
            .text()
            .contains(&format!("value=\"{alpha_id}\" checked"))
    );
    fixture.admin.execute_unprepared("DROP TRIGGER bulk_reject_commit ON security_audit_events; DROP FUNCTION bulk_reject_commit()").await?;
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_member_actor_is_last_and_self_removal_renders_no_directory() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let alpha_id = fixture.alpha.id;
    let beta_id = fixture.beta.id;
    let actor_id = fixture.membership.id;
    let alpha = fixture.alpha.id.to_string();
    let actor = fixture.membership.id.to_string();
    membership::Entity::update_many()
        .col_expr(membership::Column::Role, Expr::value("owner"))
        .col_expr(membership::Column::Status, Expr::value("active"))
        .filter(membership::Column::Id.eq(beta_id))
        .exec(&fixture.admin)
        .await?;
    let csrf = token(&fixture, &request).await;
    let reviewed = review_context(
        &fixture,
        &request,
        &csrf,
        &[actor.as_str(), alpha.as_str()],
        "revoke",
        "",
        None,
    )
    .await;
    assert_eq!(reviewed.status_code(), 200, "{}", reviewed.text());
    let response = apply_context(
        &fixture,
        &request,
        &csrf,
        &[actor.as_str(), alpha.as_str()],
        "revoke",
        &field(reviewed.text(), "review_id"),
        "",
    )
    .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    assert_eq!(status(&fixture, alpha_id).await, "revoked");
    assert_eq!(status(&fixture, actor_id).await, "revoked");
    assert_eq!(status(&fixture, beta_id).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 2);
    assert!(!response.text().contains("Back to members"));
    assert!(!response.text().contains("Review selected failures again"));
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_member_concurrent_batches_preserve_last_owner_and_rejection_audit() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let beta_id = fixture.beta.id;
    let actor_id = fixture.membership.id;
    let beta = fixture.beta.id.to_string();
    let actor = fixture.membership.id.to_string();
    membership::Entity::update_many()
        .col_expr(membership::Column::Role, Expr::value("owner"))
        .col_expr(membership::Column::Status, Expr::value("active"))
        .filter(membership::Column::Id.eq(beta_id))
        .exec(&fixture.admin)
        .await?;
    let mut second_request = server(&boot);
    second_request.save_cookies();
    select_request_household(&mut second_request, &fixture, &boot).await?;
    let csrf = token(&fixture, &request).await;
    let second_csrf = token(&fixture, &second_request).await;
    let first_review = review_context(
        &fixture,
        &request,
        &csrf,
        &[actor.as_str(), beta.as_str()],
        "suspend",
        "",
        None,
    )
    .await;
    let second_review = review_context(
        &fixture,
        &second_request,
        &second_csrf,
        &[actor.as_str(), beta.as_str()],
        "revoke",
        "",
        None,
    )
    .await;
    assert_eq!(first_review.status_code(), 200, "{}", first_review.text());
    assert_eq!(second_review.status_code(), 200, "{}", second_review.text());
    let first_id = field(first_review.text(), "review_id");
    let second_id = field(second_review.text(), "review_id");
    let selected = [actor.as_str(), beta.as_str()];
    let (first, second) = tokio::join!(
        apply_context(
            &fixture, &request, &csrf, &selected, "suspend", &first_id, ""
        ),
        apply_context(
            &fixture,
            &second_request,
            &second_csrf,
            &selected,
            "revoke",
            &second_id,
            ""
        )
    );
    assert_eq!(first.status_code(), 200, "{}", first.text());
    assert_eq!(second.status_code(), 200, "{}", second.text());
    assert_eq!(status(&fixture, actor_id).await, "active");
    assert_ne!(status(&fixture, beta_id).await, "active");
    assert!(audit_count(&fixture, "rejected").await >= 2);
    let owners = membership::Entity::find()
        .filter(membership::Column::HouseholdId.eq(fixture.household.id))
        .filter(membership::Column::Role.eq("owner"))
        .filter(membership::Column::Status.eq("active"))
        .count(&fixture.admin)
        .await?;
    assert_eq!(owners, 1);
    drop(second_request);
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}

#[tokio::test]
async fn bulk_member_all_success_has_no_empty_retry_and_preserves_escaped_identity() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let alpha_id = fixture.alpha.id;
    let beta_id = fixture.beta.id;
    let alpha = fixture.alpha.id.to_string();
    let beta = fixture.beta.id.to_string();
    let csrf = token(&fixture, &request).await;
    let reviewed = review(
        &fixture,
        &request,
        &csrf,
        &[alpha.as_str(), beta.as_str()],
        "revoke",
    )
    .await;
    assert_eq!(reviewed.status_code(), 200);
    person::Entity::update_many()
        .col_expr(
            person::Column::Name,
            Expr::value("<script>private-name</script>"),
        )
        .filter(person::Column::Id.eq(fixture.alpha.person_id))
        .exec(&fixture.admin)
        .await?;
    let response = apply_review(
        &fixture,
        &request,
        &csrf,
        &[alpha.as_str(), beta.as_str()],
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
    assert_eq!(status(&fixture, alpha_id).await, "revoked");
    assert_eq!(status(&fixture, beta_id).await, "revoked");
    assert_eq!(audit_count(&fixture, "success").await, 2);
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_member_rejects_large_unique_selection_and_missing_confirmation() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let slug = fixture.household.slug.clone();
    let alpha_id = fixture.alpha.id;
    let alpha = fixture.alpha.id.to_string();
    let csrf = token(&fixture, &request).await;
    let owned = (1..=26)
        .map(|id| (i64::MAX - id).to_string())
        .collect::<Vec<_>>();
    let ids = owned.iter().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(
        submit(&fixture, &request, &csrf, &ids, "suspend")
            .await
            .status_code(),
        422
    );
    let response = request
        .post(&format!("/households/{slug}/admin/users/bulk"))
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("authenticity_token", csrf.as_str()),
            ("action", "suspend"),
            ("membership_ids", alpha.as_str()),
        ])
        .await;
    assert_eq!(response.status_code(), 422);
    assert_eq!(status(&fixture, alpha_id).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 0);
    assert_eq!(audit_count(&fixture, "rejected").await, 0);
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_member_denies_ordinary_revoked_locked_and_inactive_household_actors() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let alpha_id = fixture.alpha.id;
    let actor_id = fixture.membership.id;
    let alpha = fixture.alpha.id.to_string();
    let csrf = token(&fixture, &request).await;
    for scenario in 0..4 {
        match scenario {
            0 => {
                membership::Entity::update_many()
                    .col_expr(membership::Column::Role, Expr::value("member"))
                    .filter(membership::Column::Id.eq(actor_id))
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
                    .filter(membership::Column::Id.eq(actor_id))
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
                    .filter(membership::Column::Id.eq(actor_id))
                    .exec(&fixture.admin)
                    .await?;
                account_lockout::ActiveModel {
                    account_id: Set(fixture.account.id),
                    key: Set("bulk-lock".into()),
                    deadline: Set((chrono::Utc::now() + chrono::Duration::hours(1)).naive_utc()),
                    ..Default::default()
                }
                .insert(&fixture.admin)
                .await?;
            }
            _ => {
                account_lockout::Entity::delete_by_id(fixture.account.id)
                    .exec(&fixture.admin)
                    .await?;
                household::Entity::update_many()
                    .col_expr(
                        household::Column::LifecycleState,
                        Expr::value("deletion_requested"),
                    )
                    .filter(household::Column::Id.eq(fixture.household.id))
                    .exec(&fixture.admin)
                    .await?;
            }
        }
        assert_eq!(
            submit(&fixture, &request, &csrf, &[alpha.as_str()], "suspend")
                .await
                .status_code(),
            403
        );
        assert_eq!(status(&fixture, alpha_id).await, "active");
    }
    assert_eq!(audit_count(&fixture, "success").await, 0);
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}

#[tokio::test]
async fn bulk_member_anonymous_malformed_post_uses_generic_navigation_without_effects() -> TestResult
{
    let (fixture, boot, request) = setup().await;
    let alpha_id = fixture.alpha.id;
    let beta_id = fixture.beta.id;
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
        assert_eq!(status(&fixture, alpha_id).await, "active");
        assert_eq!(status(&fixture, beta_id).await, "suspended");
        assert_eq!(audit_count(&fixture, "success").await, 0);
        assert_eq!(audit_count(&fixture, "rejected").await, 0);
    }
    drop(anonymous);
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}

#[tokio::test]
async fn bulk_review_rejects_filtered_out_initial_target_without_effects() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let slug = fixture.household.slug.clone();
    let alpha_id = fixture.alpha.id;
    let beta_id = fixture.beta.id;
    let beta = fixture.beta.id.to_string();
    let csrf = token(&fixture, &request).await;
    let response = request
        .post(&format!("/households/{slug}/admin/users/bulk/preview"))
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("authenticity_token", csrf.as_str()),
            ("action", "suspend"),
            ("membership_ids", beta.as_str()),
            ("q", "Bulk Alpha"),
            ("lang", "en"),
        ])
        .await;
    assert_eq!(
        response.status_code(),
        422,
        "filtered-out target must not be reviewable"
    );
    assert_eq!(status(&fixture, alpha_id).await, "active");
    assert_eq!(status(&fixture, beta_id).await, "suspended");
    assert_eq!(audit_count(&fixture, "success").await, 0);
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}
#[tokio::test]
async fn bulk_review_requires_session_confirmation_before_mutation() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let alpha_id = fixture.alpha.id;
    let alpha = fixture.alpha.id.to_string();
    let response = apply_review(
        &fixture,
        &request,
        &token(&fixture, &request).await,
        &[alpha.as_str()],
        "suspend",
        &uuid::Uuid::new_v4().to_string(),
    )
    .await;
    assert_eq!(
        response.status_code(),
        422,
        "direct apply without a reviewed session must fail"
    );
    assert_eq!(status(&fixture, alpha_id).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 0);
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}

#[tokio::test]
async fn bulk_review_rejects_off_page_selection_without_effects() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let alpha_id = fixture.alpha.id;
    let alpha = fixture.alpha.id.to_string();
    let mut later = Vec::new();
    for index in 0..25 {
        later.push(add_member(&fixture, &format!("Bulk Z{index:02}"), "active").await?);
    }
    let csrf = token(&fixture, &request).await;
    let response = review(&fixture, &request, &csrf, &[alpha.as_str()], "revoke").await;
    assert_eq!(response.status_code(), 422, "{}", response.text());
    assert_eq!(status(&fixture, alpha_id).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 0);
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}

#[tokio::test]
async fn bulk_review_binds_session_targets_action_and_context_and_consumes_confirmation()
-> TestResult {
    let (fixture, boot, request) = setup().await;
    let alpha_id = fixture.alpha.id;
    let alpha = fixture.alpha.id.to_string();
    let beta = fixture.beta.id.to_string();
    let csrf = token(&fixture, &request).await;
    for (ids, action, query) in [
        (vec![alpha.as_str(), beta.as_str()], "revoke", "Bulk"),
        (vec![alpha.as_str()], "suspend", "Bulk"),
        (vec![alpha.as_str()], "revoke", ""),
    ] {
        let reviewed = review(&fixture, &request, &csrf, &[alpha.as_str()], "revoke").await;
        assert_eq!(reviewed.status_code(), 200, "{}", reviewed.text());
        let response = apply_context(
            &fixture,
            &request,
            &csrf,
            &ids,
            action,
            &field(reviewed.text(), "review_id"),
            query,
        )
        .await;
        assert_eq!(response.status_code(), 422, "{}", response.text());
        assert_eq!(status(&fixture, alpha_id).await, "active");
    }
    let reviewed = review(&fixture, &request, &csrf, &[alpha.as_str()], "revoke").await;
    let id = field(reviewed.text(), "review_id");
    let mut other = server(&boot);
    other.save_cookies();
    select_request_household(&mut other, &fixture, &boot).await?;
    let other_csrf = token(&fixture, &other).await;
    let foreign = apply_review(
        &fixture,
        &other,
        &other_csrf,
        &[alpha.as_str()],
        "revoke",
        &id,
    )
    .await;
    assert_eq!(foreign.status_code(), 422, "{}", foreign.text());
    assert_eq!(status(&fixture, alpha_id).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 0);
    let success = apply_review(&fixture, &request, &csrf, &[alpha.as_str()], "revoke", &id).await;
    assert_eq!(success.status_code(), 200, "{}", success.text());
    let replay = apply_review(&fixture, &request, &csrf, &[alpha.as_str()], "revoke", &id).await;
    assert_eq!(replay.status_code(), 422, "{}", replay.text());
    assert_eq!(status(&fixture, alpha_id).await, "revoked");
    assert_eq!(audit_count(&fixture, "success").await, 1);
    assert_eq!(audit_count(&fixture, "rejected").await, 0);
    drop(other);
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}

#[tokio::test]
async fn bulk_review_retry_is_failed_only_despite_shifted_page() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let beta_id = fixture.beta.id;
    let alpha = fixture.alpha.id.to_string();
    let beta = fixture.beta.id.to_string();
    let csrf = token(&fixture, &request).await;
    let reviewed = review(
        &fixture,
        &request,
        &csrf,
        &[alpha.as_str(), beta.as_str()],
        "suspend",
    )
    .await;
    assert_eq!(reviewed.status_code(), 200, "{}", reviewed.text());
    let results = apply_review(
        &fixture,
        &request,
        &csrf,
        &[alpha.as_str(), beta.as_str()],
        "suspend",
        &field(reviewed.text(), "review_id"),
    )
    .await;
    assert_eq!(results.status_code(), 200, "{}", results.text());
    let retry_id = field(results.text(), "review_id");
    let mut later = Vec::new();
    for index in 0..25 {
        later.push(add_member(&fixture, &format!("Bulk Z{index:02}"), "active").await?);
    }
    let retry = review_context(
        &fixture,
        &request,
        &csrf,
        &[beta.as_str()],
        "suspend",
        "Bulk",
        Some(&retry_id),
    )
    .await;
    assert_eq!(retry.status_code(), 200, "{}", retry.text());
    membership::Entity::update_many()
        .col_expr(membership::Column::Status, Expr::value("active"))
        .filter(membership::Column::Id.eq(beta_id))
        .exec(&fixture.admin)
        .await?;
    let retried = apply_review(
        &fixture,
        &request,
        &csrf,
        &[beta.as_str()],
        "suspend",
        &field(retry.text(), "review_id"),
    )
    .await;
    assert_eq!(retried.status_code(), 200, "{}", retried.text());
    assert_eq!(status(&fixture, beta_id).await, "suspended");
    assert_eq!(audit_count(&fixture, "success").await, 2);
    assert_eq!(audit_count(&fixture, "rejected").await, 1);
    assert!(!retried.text().contains("Review selected failures again"));
    let last_text = later.last().unwrap().id.to_string();
    let reviewed = review(&fixture, &request, &csrf, &[last_text.as_str()], "suspend").await;
    assert_eq!(reviewed.status_code(), 200);
    membership::Entity::update_many()
        .col_expr(membership::Column::Status, Expr::value("suspended"))
        .filter(membership::Column::Id.eq(later.last().unwrap().id))
        .exec(&fixture.admin)
        .await?;
    let results = apply_review(
        &fixture,
        &request,
        &csrf,
        &[last_text.as_str()],
        "suspend",
        &field(reviewed.text(), "review_id"),
    )
    .await;
    let denied = review_context(
        &fixture,
        &request,
        &csrf,
        &[alpha.as_str(), last_text.as_str()],
        "suspend",
        "Bulk",
        Some(&field(results.text(), "review_id")),
    )
    .await;
    assert_eq!(denied.status_code(), 422, "{}", denied.text());
    assert_eq!(audit_count(&fixture, "success").await, 2);
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}

#[tokio::test]
async fn bulk_review_return_to_members_cancels_pending_confirmation() -> TestResult {
    let (fixture, boot, request) = setup().await;
    let slug = fixture.household.slug.clone();
    let alpha_id = fixture.alpha.id;
    let alpha = fixture.alpha.id.to_string();
    let csrf = token(&fixture, &request).await;
    let reviewed = review(&fixture, &request, &csrf, &[alpha.as_str()], "revoke").await;
    assert_eq!(reviewed.status_code(), 200);
    request
        .get(&format!(
            "/households/{slug}/admin/users?status=all&lang=en"
        ))
        .await;
    let cancelled = apply_review(
        &fixture,
        &request,
        &csrf,
        &[alpha.as_str()],
        "revoke",
        &field(reviewed.text(), "review_id"),
    )
    .await;
    assert_eq!(cancelled.status_code(), 422, "{}", cancelled.text());
    assert_eq!(status(&fixture, alpha_id).await, "active");
    assert_eq!(audit_count(&fixture, "success").await, 0);
    drop(request);
    close_request(fixture.inner, boot).await;
    Ok(())
}
