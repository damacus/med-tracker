use super::*;
use base64::Engine as _;
use med_tracker::models::_entities::{
    accounts, app_settings, household_invitations, household_memberships, households, people,
    security_audit_events, versions,
};
use med_tracker::models::entities::identity_session;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter,
    QuerySelect, Set,
};
use sha2::Digest as _;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn signed_out_root_redirects_to_login_without_profile_controls() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let mut request = server(&boot);
    request.save_cookies();
    let root = request.get("/").await;
    assert_eq!(root.status_code(), 303);
    assert_eq!(root.header("location"), "/login");
    assert_eq!(root.header("cache-control"), "no-store");
    let login = request.get("/login").await;
    assert_eq!(login.status_code(), 200);
    assert!(!login.text().contains("appearance-dialog"));
    assert!(!login.text().contains("data-appearance-choice"));
    assert!(
        !login
            .text()
            .contains("href=\"/households/persistence-fixture/profile\"")
    );
    let profile = request
        .get(&format!("/households/{}/profile", fixture.household.slug))
        .await;
    assert_eq!(profile.status_code(), 303);
    assert_eq!(profile.header("location"), "/login");
    browser_session(&fixture, &mut request, &boot).await;
    identity_session::Entity::update_many()
        .col_expr(
            identity_session::Column::ExpiresAt,
            sea_orm::sea_query::Expr::value(chrono::Utc::now() - chrono::Duration::days(1)),
        )
        .filter(identity_session::Column::UserId.eq(fixture.account.id))
        .exec(&fixture.admin)
        .await?;
    let expired = request.get("/").await;
    assert_eq!(expired.status_code(), 303);
    assert_eq!(expired.header("location"), "/login");
    assert_eq!(expired.header("cache-control"), "no-store");
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

async fn boot_browser_fixture() -> (RequestFixture, BootResult) {
    boot_request_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await
}

async fn browser_session(fixture: &RequestFixture, request: &mut TestServer, boot: &BootResult) {
    install_browser_session(request, boot, fixture.account.id).await;
}

async fn entrant_browser_session(
    fixture: &RequestFixture,
    request: &mut TestServer,
    boot: &BootResult,
) {
    install_browser_session(request, boot, fixture.foreign.account.id).await;
}

async fn entrant(fixture: &RequestFixture) -> TestResult {
    household_memberships::Entity::delete_by_id(fixture.foreign.membership.id)
        .exec(&fixture.admin)
        .await?;
    households::Entity::delete_by_id(fixture.foreign.household.id)
        .exec(&fixture.admin)
        .await?;
    Ok(())
}

pub(super) async fn home(
    scenario: &Scenario,
    name: &str,
    slug_label: &str,
    role: &str,
    revoked: bool,
) -> Result<(households::Model, household_memberships::Model), sea_orm::DbErr> {
    let now = chrono::Utc::now().naive_utc();
    let household = households::ActiveModel {
        created_by_account_id: Set(Some(scenario.account.id)),
        name: Set(name.into()),
        slug: Set(format!("{slug_label}-{}", uuid::Uuid::new_v4())),
        timezone: Set("UTC".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&scenario.admin)
    .await?;
    let membership = household_memberships::ActiveModel {
        household_id: Set(household.id),
        person_id: Set(scenario.account.person_id),
        role: Set(role.into()),
        status: Set(if revoked { "revoked" } else { "active" }.into()),
        revoked_at: Set(revoked.then_some(now)),
        joined_at: Set(now - chrono::Duration::days(i64::from(revoked))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&scenario.admin)
    .await?;
    Ok((household, membership))
}

async fn invitation(fixture: &RequestFixture, token: &str) -> TestResult {
    let now = chrono::Utc::now().naive_utc();
    household_invitations::ActiveModel {
        household_id: Set(fixture.household.id),
        email: Set(fixture.foreign.account.email.clone()),
        membership_role: Set("member".into()),
        token_digest: Set(hex::encode(Sha256::digest(token.as_bytes()))),
        invited_by_membership_id: Set(fixture.membership.id),
        expires_at: Set(now + chrono::Duration::days(7)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    Ok(())
}

async fn entry_state(fixture: &RequestFixture) -> Value {
    let person_id = fixture.foreign.account.person_id;
    let account_id = fixture.foreign.account.id;
    let households = households::Entity::find()
        .filter(households::Column::CreatedByAccountId.eq(account_id))
        .count(&fixture.admin)
        .await
        .unwrap();
    let members = household_memberships::Entity::find()
        .filter(household_memberships::Column::PersonId.eq(person_id))
        .all(&fixture.admin)
        .await
        .unwrap();
    let audits = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::ActorAccountId.eq(account_id))
        .filter(security_audit_events::Column::EventType.eq("household_access.membership_created"))
        .count(&fixture.admin)
        .await
        .unwrap();
    let selected = identity_session::Entity::find()
        .filter(identity_session::Column::UserId.eq(account_id))
        .filter(identity_session::Column::ActiveHouseholdId.is_not_null())
        .count(&fixture.admin)
        .await
        .unwrap();
    let accepted = household_invitations::Entity::find()
        .filter(household_invitations::Column::Email.eq(&fixture.foreign.account.email))
        .filter(household_invitations::Column::AcceptedAt.is_not_null())
        .count(&fixture.admin)
        .await
        .unwrap();
    let accounts = accounts::Entity::find()
        .filter(accounts::Column::PersonId.eq(person_id))
        .count(&fixture.admin)
        .await
        .unwrap();
    let people = people::Entity::find_by_id(person_id)
        .count(&fixture.admin)
        .await
        .unwrap();
    let person_events = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("Person"))
        .filter(versions::Column::ItemId.eq(person_id))
        .filter(versions::Column::Event.eq("create"))
        .count(&fixture.admin)
        .await
        .unwrap();
    json!({"households":households,"memberships":members.len(),"audits":audits,"selected":selected,"accepted":accepted,
        "active":members.iter().filter(|row|row.status=="active").count(),
        "revoked":members.iter().filter(|row|row.status=="revoked").count(),
        "accounts":accounts,"people":people,"person_events":person_events})
}

async fn selected_count(fixture: &RequestFixture) -> Result<u64, sea_orm::DbErr> {
    identity_session::Entity::find()
        .filter(identity_session::Column::UserId.eq(fixture.account.id))
        .filter(identity_session::Column::ActiveHouseholdId.is_not_null())
        .count(&fixture.admin)
        .await
}

#[tokio::test]
async fn verified_account_without_membership_creates_first_household() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let household_id = fixture.household.id;
    let _household_text = household_id.to_string();

    let previous_settings = super::account_authentication::registration_setting(&fixture).await;
    let mut settings = app_settings::Entity::find()
        .one(&fixture.admin)
        .await?
        .unwrap()
        .into_active_model();
    settings.invite_only = Set(true);
    settings.update(&fixture.admin).await?;
    entrant(&fixture).await?;
    let mut account = fixture.foreign.account.clone().into_active_model();
    account.preferences = Set(json!({"time_zone":"Europe/London"}));
    account.update(&fixture.admin).await?;
    let mut request = server(&boot);
    request.save_cookies();
    entrant_browser_session(&fixture, &mut request, &boot).await;
    let chooser = request.get("/households/choose").await;
    assert_eq!(chooser.status_code(), 200, "{}", chooser.text());
    let html = chooser.text();
    assert!(html.contains("action=\"/households\""));
    let token = authenticity_token(&html).ok_or("creation token missing")?;
    let created = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", "New care home"), ("authenticity_token", token)])
        .await;
    assert_eq!(created.status_code(), 303, "{}", created.text());
    let response_request_id = created.header("x-request-id").to_str()?.to_owned();
    let audit = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::ActorAccountId.eq(fixture.foreign.account.id))
        .filter(security_audit_events::Column::EventType.eq("household_access.membership_created"))
        .one(&fixture.admin)
        .await?
        .ok_or("creation audit missing")?;
    assert_eq!(
        audit.request_id.as_deref(),
        Some(response_request_id.as_str())
    );
    assert_eq!(audit.audit_context["request_id"], response_request_id);
    let location_header = created.header("location");
    let location = location_header.to_str()?;
    let dashboard = request.get(location).await;
    assert_eq!(dashboard.status_code(), 200, "{}", dashboard.text());
    let homes = households::Entity::find()
        .filter(households::Column::CreatedByAccountId.eq(fixture.foreign.account.id))
        .filter(households::Column::Name.eq("New care home"))
        .filter(households::Column::Timezone.eq("Europe/London"))
        .all(&fixture.admin)
        .await?;
    let count = household_memberships::Entity::find()
        .filter(household_memberships::Column::PersonId.eq(fixture.foreign.account.person_id))
        .filter(household_memberships::Column::Role.eq("owner"))
        .filter(household_memberships::Column::Status.eq("active"))
        .filter(
            household_memberships::Column::HouseholdId.is_in(homes.into_iter().map(|home| home.id)),
        )
        .count(&fixture.admin)
        .await?;
    assert_eq!(count, 1);
    let row = entry_state(&fixture).await;
    assert_eq!(row["memberships"].as_u64().unwrap(), 1);
    let identity = entry_state(&fixture).await;
    assert_eq!(identity["accounts"].as_u64().unwrap(), 1);
    assert_eq!(identity["people"].as_u64().unwrap(), 1);
    assert_eq!(identity["person_events"].as_u64().unwrap(), 0);
    super::account_authentication::restore_registration(&fixture, previous_settings).await;
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn held_or_archived_household_membership_allows_new_selectable_household() -> TestResult {
    for inactive_state in ["held", "archived"] {
        let (fixture, boot) = boot_browser_fixture().await;
        let household_id = fixture.household.id;
        let _household_text = household_id.to_string();

        entrant(&fixture).await?;
        let (previous, _) = home(
            &fixture.foreign,
            "Previous household",
            "previous-household",
            "owner",
            false,
        )
        .await?;
        let previous_slug = previous.slug.clone();
        let update = households::Entity::update_many();
        let update = if inactive_state == "held" {
            update.col_expr(households::Column::LifecycleState, Expr::value("held"))
        } else {
            update.col_expr(households::Column::Status, Expr::value("archived"))
        };
        update
            .filter(households::Column::Id.eq(previous.id))
            .exec(&fixture.admin)
            .await?;
        let mut request = server(&boot);
        request.save_cookies();
        entrant_browser_session(&fixture, &mut request, &boot).await;
        let chooser = request.get("/households/choose").await;
        assert_eq!(chooser.status_code(), 200, "{}", chooser.text());
        let html = chooser.text();
        assert!(html.contains("Create a household"));
        let csrf = authenticity_token(&html).ok_or("creation token missing")?;
        let created = request
            .post("/households")
            .add_header("sec-fetch-site", "same-origin")
            .form(&[
                ("name", "New selectable home"),
                ("authenticity_token", csrf),
            ])
            .await;
        assert_eq!(
            created.status_code(),
            303,
            "{inactive_state}: {}",
            created.text()
        );
        let dashboard = request.get(created.header("location").to_str()?).await;
        assert_eq!(
            dashboard.status_code(),
            200,
            "{inactive_state}: {}",
            dashboard.text()
        );
        let old = request
            .get(format!("/households/{previous_slug}/dashboard").as_str())
            .await;
        assert!(
            old.status_code().is_client_error(),
            "{inactive_state}: {}",
            old.text()
        );
        drop(request);
        close_request(fixture, boot).await;
    }
    Ok(())
}

#[tokio::test]
async fn existing_member_invalid_creation_post_does_not_render_empty_chooser() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let household_id = fixture.household.id;
    let _household_text = household_id.to_string();

    let mut request = server(&boot);
    request.save_cookies();
    browser_session(&fixture, &mut request, &boot).await;
    let chooser = request.get("/households/choose?switch=1").await;
    assert_eq!(chooser.status_code(), 200, "{}", chooser.text());
    let chooser_html = chooser.text();
    let csrf = authenticity_token(&chooser_html).ok_or("selection token missing")?;
    let rejected = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", " "), ("authenticity_token", csrf)])
        .await;
    assert_eq!(rejected.status_code(), 403, "{}", rejected.text());
    let chooser = request.get("/households/choose?switch=1").await;
    assert_eq!(chooser.status_code(), 200, "{}", chooser.text());
    assert!(chooser.text().contains("action=\"/households/select\""));
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn supplementary_plane_name_above_browser_utf16_limit_is_accepted() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let household_id = fixture.household.id;
    let _household_text = household_id.to_string();

    entrant(&fixture).await?;
    let mut request = server(&boot);
    request.save_cookies();
    entrant_browser_session(&fixture, &mut request, &boot).await;
    let html = request.get("/households/choose").await.text();
    let csrf = authenticity_token(&html).ok_or("creation token missing")?;
    let name = format!("Care {}", "😀".repeat(51));
    assert!(name.chars().count() <= 100);
    assert!(name.encode_utf16().count() > 100);
    let created = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", name.as_str()), ("authenticity_token", csrf)])
        .await;
    assert_eq!(created.status_code(), 303, "{}", created.text());
    let row = households::Entity::find()
        .filter(households::Column::CreatedByAccountId.eq(fixture.foreign.account.id))
        .one(&fixture.admin)
        .await?
        .ok_or("household missing")?;
    assert_eq!(row.name, name);
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn verified_account_without_membership_accepts_invitation() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let household_id = fixture.household.id;
    let _household_text = household_id.to_string();
    let slug = fixture.household.slug.clone();

    entrant(&fixture).await?;
    let (departed, _) = home(
        &fixture.foreign,
        "Departed household",
        "departed-household",
        "member",
        true,
    )
    .await?;
    let departed_slug = departed.slug.clone();
    let invitation_token = fixture.credential("account-only-household-invitation");
    invitation(&fixture, &invitation_token).await?;
    let mut request = server(&boot);
    request.save_cookies();
    entrant_browser_session(&fixture, &mut request, &boot).await;
    let preview = request
        .get(&format!("/invitations/accept?token={invitation_token}"))
        .await;
    assert_eq!(preview.status_code(), 200, "{}", preview.text());
    let html = preview.text();
    let csrf = authenticity_token(&html).ok_or("invitation token missing")?;
    let accepted = request
        .post("/invitations/accept")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("token", invitation_token.as_str()),
            ("sharing_consent", "true"),
            ("authenticity_token", csrf),
        ])
        .await;
    assert_eq!(accepted.status_code(), 303, "{}", accepted.text());
    assert_eq!(
        accepted.header("location"),
        format!("/households/{slug}/dashboard").as_str()
    );
    let dashboard = request
        .get(format!("/households/{slug}/dashboard").as_str())
        .await;
    assert_eq!(dashboard.status_code(), 200, "{}", dashboard.text());
    let departed = request
        .get(format!("/households/{departed_slug}/dashboard").as_str())
        .await;
    assert!(
        departed.status_code().is_client_error(),
        "{}",
        departed.text()
    );
    let replay = request
        .post("/invitations/accept")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("token", invitation_token.as_str()),
            ("sharing_consent", "true"),
            ("authenticity_token", csrf),
        ])
        .await;
    assert_eq!(replay.status_code(), 303, "{}", replay.text());
    let mut row = entry_state(&fixture).await;
    let members = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(fixture.household.id))
        .filter(household_memberships::Column::PersonId.eq(fixture.foreign.account.person_id))
        .filter(household_memberships::Column::Status.eq("active"))
        .count(&fixture.admin)
        .await?;
    row["memberships"] = json!(members);
    assert_eq!(row["memberships"].as_u64().unwrap(), 1);
    assert_eq!(row["person_events"].as_u64().unwrap(), 0);
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn first_household_creation_rejects_invalid_input_csrf_and_expired_session() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let household_id = fixture.household.id;
    let _household_text = household_id.to_string();

    entrant(&fixture).await?;
    let mut request = server(&boot);
    request.save_cookies();
    entrant_browser_session(&fixture, &mut request, &boot).await;
    let chooser = request.get("/households/choose").await;
    let html = chooser.text();
    let csrf = authenticity_token(&html).ok_or("creation token missing")?;
    let invalid_csrf = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("name", "Rejected household"),
            ("authenticity_token", "invalid"),
        ])
        .await;
    assert_eq!(invalid_csrf.status_code(), 403, "{}", invalid_csrf.text());
    let overlong = "x".repeat(101);
    for name in [" ", "A\nB", overlong.as_str()] {
        let invalid = request
            .post("/households")
            .add_header("sec-fetch-site", "same-origin")
            .form(&[("name", name), ("authenticity_token", csrf)])
            .await;
        assert_eq!(invalid.status_code(), 422, "{}", invalid.text());
        assert!(invalid.text().contains("Enter a household name"));
    }
    identity_session::Entity::update_many()
        .col_expr(
            identity_session::Column::ExpiresAt,
            Expr::value(chrono::Utc::now() - chrono::Duration::seconds(1)),
        )
        .filter(identity_session::Column::UserId.eq(fixture.foreign.account.id))
        .exec(&fixture.admin)
        .await?;
    let expired = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", "Rejected household"), ("authenticity_token", csrf)])
        .await;
    assert!(
        expired.status_code().is_client_error(),
        "{}",
        expired.text()
    );
    let row = entry_state(&fixture).await;
    assert_eq!(row["households"].as_u64().unwrap(), 0);
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn concurrent_first_household_submissions_create_only_one_membership() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let household_id = fixture.household.id;
    let _household_text = household_id.to_string();

    entrant(&fixture).await?;
    let mut first = server(&boot);
    first.save_cookies();
    let mut second = server(&boot);
    second.save_cookies();
    entrant_browser_session(&fixture, &mut first, &boot).await;
    entrant_browser_session(&fixture, &mut second, &boot).await;
    let first_html = first.get("/households/choose").await.text();
    let second_html = second.get("/households/choose").await.text();
    let first_csrf = authenticity_token(&first_html).ok_or("first creation token missing")?;
    let second_csrf = authenticity_token(&second_html).ok_or("second creation token missing")?;
    let (one, two) = tokio::join!(
        first
            .post("/households")
            .add_header("sec-fetch-site", "same-origin")
            .form(&[("name", "First home"), ("authenticity_token", first_csrf)]),
        second
            .post("/households")
            .add_header("sec-fetch-site", "same-origin")
            .form(&[("name", "Second home"), ("authenticity_token", second_csrf)])
    );
    let statuses = [one.status_code().as_u16(), two.status_code().as_u16()];
    assert!(
        statuses.contains(&303),
        "{statuses:?}: {} / {}",
        one.text(),
        two.text()
    );
    assert!(
        statuses.contains(&403),
        "{statuses:?}: {} / {}",
        one.text(),
        two.text()
    );
    let row = entry_state(&fixture).await;
    assert_eq!(row["households"].as_u64().unwrap(), 1);
    assert_eq!(row["memberships"].as_u64().unwrap(), 1);
    assert_eq!(row["audits"].as_u64().unwrap(), 1);
    drop(first);
    drop(second);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn revoked_last_membership_allows_new_household_without_restoring_access() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let household_id = fixture.household.id;
    let _household_text = household_id.to_string();

    entrant(&fixture).await?;
    let mut request = server(&boot);
    request.save_cookies();
    entrant_browser_session(&fixture, &mut request, &boot).await;
    let chooser = request.get("/households/choose").await.text();
    let csrf = authenticity_token(&chooser).ok_or("creation token missing")?;
    let first = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", "First home"), ("authenticity_token", csrf)])
        .await;
    assert_eq!(first.status_code(), 303, "{}", first.text());
    let first_header = first.header("location");
    let first_path = first_header.to_str()?;
    let duplicate = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", "Duplicate home"), ("authenticity_token", csrf)])
        .await;
    assert_eq!(duplicate.status_code(), 403, "{}", duplicate.text());
    household_memberships::Entity::update_many()
        .col_expr(
            household_memberships::Column::Status,
            Expr::value("revoked"),
        )
        .col_expr(
            household_memberships::Column::RevokedAt,
            Expr::value(chrono::Utc::now().naive_utc()),
        )
        .filter(household_memberships::Column::PersonId.eq(fixture.foreign.account.person_id))
        .exec(&fixture.admin)
        .await?;
    let denied = request.get(first_path).await;
    assert!(denied.status_code().is_client_error(), "{}", denied.text());
    let empty = request.get("/households/choose").await;
    assert_eq!(empty.status_code(), 200, "{}", empty.text());
    let empty_html = empty.text();
    let new_csrf = authenticity_token(&empty_html).ok_or("new creation token missing")?;
    let second = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", "Second home"), ("authenticity_token", new_csrf)])
        .await;
    assert_eq!(second.status_code(), 303, "{}", second.text());
    let second_header = second.header("location");
    let second_dashboard = request.get(second_header.to_str()?).await;
    assert_eq!(
        second_dashboard.status_code(),
        200,
        "{}",
        second_dashboard.text()
    );
    let still_denied = request.get(first_path).await;
    assert!(
        still_denied.status_code().is_client_error(),
        "{}",
        still_denied.text()
    );
    let row = entry_state(&fixture).await;
    assert_eq!(row["active"].as_u64().unwrap(), 1);
    assert_eq!(row["revoked"].as_u64().unwrap(), 1);
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn first_household_creation_rolls_back_when_audit_fails() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let household_id = fixture.household.id;
    let _household_text = household_id.to_string();

    let entrant_account_id = fixture.foreign.account.id;
    entrant(&fixture).await?;
    fixture.admin.execute_unprepared(&format!("CREATE FUNCTION reject_first_household_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.actor_account_id={entrant_account_id} AND NEW.event_type='household_access.membership_created' THEN RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_first_household_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION reject_first_household_audit()")).await?;
    let mut request = server(&boot);
    request.save_cookies();
    entrant_browser_session(&fixture, &mut request, &boot).await;
    let chooser = request.get("/households/choose").await.text();
    let csrf = authenticity_token(&chooser).ok_or("creation token missing")?;
    let failed = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", "Should roll back"), ("authenticity_token", csrf)])
        .await;
    assert_eq!(failed.status_code(), 503, "{}", failed.text());
    let row = entry_state(&fixture).await;
    assert_eq!(row["households"].as_u64().unwrap(), 0);
    assert_eq!(row["memberships"].as_u64().unwrap(), 0);
    assert_eq!(row["audits"].as_u64().unwrap(), 0);
    fixture.admin.execute_unprepared("DROP TRIGGER reject_first_household_audit ON security_audit_events; DROP FUNCTION reject_first_household_audit()").await?;
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn committed_household_survives_current_session_selection_failure() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let household_id = fixture.household.id;
    let _household_text = household_id.to_string();

    let entrant_account_id = fixture.foreign.account.id;
    entrant(&fixture).await?;
    fixture.admin.execute_unprepared(&format!("CREATE FUNCTION reject_first_household_selection() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.account_id={entrant_account_id} AND NEW.active_household_id IS DISTINCT FROM OLD.active_household_id THEN RAISE EXCEPTION 'synthetic selection failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_first_household_selection BEFORE UPDATE ON identity_sessions FOR EACH ROW EXECUTE FUNCTION reject_first_household_selection()")).await?;
    let mut request = server(&boot);
    request.save_cookies();
    entrant_browser_session(&fixture, &mut request, &boot).await;
    let chooser = request.get("/households/choose").await.text();
    let csrf = authenticity_token(&chooser).ok_or("creation token missing")?;
    let created = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", "Recoverable home"), ("authenticity_token", csrf)])
        .await;
    assert_eq!(created.status_code(), 303, "{}", created.text());
    assert_eq!(created.header("location"), "/households/choose");
    let recovery = request.get("/households/choose").await;
    assert_eq!(recovery.status_code(), 200, "{}", recovery.text());
    assert!(
        recovery
            .text()
            .contains("Your household was created. Select it below")
    );
    assert!(recovery.text().contains("Recoverable home"));
    let row = entry_state(&fixture).await;
    assert_eq!(row["households"].as_u64().unwrap(), 1);
    assert_eq!(row["memberships"].as_u64().unwrap(), 1);
    assert_eq!(row["selected"].as_u64().unwrap(), 0);
    fixture.admin.execute_unprepared("DROP TRIGGER reject_first_household_selection ON identity_sessions; DROP FUNCTION reject_first_household_selection()").await?;
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn account_only_invitation_rechecks_csrf_expiry_and_inviter_authority() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let household_id = fixture.household.id;
    let _household_text = household_id.to_string();

    entrant(&fixture).await?;
    let invitation_token = fixture.credential("account-only-household-invitation");
    invitation(&fixture, &invitation_token).await?;
    let mut request = server(&boot);
    request.save_cookies();
    entrant_browser_session(&fixture, &mut request, &boot).await;
    let preview = request
        .get(&format!("/invitations/accept?token={invitation_token}"))
        .await;
    assert_eq!(preview.status_code(), 200, "{}", preview.text());
    let html = preview.text();
    let csrf = authenticity_token(&html).ok_or("invitation token missing")?;
    let rejected = request
        .post("/invitations/accept")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("token", invitation_token.as_str()),
            ("sharing_consent", "true"),
            ("authenticity_token", "invalid"),
        ])
        .await;
    assert_eq!(rejected.status_code(), 403, "{}", rejected.text());
    household_invitations::Entity::update_many()
        .col_expr(
            household_invitations::Column::ExpiresAt,
            Expr::value(chrono::Utc::now().naive_utc() - chrono::Duration::seconds(1)),
        )
        .filter(household_invitations::Column::Email.eq(&fixture.foreign.account.email))
        .exec(&fixture.admin)
        .await?;
    let expired = request
        .post("/invitations/accept")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("token", invitation_token.as_str()),
            ("sharing_consent", "true"),
            ("authenticity_token", csrf),
        ])
        .await;
    assert_eq!(expired.status_code(), 404, "{}", expired.text());
    household_invitations::Entity::update_many()
        .col_expr(
            household_invitations::Column::ExpiresAt,
            Expr::value(chrono::Utc::now().naive_utc() + chrono::Duration::days(7)),
        )
        .filter(household_invitations::Column::Email.eq(&fixture.foreign.account.email))
        .exec(&fixture.admin)
        .await?;
    fixture
        .membership_state(fixture.membership.permissions_version, true)
        .await;
    let mut row = fixture.membership.clone().into_active_model();
    row.status = Set("revoked".into());
    row.update(&fixture.admin).await?;
    let revoked_inviter = request
        .post("/invitations/accept")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("token", invitation_token.as_str()),
            ("sharing_consent", "true"),
            ("authenticity_token", csrf),
        ])
        .await;
    assert_eq!(
        revoked_inviter.status_code(),
        404,
        "{}",
        revoked_inviter.text()
    );
    let row = entry_state(&fixture).await;
    assert_eq!(row["memberships"].as_u64().unwrap(), 0);
    assert_eq!(row["accepted"].as_u64().unwrap(), 0);
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

fn authenticity_token(html: &str) -> Option<&str> {
    html.split("name=\"authenticity_token\" value=\"")
        .nth(1)?
        .split('"')
        .next()
}

#[tokio::test]
async fn signed_in_without_active_household_reaches_initial_selection() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let household_id = fixture.household.id;
    let household_text = household_id.to_string();
    let slug = fixture.household.slug.clone();

    let mut request = server(&boot);
    request.save_cookies();
    browser_session(&fixture, &mut request, &boot).await;
    let home = request.get("/").await;
    assert_eq!(home.status_code(), 303, "{}", home.text());
    assert_eq!(home.header("location"), "/households/choose");
    assert_eq!(home.header("cache-control"), "no-store");
    let chooser = request.get("/households/choose").await;
    assert_eq!(chooser.status_code(), 200, "{}", chooser.text());
    assert_eq!(chooser.header("cache-control"), "no-store");
    assert!(chooser.text().contains("action=\"/households/select\""));
    assert!(chooser.text().contains("name=\"authenticity_token\""));
    assert!(chooser.text().contains("name=\"household_id\""));
    let chooser_html = chooser.text();
    let token = authenticity_token(&chooser_html).ok_or("household form token missing")?;
    let selected = request
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("household_id", household_text.as_str()),
            ("authenticity_token", token),
        ])
        .await;
    assert_eq!(selected.status_code(), 303, "{}", selected.text());
    assert_eq!(
        selected.header("location"),
        format!("/households/{slug}/dashboard").as_str()
    );
    let chosen_again = request.get("/households/choose").await;
    assert_eq!(chosen_again.status_code(), 303, "{}", chosen_again.text());
    assert_eq!(
        chosen_again.header("location"),
        format!("/households/{slug}/dashboard").as_str()
    );
    assert_eq!(chosen_again.header("cache-control"), "no-store");
    fixture
        .membership_state(fixture.membership.permissions_version, true)
        .await;
    let mut row = fixture.membership.clone().into_active_model();
    row.status = Set("revoked".into());
    row.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    row.update(&fixture.admin).await?;
    let revoked_home = request.get("/").await;
    assert_eq!(revoked_home.status_code(), 303, "{}", revoked_home.text());
    assert_eq!(revoked_home.header("location"), "/households/choose");
    assert_eq!(revoked_home.header("cache-control"), "no-store");
    let empty_chooser = request.get("/households/choose").await;
    assert_eq!(empty_chooser.status_code(), 200, "{}", empty_chooser.text());
    assert!(
        empty_chooser
            .text()
            .contains("You do not have an active household membership.")
    );
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn household_free_profile_survives_last_membership_revocation() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let mut request = server(&boot);
    request.save_cookies();
    browser_session(&fixture, &mut request, &boot).await;
    let mut membership = fixture.membership.clone().into_active_model();
    membership.status = Set("revoked".into());
    membership.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    membership.update(&fixture.admin).await?;
    let chooser = request.get("/households/choose").await;
    assert_eq!(chooser.status_code(), 200, "{}", chooser.text());
    assert!(chooser.text().contains("href=\"/profile\""));
    let page = request.get("/profile").await;
    assert_eq!(page.status_code(), 200, "{}", page.text());
    assert_eq!(page.header("cache-control"), "no-store");
    let html = page.text();
    assert!(html.contains(&fixture.account.email));
    assert!(!html.contains("data-app-drawer-layout"));
    assert!(!html.contains("action=\"/households/"));
    let csrf = authenticity_token(&html).ok_or("profile form token missing")?;
    let saved = request
        .post("/profile")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("authenticity_token", csrf),
            ("setting", "time_zone"),
            ("time_zone", "Europe/London"),
        ])
        .await;
    assert_eq!(saved.status_code(), 303, "{}", saved.text());
    let refreshed = request.get("/profile").await;
    assert_eq!(refreshed.status_code(), 200, "{}", refreshed.text());
    assert!(refreshed.text().contains("Europe/London"));
    let invalid = request
        .post("/profile")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("authenticity_token", csrf),
            ("setting", "time_zone"),
            ("time_zone", "Invalid/Zone"),
        ])
        .await;
    assert_eq!(invalid.status_code(), 422, "{}", invalid.text());
    assert!(invalid.text().contains("Invalid/Zone"));
    let denied = request
        .get(&format!("/households/{}/dashboard", fixture.household.slug))
        .await;
    assert!(matches!(denied.status_code().as_u16(), 403 | 404));
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn household_free_history_contains_only_the_subjects_taken_doses() -> TestResult {
    use med_tracker::models::_entities::{medication_takes, person_medications, schedules};
    let (fixture, boot) = boot_browser_fixture().await;
    let now = chrono::Utc::now().naive_utc();
    let assignment = person_medications::Entity::find()
        .filter(person_medications::Column::PersonId.eq(fixture.account.person_id))
        .one(&fixture.admin)
        .await?
        .ok_or("assignment missing")?;
    medication_takes::ActiveModel {
        household_id: Set(fixture.household.id),
        person_medication_id: Set(Some(assignment.id)),
        taken_from_medication_id: Set(Some(fixture.medication.id)),
        taken_from_location_id: Set(Some(fixture.medication.location_id)),
        dose_amount: Set(Some(sea_orm::prelude::Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".into())),
        taken_at: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    let (other_home, other_membership) = home(
        &fixture.scenario,
        "Former home",
        "personal-history",
        "member",
        true,
    )
    .await?;
    let other = Scenario {
        admin: fixture.admin.clone(),
        runtime: fixture.runtime.clone(),
        account: fixture.account.clone(),
        household: other_home,
        membership: other_membership,
    };
    let other_medicine = other
        .medication(
            "Personal medicine from another home",
            sea_orm::prelude::Decimal::new(8, 0),
        )
        .await;
    let schedule = schedules::ActiveModel {
        household_id: Set(other.household.id),
        person_id: Set(fixture.account.person_id),
        medication_id: Set(other_medicine.id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    medication_takes::ActiveModel {
        household_id: Set(other.household.id),
        schedule_id: Set(Some(schedule.id)),
        taken_from_medication_id: Set(Some(other_medicine.id)),
        dose_amount: Set(Some(sea_orm::prelude::Decimal::new(1, 0))),
        dose_unit: Set(Some("tablet".into())),
        taken_at: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    let foreign_medication = fixture
        .foreign
        .medication(
            "Private foreign medicine",
            sea_orm::prelude::Decimal::new(20, 0),
        )
        .await;
    let foreign_assignment = fixture
        .foreign
        .personal_assignment(foreign_medication.id, 0)
        .await;
    medication_takes::ActiveModel {
        household_id: Set(fixture.foreign.household.id),
        person_medication_id: Set(Some(foreign_assignment.id)),
        taken_from_medication_id: Set(Some(foreign_medication.id)),
        dose_amount: Set(Some(sea_orm::prelude::Decimal::new(1, 0))),
        dose_unit: Set(Some("tablet".into())),
        taken_at: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    let mut request = server(&boot);
    request.save_cookies();
    browser_session(&fixture, &mut request, &boot).await;
    let mut membership = fixture.membership.clone().into_active_model();
    membership.status = Set("revoked".into());
    membership.revoked_at = Set(Some(now));
    membership.update(&fixture.admin).await?;
    let mut subject = people::Entity::find_by_id(fixture.account.person_id)
        .one(&fixture.admin)
        .await?
        .ok_or("clinical person missing")?
        .into_active_model();
    subject.person_type = Set(2);
    subject.has_capacity = Set(false);
    subject.update(&fixture.admin).await?;
    let history = request.get("/profile/history").await;
    assert_eq!(history.status_code(), 200, "{}", history.text());
    assert_eq!(history.header("cache-control"), "no-store");
    assert!(history.text().contains("Synthetic tablets"));
    assert!(
        history
            .text()
            .contains("Personal medicine from another home")
    );
    assert!(!history.text().contains("Private foreign medicine"));
    assert!(!history.text().contains("Synthetic cabinet"));
    assert!(!history.text().contains("current_supply"));
    let other = request
        .get(&format!(
            "/profile/history?person_id={}",
            fixture.foreign.account.person_id
        ))
        .await;
    assert!(!other.text().contains("Private foreign medicine"));
    let anonymous = server(&boot).get("/profile/history").await;
    assert!(matches!(anonymous.status_code().as_u16(), 303 | 401));
    let audits = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("AccountClinicalAccess"))
        .filter(versions::Column::ItemId.eq(fixture.account.id))
        .filter(versions::Column::Event.eq("personal_history_read"))
        .count(&fixture.admin)
        .await?;
    assert!(audits >= 1);
    for (privilege, expected) in [
        ("SELECT", true),
        ("INSERT", false),
        ("UPDATE", false),
        ("DELETE", false),
    ] {
        let permissions = sea_orm::sea_query::Query::select()
            .expr_as(
                sea_orm::sea_query::Func::cust("pg_catalog.has_table_privilege").args([
                    Expr::val("public.personal_taken_doses"),
                    Expr::val(privilege),
                ]),
                "allowed",
            )
            .to_owned();
        let row = fixture
            .runtime
            .query_one_raw(fixture.runtime.get_database_backend().build(&permissions))
            .await?
            .ok_or("view privileges missing")?;
        assert_eq!(
            row.try_get::<bool>("", "allowed")?,
            expected,
            "runtime {privilege} privilege"
        );
    }
    let transaction = fixture.runtime.begin().await?;
    let context = sea_orm::sea_query::Query::select()
        .expr(
            sea_orm::sea_query::Func::cust("pg_catalog.set_config").args([
                Expr::val("med_tracker.current_account_id"),
                Expr::val(fixture.account.id.to_string()),
                Expr::val(true),
            ]),
        )
        .expr(
            sea_orm::sea_query::Func::cust("pg_catalog.set_config").args([
                Expr::val("med_tracker.current_household_id"),
                Expr::val(""),
                Expr::val(true),
            ]),
        )
        .expr(
            sea_orm::sea_query::Func::cust("pg_catalog.set_config").args([
                Expr::val("med_tracker.personal_history_access"),
                Expr::val("true"),
                Expr::val(true),
            ]),
        )
        .to_owned();
    transaction
        .execute_raw(transaction.get_database_backend().build(&context))
        .await?;
    assert!(
        med_tracker::models::_entities::medications::Entity::find()
            .all(&transaction)
            .await?
            .is_empty()
    );
    assert!(
        person_medications::Entity::find()
            .all(&transaction)
            .await?
            .is_empty()
    );
    assert!(
        medication_takes::Entity::find()
            .all(&transaction)
            .await?
            .is_empty()
    );
    let own_history = sea_orm::sea_query::Query::select()
        .column(sea_orm::sea_query::Asterisk)
        .from("personal_taken_doses")
        .to_owned();
    let rows = transaction
        .query_all_raw(transaction.get_database_backend().build(&own_history))
        .await?;
    assert_eq!(rows.len(), 2);
    for row in rows {
        assert_eq!(
            row.try_get::<i64>("", "person_id")?,
            fixture.account.person_id
        );
    }
    transaction.rollback().await?;
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn household_free_profile_uses_account_identity_without_a_household() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    entrant(&fixture).await?;
    let mut request = server(&boot);
    request.save_cookies();
    entrant_browser_session(&fixture, &mut request, &boot).await;
    let page = request.get("/profile").await;
    assert_eq!(page.status_code(), 200, "{}", page.text());
    assert!(page.text().contains(&fixture.foreign.account.email));
    assert!(!page.text().contains(&fixture.account.email));
    let empty_history = request.get("/profile/history").await;
    assert_eq!(empty_history.status_code(), 200, "{}", empty_history.text());
    let html = page.text();
    let csrf = authenticity_token(&html).ok_or("profile form token missing")?;
    let saved = request
        .post("/profile")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("authenticity_token", csrf),
            ("setting", "clinical"),
            ("name", "Own clinical name"),
            ("date_of_birth", "1990-02-03"),
        ])
        .await;
    assert_eq!(saved.status_code(), 303, "{}", saved.text());
    let person = people::Entity::find_by_id(fixture.foreign.account.person_id)
        .one(&fixture.admin)
        .await?
        .ok_or("clinical person missing")?;
    assert_eq!(person.name, "Own clinical name");
    assert_eq!(person.date_of_birth.unwrap().to_string(), "1990-02-03");
    let account = accounts::Entity::find_by_id(fixture.foreign.account.id)
        .one(&fixture.admin)
        .await?
        .ok_or("account missing")?;
    assert_eq!(account.name, fixture.foreign.account.name);
    let csrf_denied = request
        .post("/profile")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("setting", "time_zone"), ("time_zone", "UTC")])
        .await;
    assert_eq!(csrf_denied.status_code(), 403);
    let injection = request
        .post("/profile")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("authenticity_token", csrf),
            ("setting", "clinical"),
            ("name", "Forbidden target"),
            ("person_id", &fixture.account.person_id.to_string()),
        ])
        .await;
    assert_eq!(injection.status_code(), 422, "{}", injection.text());
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn household_free_landing_keeps_the_opened_invitation_available() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    entrant(&fixture).await?;
    let invitation_token = format!("personal-landing-{}", uuid::Uuid::new_v4());
    invitation(&fixture, &invitation_token).await?;
    let mut request = server(&boot);
    request.save_cookies();
    entrant_browser_session(&fixture, &mut request, &boot).await;
    let preview = request
        .get(&format!("/invitations/accept?token={invitation_token}"))
        .await;
    assert_eq!(preview.status_code(), 200, "{}", preview.text());
    let landing = request.get("/households/choose").await;
    assert_eq!(landing.status_code(), 200, "{}", landing.text());
    assert!(
        landing
            .text()
            .contains(&format!("/invitations/accept?token={invitation_token}"))
    );
    assert!(landing.text().contains("href=\"/profile\""));
    assert!(landing.text().contains("action=\"/households\""));
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn household_free_clinical_birth_date_cannot_lock_out_the_account() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let original = people::Entity::find_by_id(fixture.account.person_id)
        .one(&fixture.admin)
        .await?
        .ok_or("clinical person missing")?;
    let versions = || {
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("Person"))
            .filter(versions::Column::ItemId.eq(original.id))
    };
    let before = versions().count(&fixture.admin).await?;
    let mut request = server(&boot);
    request.save_cookies();
    browser_session(&fixture, &mut request, &boot).await;
    for date in ["2015-01-01", "2099-01-01"] {
        let page = request.get("/profile").await;
        assert_eq!(page.status_code(), 200, "{}", page.text());
        let html = page.text();
        let csrf = authenticity_token(&html).ok_or("profile token missing")?;
        let invalid = request
            .post("/profile")
            .add_header("sec-fetch-site", "same-origin")
            .form(&[
                ("authenticity_token", csrf),
                ("setting", "clinical"),
                ("name", "Must not persist"),
                ("date_of_birth", date),
            ])
            .await;
        assert_eq!(invalid.status_code(), 422, "{}", invalid.text());
        assert!(invalid.text().contains(date));
        let unchanged = people::Entity::find_by_id(original.id)
            .one(&fixture.admin)
            .await?
            .ok_or("clinical person missing")?;
        assert_eq!(unchanged, original);
        assert_eq!(versions().count(&fixture.admin).await?, before);
        let still_signed_in = request.get("/profile").await;
        assert_eq!(
            still_signed_in.status_code(),
            200,
            "{}",
            still_signed_in.text()
        );
    }
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn household_selection_requires_csrf_and_does_not_change_session() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let household_id = fixture.household.id;
    let household_text = household_id.to_string();

    let mut request = server(&boot);
    request.save_cookies();
    browser_session(&fixture, &mut request, &boot).await;
    let rejected = request
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("household_id", household_text.as_str()),
            ("authenticity_token", "invalid"),
        ])
        .await;
    assert_eq!(rejected.status_code(), 403, "{}", rejected.text());
    assert_eq!(rejected.header("cache-control"), "no-store");
    let missing = request
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("household_id", household_text.as_str())])
        .await;
    assert!(
        missing.status_code().is_client_error(),
        "{}",
        missing.text()
    );
    assert_eq!(missing.header("cache-control"), "no-store");
    let home = request.get("/households/choose").await;
    let html = home.text();
    let token = authenticity_token(&html).ok_or("household form token missing")?;
    let foreign = request
        .post("/households/select")
        .add_header("origin", "https://foreign.example.test")
        .add_header("sec-fetch-site", "cross-site")
        .form(&[
            ("household_id", household_text.as_str()),
            ("authenticity_token", token),
        ])
        .await;
    assert_eq!(foreign.status_code(), 403, "{}", foreign.text());
    assert_eq!(foreign.header("cache-control"), "no-store");
    let missing_origin = request
        .post("/households/select")
        .add_header("sec-fetch-site", "cross-site")
        .form(&[
            ("household_id", household_text.as_str()),
            ("authenticity_token", token),
        ])
        .await;
    assert_eq!(
        missing_origin.status_code(),
        403,
        "{}",
        missing_origin.text()
    );
    assert_eq!(missing_origin.header("cache-control"), "no-store");
    assert_eq!(selected_count(&fixture).await?, 0);
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn household_selection_requires_current_account_and_session() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let household_id = fixture.household.id;
    let household_text = household_id.to_string();

    let mut request = server(&boot);
    request.save_cookies();
    browser_session(&fixture, &mut request, &boot).await;
    let home = request.get("/households/choose").await;
    let token = authenticity_token(&home.text())
        .ok_or("household form token missing")?
        .to_owned();
    for expired in [true, false] {
        identity_session::Entity::update_many()
            .col_expr(
                identity_session::Column::ExpiresAt,
                Expr::value(
                    chrono::Utc::now() + chrono::Duration::seconds(if expired { -1 } else { 3600 }),
                ),
            )
            .filter(identity_session::Column::UserId.eq(fixture.account.id))
            .exec(&fixture.admin)
            .await?;
        if !expired {
            accounts::Entity::update_many()
                .col_expr(accounts::Column::Status, Expr::value(1))
                .filter(accounts::Column::Id.eq(fixture.account.id))
                .exec(&fixture.admin)
                .await?;
        }
        let denied = request
            .post("/households/select")
            .add_header("sec-fetch-site", "same-origin")
            .form(&[
                ("household_id", household_text.as_str()),
                ("authenticity_token", token.as_str()),
            ])
            .await;
        assert!(denied.status_code().is_client_error(), "{}", denied.text());
        assert_eq!(denied.header("cache-control"), "no-store");
    }
    assert_eq!(selected_count(&fixture).await?, 0);
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn household_selection_is_independent_for_two_browser_sessions() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let household_id = fixture.household.id;
    let household_text = household_id.to_string();
    let slug = fixture.household.slug.clone();

    let (additional, _additional_member) = home(
        &fixture.scenario,
        "Second household",
        "second-household",
        "member",
        false,
    )
    .await?;
    let additional_slug = additional.slug.clone();
    let additional_text = additional.id.to_string();
    let mut first = server(&boot);
    first.save_cookies();
    let mut second = server(&boot);
    second.save_cookies();
    browser_session(&fixture, &mut first, &boot).await;
    browser_session(&fixture, &mut second, &boot).await;

    let first_home = first.get("/households/choose").await;
    let first_html = first_home.text();
    let first_token =
        authenticity_token(&first_html).ok_or("first household form token missing")?;
    let first_selected = first
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("household_id", household_text.as_str()),
            ("authenticity_token", first_token),
        ])
        .await;
    assert_eq!(
        first_selected.status_code(),
        303,
        "{}",
        first_selected.text()
    );
    assert_eq!(
        first_selected.header("location"),
        format!("/households/{slug}/dashboard").as_str()
    );
    let first_home_after_selection = first.get("/").await;
    assert_eq!(
        first_home_after_selection.status_code(),
        303,
        "{}",
        first_home_after_selection.text()
    );
    assert_eq!(
        first_home_after_selection.header("location"),
        format!("/households/{slug}/dashboard").as_str()
    );
    let first_dashboard = first
        .get(format!("/households/{slug}/dashboard").as_str())
        .await;
    assert_eq!(
        first_dashboard.status_code(),
        200,
        "{}",
        first_dashboard.text()
    );
    let navigation = first
        .get(format!("/households/{slug}/profile/navigation").as_str())
        .await;
    assert_eq!(navigation.status_code(), 200, "{}", navigation.text());
    assert_eq!(navigation.header("cache-control"), "no-store");
    let shell = &navigation.json::<Value>()["shell"];
    assert_eq!(shell["active_household_id"], household_text.as_str());
    assert_eq!(shell["households"].as_array().map(Vec::len), Some(2));
    assert!(shell["households"].as_array().is_some_and(|households| {
        households.iter().any(|household| {
            household["id"] == additional.id && household["name"] == "Second household"
        })
    }));

    let second_home = second.get("/households/choose").await;
    let second_html = second_home.text();
    let second_token =
        authenticity_token(&second_html).ok_or("second household form token missing")?;
    let second_selected = second
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("household_id", additional_text.as_str()),
            ("authenticity_token", second_token),
        ])
        .await;
    assert_eq!(
        second_selected.status_code(),
        303,
        "{}",
        second_selected.text()
    );
    assert_eq!(
        second_selected.header("location"),
        format!("/households/{additional_slug}/dashboard").as_str()
    );
    let second_home_after_selection = second.get("/").await;
    assert_eq!(
        second_home_after_selection.status_code(),
        303,
        "{}",
        second_home_after_selection.text()
    );
    assert_eq!(
        second_home_after_selection.header("location"),
        format!("/households/{additional_slug}/dashboard").as_str()
    );
    let second_inventory = second
        .get(format!("/households/{additional_slug}/medications").as_str())
        .await;
    assert_eq!(
        second_inventory.status_code(),
        200,
        "{}",
        second_inventory.text()
    );

    let first_session = first.get("/api/auth/get-session").await;
    let second_session = second.get("/api/auth/get-session").await;
    assert_eq!(first_session.status_code(), 200, "{}", first_session.text());
    assert_eq!(
        second_session.status_code(),
        200,
        "{}",
        second_session.text()
    );
    let selections = identity_session::Entity::find()
        .filter(identity_session::Column::UserId.eq(fixture.account.id))
        .filter(identity_session::Column::Active.eq(true))
        .all(&fixture.admin)
        .await?;
    assert_eq!(
        selections
            .iter()
            .filter(|row| row.active_household_id == Some(fixture.household.id))
            .count(),
        1
    );
    assert_eq!(
        selections
            .iter()
            .filter(|row| row.active_household_id == Some(additional.id))
            .count(),
        1
    );
    drop(first);
    drop(second);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn household_selection_rechecks_membership_and_household_state() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let household_id = fixture.household.id;
    let household_text = household_id.to_string();

    let (additional, additional_member) = home(
        &fixture.scenario,
        "Another household",
        "another-household",
        "member",
        false,
    )
    .await?;
    let _additional_slug = additional.slug.clone();
    let _additional_text = additional.id.to_string();
    let mut request = server(&boot);
    request.save_cookies();
    browser_session(&fixture, &mut request, &boot).await;
    let home = request.get("/households/choose").await;
    let html = home.text();
    let token = authenticity_token(&html)
        .ok_or("household form token missing")?
        .to_owned();
    let selected = request
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("household_id", household_text.as_str()),
            ("authenticity_token", &token),
        ])
        .await;
    assert_eq!(selected.status_code(), 303, "{}", selected.text());

    for (id, state) in [
        (i64::MAX.to_string(), 0),
        ("not-a-number".into(), 0),
        ("0".into(), 0),
        ("-1".into(), 0),
        (fixture.foreign.household.id.to_string(), 0),
        (additional.id.to_string(), 1),
        (additional.id.to_string(), 2),
        (additional.id.to_string(), 3),
    ] {
        if state == 1 {
            households::Entity::update_many()
                .col_expr(households::Column::LifecycleState, Expr::value("held"))
                .filter(households::Column::Id.eq(additional.id))
                .exec(&fixture.admin)
                .await?;
        } else if state == 2 {
            households::Entity::update_many()
                .col_expr(households::Column::LifecycleState, Expr::value("active"))
                .filter(households::Column::Id.eq(additional.id))
                .exec(&fixture.admin)
                .await?;
            household_memberships::Entity::update_many()
                .col_expr(
                    household_memberships::Column::Status,
                    Expr::value("suspended"),
                )
                .filter(household_memberships::Column::Id.eq(additional_member.id))
                .exec(&fixture.admin)
                .await?;
        } else if state == 3 {
            household_memberships::Entity::update_many()
                .col_expr(
                    household_memberships::Column::Status,
                    Expr::value("revoked"),
                )
                .col_expr(
                    household_memberships::Column::RevokedAt,
                    Expr::value(chrono::Utc::now().naive_utc()),
                )
                .filter(household_memberships::Column::Id.eq(additional_member.id))
                .exec(&fixture.admin)
                .await?;
        }
        let rejected = request
            .post("/households/select")
            .add_header("sec-fetch-site", "same-origin")
            .form(&[
                ("household_id", id.as_str()),
                ("authenticity_token", &token),
            ])
            .await;
        assert!(
            rejected.status_code().is_client_error(),
            "{id}: {} {}",
            rejected.status_code(),
            rejected.text()
        );
        assert_eq!(rejected.header("cache-control"), "no-store");
    }
    let selections = identity_session::Entity::find()
        .filter(identity_session::Column::UserId.eq(fixture.account.id))
        .filter(identity_session::Column::Active.eq(true))
        .all(&fixture.admin)
        .await?;
    assert_eq!(
        selections
            .iter()
            .filter(|row| row.active_household_id == Some(fixture.household.id))
            .count(),
        1
    );
    assert_eq!(
        selections
            .iter()
            .filter(|row| row
                .active_household_id
                .is_some_and(|id| id != fixture.household.id))
            .count(),
        0
    );
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn household_revocation_serializes_with_selection() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let household_id = fixture.household.id;
    let household_text = household_id.to_string();

    let mut request = server(&boot);
    request.save_cookies();
    browser_session(&fixture, &mut request, &boot).await;
    let home = request.get("/households/choose").await;
    let token = authenticity_token(&home.text())
        .ok_or("household form token missing")?
        .to_owned();

    let blocker = fixture.admin.begin().await?;
    let blocker_pid: i32 = blocker
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_backend_pid() AS pid",
        ))
        .await?
        .ok_or("blocker backend must exist")?
        .try_get("", "pid")?;
    household_memberships::Entity::find_by_id(fixture.membership.id)
        .lock_exclusive()
        .one(&blocker)
        .await?;
    let selection = async {
        request
            .post("/households/select")
            .add_header("sec-fetch-site", "same-origin")
            .form(&[
                ("household_id", household_text.as_str()),
                ("authenticity_token", token.as_str()),
            ])
            .await
    };
    let release = async {
        let observed = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let row = fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                    "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE '%household_memberships%' AND query LIKE '%FOR SHARE%' AND $1::int=ANY(pg_blocking_pids(pid))) AS waiting",
                    [blocker_pid.into()],
                )).await?.ok_or("lock observation must return one row")?;
                if row.try_get::<bool>("", "waiting")? {
                    break Ok::<_,Box<dyn std::error::Error>>(true);
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }).await.unwrap_or(Ok(false))?;
        household_memberships::Entity::update_many()
            .col_expr(
                household_memberships::Column::Status,
                Expr::value("revoked"),
            )
            .col_expr(
                household_memberships::Column::RevokedAt,
                Expr::value(chrono::Utc::now().naive_utc()),
            )
            .filter(household_memberships::Column::Id.eq(fixture.membership.id))
            .exec(&blocker)
            .await?;
        blocker.commit().await?;
        Ok::<_, Box<dyn std::error::Error>>(observed)
    };
    let (denied, observed) = tokio::join!(selection, release);
    assert!(
        observed?,
        "selection must wait on the revoking membership row; actual HTTP {}",
        denied.status_code()
    );
    assert!(denied.status_code().is_client_error(), "{}", denied.text());
    assert_eq!(denied.header("cache-control"), "no-store");
    assert_eq!(selected_count(&fixture).await?, 0);
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[path = "household_selection/personal_review.rs"]
mod personal_review;
