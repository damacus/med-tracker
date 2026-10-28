use super::*;
use base64::Engine as _;
use med_tracker::models::_entities::{
    account_lockouts, accounts, household_invitations, household_memberships, households, people,
    platform_admins, schedules, security_audit_events, versions,
};
use sea_orm::sea_query::{Expr, Func, Query};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter,
    QuerySelect, Set, TransactionTrait,
};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

pub(super) fn authenticity_token(html: &str) -> Option<&str> {
    html.split("name=\"authenticity_token\" value=\"")
        .nth(1)?
        .split('"')
        .next()
}

fn listed_member_names(html: &str) -> Vec<String> {
    let document = scraper::Html::parse_document(html);
    let selector =
        scraper::Selector::parse(".admin-member-directory tbody th[scope='row'] a[href]")
            .expect("static member-name selector");
    document
        .select(&selector)
        .map(|link| link.text().collect::<String>().trim().to_owned())
        .collect()
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

pub(super) async fn select_household(
    request: &mut TestServer,
    fixture: &RequestFixture,
    boot: &BootResult,
) -> TestResult {
    install_browser_session(request, boot, fixture.account.id).await;
    let chooser = request.get("/households/choose").await;
    let chooser_html = chooser.text();
    let token = authenticity_token(&chooser_html).ok_or("household form token missing")?;
    let selected = request
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("household_id", fixture.household.id.to_string().as_str()),
            ("authenticity_token", token),
        ])
        .await;
    assert_eq!(selected.status_code(), 303, "{}", selected.text());
    Ok(())
}

struct DirectoryIdentity {
    person: people::Model,
    account: Option<accounts::Model>,
    membership: household_memberships::Model,
    email: String,
}
async fn directory_identity(
    scenario: &Scenario,
    name: &str,
    account_name: Option<Option<&str>>,
    email_label: &str,
    role: &str,
    status: &str,
    person_type: i32,
) -> Result<DirectoryIdentity, sea_orm::DbErr> {
    let now = chrono::Utc::now().naive_utc();
    let email = format!(
        "{email_label}-{}@{}.example.test",
        uuid::Uuid::new_v4(),
        scenario.account.id
    );
    let person = people::ActiveModel {
        name: Set(name.into()),
        email: Set(Some(email.clone())),
        person_type: Set(person_type),
        has_capacity: Set(person_type == 0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&scenario.admin)
    .await?;
    let account = if let Some(account_name) = account_name {
        Some(
            accounts::ActiveModel {
                person_id: Set(person.id),
                email: Set(email.clone()),
                name: Set(account_name.map(str::to_owned)),
                status: Set(2),
                created_at: Set(now.and_utc()),
                updated_at: Set(now.and_utc()),
                ..Default::default()
            }
            .insert(&scenario.admin)
            .await?,
        )
    } else {
        None
    };
    let membership = household_memberships::ActiveModel {
        household_id: Set(scenario.household.id),
        person_id: Set(person.id),
        role: Set(role.into()),
        status: Set(status.into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&scenario.admin)
    .await?;
    Ok(DirectoryIdentity {
        person,
        account,
        membership,
        email,
    })
}
async fn new_home(
    scenario: &Scenario,
    name: &str,
) -> Result<(households::Model, household_memberships::Model), sea_orm::DbErr> {
    super::household_selection::home(scenario, name, "request-home", "owner", false).await
}
async fn invite(
    scenario: &Scenario,
    household_id: i64,
    inviter: i64,
    label: &str,
    role: &str,
    expired: bool,
) -> Result<household_invitations::Model, sea_orm::DbErr> {
    let now = chrono::Utc::now().naive_utc();
    household_invitations::ActiveModel {
        household_id: Set(household_id),
        email: Set(format!(
            "{label}@{household_id}.{}.example.test",
            scenario.account.id
        )),
        membership_role: Set(role.into()),
        token_digest: Set(uuid::Uuid::new_v4().to_string()),
        invited_by_membership_id: Set(inviter),
        expires_at: Set(now + chrono::Duration::days(if expired { -1 } else { 7 })),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&scenario.admin)
    .await
}
async fn event(
    scenario: &Scenario,
    household_id: i64,
    kind: &str,
    actors: (bool, bool),
    metadata: Value,
    context: Value,
    at: chrono::NaiveDateTime,
) -> Result<security_audit_events::Model, sea_orm::DbErr> {
    let (actor, member_actor) = actors;
    security_audit_events::ActiveModel {
        household_id: Set(household_id),
        actor_account_id: Set(actor.then_some(scenario.account.id)),
        actor_membership_id: Set(member_actor.then_some(scenario.membership.id)),
        event_type: Set(kind.into()),
        request_id: Set(Some(uuid::Uuid::new_v4().to_string())),
        metadata: Set(metadata),
        audit_context: Set(context),
        created_at: Set(at),
        updated_at: Set(at),
        ..Default::default()
    }
    .insert(&scenario.admin)
    .await
}
async fn audit_total(fixture: &RequestFixture) -> Result<u64, sea_orm::DbErr> {
    security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id))
        .count(&fixture.admin)
        .await
}
async fn attempt_total(fixture: &RequestFixture) -> Result<i64, sea_orm::DbErr> {
    let query = {
        use sea_orm::sea_query::ExprTrait;
        Query::select()
            .expr_as(Expr::col("id").count(), "total")
            .from("invitation_mail_attempts")
            .and_where(Expr::col("household_id").eq(fixture.household.id))
            .to_owned()
    };
    fixture
        .admin
        .query_one_raw(fixture.admin.get_database_backend().build(&query))
        .await?
        .unwrap()
        .try_get("", "total")
}
async fn invitation_state(
    fixture: &RequestFixture,
    id: i64,
) -> Result<(String, i64, u64), sea_orm::DbErr> {
    let invitation = household_invitations::Entity::find_by_id(id)
        .one(&fixture.admin)
        .await?
        .unwrap();
    let attempts = attempt_total(fixture).await?;
    let versions = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(fixture.household.id))
        .filter(versions::Column::ItemType.eq("HouseholdInvitation"))
        .count(&fixture.admin)
        .await?;
    Ok((invitation.token_digest, attempts, versions))
}
async fn set_scope(
    transaction: &sea_orm::DatabaseTransaction,
    fixture: &RequestFixture,
    household_id: i64,
) -> Result<(), sea_orm::DbErr> {
    let query = Query::select()
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.current_account_id"),
            Expr::val(fixture.account.id.to_string()),
            Expr::val(true),
        ]))
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.current_membership_id"),
            Expr::val(fixture.membership.id.to_string()),
            Expr::val(true),
        ]))
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.current_household_id"),
            Expr::val(household_id.to_string()),
            Expr::val(true),
        ]))
        .to_owned();
    transaction
        .execute_raw(transaction.get_database_backend().build(&query))
        .await?;
    Ok(())
}

#[tokio::test]
async fn member_administration_preserves_inactive_accountless_directory_identity() -> TestResult {
    let (fixture, boot) = boot_request_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let slug = fixture.household.slug.clone();
    let zulu = directory_identity(
        &fixture.scenario,
        "Zulu accountless",
        None,
        "zulu",
        "member",
        "suspended",
        0,
    )
    .await?;
    let subject_member_id = zulu.membership.id;
    let subject_person_id = zulu.person.id;
    let alpha = directory_identity(
        &fixture.scenario,
        "Alpha accountless",
        None,
        "alpha",
        "member",
        "revoked",
        0,
    )
    .await?;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&mut request, &fixture, &boot).await?;
    for (query, expected) in [
        (
            "q=ACCOUNTLESS&sort=name&direction=asc",
            vec![alpha.membership.id, subject_member_id],
        ),
        (
            "q=ACCOUNTLESS&sort=name&direction=desc",
            vec![subject_member_id, alpha.membership.id],
        ),
        (
            "q=example.test&status=revoked&sort=email",
            vec![alpha.membership.id],
        ),
        (
            format!(
                "{}&sort=email",
                url::form_urlencoded::Serializer::new(String::new())
                    .append_pair("q", &zulu.email)
                    .finish()
            )
            .as_str(),
            vec![subject_member_id],
        ),
    ] {
        let response = request
            .get(&format!("/households/{slug}/admin/users?{query}"))
            .await;
        assert_eq!(response.status_code(), 200, "{}", response.text());
        assert_eq!(
            listed_membership_ids(&response.text()),
            expected,
            "{}",
            response.text()
        );
    }
    let editor = request
        .get(format!("/households/{slug}/admin/memberships/{subject_member_id}/edit").as_str())
        .await;
    assert_eq!(editor.status_code(), 200, "{}", editor.text());
    assert!(
        editor.text().contains("Zulu accountless"),
        "{}",
        editor.text()
    );
    let person = request
        .get(format!("/households/{slug}/people/{subject_person_id}").as_str())
        .await;
    assert_eq!(person.status_code(), 404, "{}", person.text());
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn member_administration_directory_projection_checks_live_scope_and_role() -> TestResult {
    let (fixture, boot) = boot_request_fixture().await;

    let retained = directory_identity(
        &fixture.scenario,
        "Retained identity",
        None,
        "retained",
        "member",
        "revoked",
        0,
    )
    .await?;
    let _subject_member_id = retained.membership.id;
    let _subject_person_id = retained.person.id;
    let privileges = fixture.runtime.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT p.prosecdef,p.proconfig::text AS settings,pg_get_function_result(p.oid) AS result,has_function_privilege('med_tracker_app',p.oid,'EXECUTE') AS app_execute,has_function_privilege('med_tracker_app','med_tracker.can_read_member_directory()','EXECUTE') AS helper_execute,EXISTS(SELECT 1 FROM aclexplode(p.proacl) acl WHERE acl.grantee=0 AND acl.privilege_type='EXECUTE') AS public_execute FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='med_tracker' AND p.proname='member_directory_identity'"
    )).await?.ok_or("projection missing")?;
    assert!(privileges.try_get::<bool>("", "prosecdef")?);
    assert_eq!(
        privileges.try_get::<String>("", "settings")?,
        "{search_path=pg_catalog}"
    );
    assert_eq!(
        privileges.try_get::<String>("", "result")?,
        "TABLE(membership_id bigint, name text, email text)"
    );
    assert!(privileges.try_get::<bool>("", "app_execute")?);
    assert!(!privileges.try_get::<bool>("", "helper_execute")?);
    assert!(!privileges.try_get::<bool>("", "public_execute")?);
    for (household_id, permitted) in [
        (fixture.household.id, true),
        (fixture.foreign.household.id, false),
    ] {
        let transaction = fixture.runtime.begin().await?;
        transaction
            .execute_unprepared("SET LOCAL ROLE med_tracker_app")
            .await?;
        set_scope(&transaction, &fixture, household_id).await?;
        let hidden = people::Entity::find_by_id(retained.person.id)
            .all(&transaction)
            .await?;
        assert!(hidden.is_empty());
        let directory=transaction.query_all_raw(Statement::from_sql_and_values(fixture.runtime.get_database_backend(),"SELECT name,email FROM med_tracker.member_directory_identity() WHERE membership_id=$1",[retained.membership.id.into()])).await;
        if permitted {
            let rows = directory?;
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].try_get::<String>("", "name")?, "Retained identity");
        } else {
            assert!(directory.is_err());
        }
        transaction.rollback().await?;
    }
    for state in 0..4 {
        match state {
            0 => fixture.member().await,
            1 => {
                let mut member = fixture.membership.clone().into_active_model();
                member.role = Set("owner".into());
                member.status = Set("revoked".into());
                member.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
                member.update(&fixture.admin).await?;
            }
            2 => {
                let mut member = fixture.membership.clone().into_active_model();
                member.role = Set("owner".into());
                member.status = Set("active".into());
                member.revoked_at = Set(None);
                member.update(&fixture.admin).await?;
                account_lockouts::ActiveModel {
                    account_id: Set(fixture.account.id),
                    key: Set(fixture.credential("directory-lockout")),
                    deadline: Set((chrono::Utc::now() + chrono::Duration::hours(1)).naive_utc()),
                    ..Default::default()
                }
                .insert(&fixture.admin)
                .await?;
            }
            _ => {
                account_lockouts::Entity::delete_by_id(fixture.account.id)
                    .exec(&fixture.admin)
                    .await?;
                households::Entity::update_many()
                    .col_expr(
                        households::Column::LifecycleState,
                        Expr::value("deletion_requested"),
                    )
                    .filter(households::Column::Id.eq(fixture.household.id))
                    .exec(&fixture.admin)
                    .await?;
            }
        }
        let transaction = fixture.runtime.begin().await?;
        transaction
            .execute_unprepared("SET LOCAL ROLE med_tracker_app")
            .await?;
        set_scope(&transaction, &fixture, fixture.household.id).await?;
        assert!(
            transaction
                .query_all_raw(Statement::from_string(
                    DbBackend::Postgres,
                    "SELECT * FROM med_tracker.member_directory_identity()"
                ))
                .await
                .is_err()
        );
        transaction.rollback().await?;
    }
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn member_administration_lists_inactive_identity_and_changes_membership_state() -> TestResult
{
    let (fixture, boot) = boot_request_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let dependent_membership_id = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(fixture.household.id))
        .filter(household_memberships::Column::PersonId.eq(fixture.dependent.id))
        .one(&fixture.admin)
        .await?
        .unwrap()
        .id;
    let slug = fixture.household.slug.clone();
    let actor_member_id = fixture.membership.id;

    let user_id = fixture.membership.id;
    let minor_membership_id = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(fixture.household.id))
        .filter(household_memberships::Column::PersonId.eq(fixture.minor.id))
        .one(&fixture.admin)
        .await?
        .unwrap()
        .id;
    let other_member_id = fixture.foreign.membership.id;
    let inactive = directory_identity(
        &fixture.scenario,
        "Hidden person name",
        Some(Some("Inactive member")),
        "inactive",
        "member",
        "suspended",
        0,
    )
    .await?;
    let inactive_member_id = inactive.membership.id;
    let inactive_query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("q", &inactive.email.to_uppercase())
        .finish();
    let fallback = directory_identity(
        &fixture.scenario,
        "Other hidden person",
        Some(None),
        "fallback",
        "member",
        "suspended",
        0,
    )
    .await?;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&mut request, &fixture, &boot).await?;
    let list = request
        .get(format!("/households/{slug}/admin/users?status=suspended").as_str())
        .await;
    let html = list.text();
    assert_eq!(list.status_code(), 200, "{html}");
    assert!(html.contains("Inactive member"), "{html}");
    assert!(html.contains(inactive.email.as_str()), "{html}");
    assert!(html.contains(fallback.email.as_str()), "{html}");
    assert!(!html.contains("Hidden person name"), "{html}");
    let canonical_editor = request
        .get(format!("/households/{slug}/admin/users/{inactive_member_id}/edit").as_str())
        .await;
    assert_eq!(canonical_editor.status_code(), 200);
    assert!(
        canonical_editor
            .text()
            .contains("Edit household role for Inactive member")
    );
    let member_editor = request.get(format!("/households/{slug}/admin/memberships/{inactive_member_id}/edit?status=suspended&sort=email&direction=desc&page=1&lang=es").as_str()).await;
    assert_eq!(member_editor.status_code(), 200, "{}", member_editor.text());
    assert!(
        member_editor
            .text()
            .contains("Editar el rol de Inactive member en el hogar")
    );
    assert!(member_editor.text().contains("/admin/users?q=&amp;role=all&amp;status=suspended&amp;sort=email&amp;direction=desc&amp;page=1&amp;lang=es"));
    let editor_csrf = authenticity_token(&member_editor.text())
        .ok_or("editor token missing")?
        .to_owned();
    let stateful_save = request
        .post(
            format!("/households/{slug}/admin/memberships/{inactive_member_id}/membership_role")
                .as_str(),
        )
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
        format!("/households/{slug}/admin/users?q=&role=all&status=suspended&sort=email&direction=desc&page=1&lang=es").as_str()
    );
    assert!(html.contains("suspended"), "{html}");
    assert!(html.contains("Restore"), "{html}");
    let search = request.get(format!("/households/{slug}/admin/users?{inactive_query}&role=member&status=suspended&sort=email&direction=desc").as_str()).await;
    assert_eq!(search.status_code(), 200, "{}", search.text());
    assert!(search.text().contains(inactive.email.as_str()));
    assert!(!search.text().contains(fallback.email.as_str()));
    assert_eq!(
        request
            .get(
                format!("/households/{slug}/admin/users?sort=name%3BDROP%20TABLE%20accounts")
                    .as_str()
            )
            .await
            .status_code(),
        422
    );
    let role = request
        .get(format!("/households/{slug}/admin/memberships/{inactive_member_id}/edit").as_str())
        .await;
    assert_eq!(role.status_code(), 200, "{}", role.text());
    assert!(
        role.text()
            .contains("Edit household role for Inactive member")
    );
    assert_eq!(
        request
            .get(format!("/households/{slug}/admin/users/{user_id}/edit").as_str())
            .await
            .status_code(),
        200
    );
    let csrf = authenticity_token(&html).ok_or("member form token missing")?;
    let edited_role = request
        .post(
            format!("/households/{slug}/admin/memberships/{inactive_member_id}/membership_role")
                .as_str(),
        )
        .form(&[("role", "administrator"), ("authenticity_token", csrf)])
        .await;
    assert_eq!(edited_role.status_code(), 303, "{}", edited_role.text());
    let restored_role = request
        .post(
            format!("/households/{slug}/admin/memberships/{inactive_member_id}/membership_role")
                .as_str(),
        )
        .form(&[("role", "member"), ("authenticity_token", csrf)])
        .await;
    assert_eq!(restored_role.status_code(), 303, "{}", restored_role.text());
    let missing_csrf = request
        .post(format!("/households/{slug}/admin/users/{inactive_member_id}/restore").as_str())
        .form(&[("confirmation", "restored")])
        .await;
    assert_eq!(missing_csrf.status_code(), 403);
    let foreign = request
        .post(format!("/households/{slug}/admin/users/{other_member_id}/revoke").as_str())
        .form(&[("confirmation", "revoked"), ("authenticity_token", csrf)])
        .await;
    assert_eq!(foreign.status_code(), 404, "{}", foreign.text());
    let restored = request
        .post(format!("/households/{slug}/admin/users/{inactive_member_id}/restore").as_str())
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
    let active = household_memberships::Entity::find_by_id(inactive.membership.id)
        .one(&fixture.admin)
        .await?
        .unwrap();
    assert_eq!(active.status, "active");
    assert_eq!(active.permissions_version, 4);
    let active_list = request
        .get(format!("/households/{slug}/admin/users").as_str())
        .await;
    let active_html = active_list.text();
    let next_csrf = authenticity_token(&active_html).ok_or("active form token missing")?;
    let suspended = request
        .post(format!("/households/{slug}/admin/users/{inactive_member_id}/suspend").as_str())
        .form(&[
            ("confirmation", "suspended"),
            ("authenticity_token", next_csrf),
        ])
        .await;
    assert_eq!(suspended.status_code(), 303, "{}", suspended.text());
    assert_eq!(
        suspended.header("location").to_str()?,
        format!("/households/{slug}/admin/users?notice=suspended").as_str()
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
        .post(format!("/households/{slug}/admin/users/{inactive_member_id}/revoke").as_str())
        .form(&[
            ("confirmation", "revoked"),
            ("authenticity_token", next_csrf),
        ])
        .await;
    assert_eq!(revoked.status_code(), 303, "{}", revoked.text());
    let forbidden_suspend = request
        .post(format!("/households/{slug}/admin/users/{inactive_member_id}/suspend").as_str())
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
        .post(format!("/households/{slug}/admin/users/{inactive_member_id}/restore").as_str())
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
    household_memberships::Entity::find_by_id(inactive.membership.id)
        .lock_exclusive()
        .one(&blocker)
        .await?;
    let stale_suspend = async {
        request
            .post(format!("/households/{slug}/admin/users/{inactive_member_id}/suspend").as_str())
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
        household_memberships::Entity::update_many()
            .col_expr(
                household_memberships::Column::Status,
                Expr::value("revoked"),
            )
            .col_expr(
                household_memberships::Column::RevokedAt,
                Expr::value(chrono::Utc::now().naive_utc()),
            )
            .filter(household_memberships::Column::Id.eq(inactive.membership.id))
            .exec(&blocker)
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
    let raced_state = household_memberships::Entity::find_by_id(inactive.membership.id)
        .one(&fixture.admin)
        .await?
        .unwrap();
    assert_eq!(raced_state.status, "revoked");
    let events = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id))
        .all(&fixture.admin)
        .await?;
    let success = events
        .iter()
        .filter(|row| {
            row.metadata["target_membership_id"].as_i64() == Some(inactive.membership.id)
                && row.metadata["outcome"] == "success"
        })
        .count();
    assert!(success >= 4);
    let account = accounts::Entity::find_by_id(inactive.account.as_ref().unwrap().id)
        .one(&fixture.admin)
        .await?
        .unwrap();
    assert_eq!(account.status, 2);
    let mut bulk_members = Vec::new();
    for _ in 0..27 {
        bulk_members.push(
            directory_identity(
                &fixture.scenario,
                "Bulk member",
                None,
                "bulk",
                "member",
                "active",
                0,
            )
            .await?,
        );
    }
    let first_page = request.get(format!("/households/{slug}/admin/users?role=member&status=active&sort=name&direction=asc&page=1&lang=es").as_str()).await;
    let first_html = first_page.text();
    assert_eq!(first_page.status_code(), 200, "{first_html}");
    assert_eq!(
        listed_member_names(&first_html)
            .iter()
            .filter(|name| name.as_str() == "Bulk member")
            .count(),
        25
    );
    assert_eq!(
        listed_membership_ids(&first_html),
        bulk_members[..25]
            .iter()
            .map(|member| member.membership.id)
            .collect::<Vec<_>>()
    );
    assert!(first_html.contains("page=2&amp;lang=es"), "{first_html}");
    let second_page = request.get(format!("/households/{slug}/admin/users?role=member&status=active&sort=name&direction=asc&page=2&lang=es").as_str()).await;
    assert_eq!(second_page.status_code(), 200, "{}", second_page.text());
    assert_eq!(
        listed_member_names(&second_page.text())
            .iter()
            .filter(|name| name.as_str() == "Bulk member")
            .count(),
        2
    );
    assert_eq!(
        listed_membership_ids(&second_page.text()),
        vec![
            bulk_members[25].membership.id,
            bulk_members[26].membership.id,
            dependent_membership_id,
            minor_membership_id
        ]
    );
    household_memberships::Entity::update_many()
        .col_expr(household_memberships::Column::Role, Expr::value("owner"))
        .filter(household_memberships::Column::Id.eq(fixture.membership.id))
        .exec(&fixture.admin)
        .await?;
    let owner_page = request
        .get(format!("/households/{slug}/admin/users").as_str())
        .await;
    let owner_html = owner_page.text();
    let owner_csrf = authenticity_token(&owner_html).ok_or("owner form token missing")?;
    let last_owner = request
        .post(format!("/households/{slug}/admin/users/{actor_member_id}/suspend").as_str())
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
    let retained = household_memberships::Entity::find_by_id(fixture.membership.id)
        .one(&fixture.admin)
        .await?
        .unwrap();
    assert_eq!(retained.status, "active");
    household_memberships::Entity::update_many()
        .col_expr(household_memberships::Column::Role, Expr::value("member"))
        .filter(household_memberships::Column::Id.eq(fixture.membership.id))
        .exec(&fixture.admin)
        .await?;
    let before_denial = (
        household_memberships::Entity::find_by_id(inactive.membership.id)
            .one(&fixture.admin)
            .await?
            .unwrap()
            .status,
        audit_total(&fixture).await?,
    );
    let stale_manager = request
        .post(format!("/households/{slug}/admin/users/{inactive_member_id}/suspend").as_str())
        .form(&[
            ("confirmation", "suspended"),
            ("authenticity_token", owner_csrf),
        ])
        .await;
    assert_eq!(stale_manager.status_code(), 403, "{}", stale_manager.text());
    assert!(
        !stale_manager
            .text()
            .contains(format!("/admin/memberships/{inactive_member_id}/edit").as_str())
    );
    let unchanged = (
        household_memberships::Entity::find_by_id(inactive.membership.id)
            .one(&fixture.admin)
            .await?
            .unwrap()
            .status,
        audit_total(&fixture).await?,
    );
    assert_eq!(unchanged, before_denial);

    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn member_administration_orders_and_filters_exact_memberships() -> TestResult {
    let (fixture, boot) = boot_request_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let slug = fixture.household.slug.clone();
    let alpha = directory_identity(
        &fixture.scenario,
        "Hidden Alpha",
        Some(Some("Sortprobe Alpha")),
        "zeta-sortprobe",
        "owner",
        "active",
        0,
    )
    .await?;
    let bravo = directory_identity(
        &fixture.scenario,
        "Hidden Bravo",
        Some(Some("Sortprobe Bravo")),
        "delta-sortprobe",
        "administrator",
        "suspended",
        0,
    )
    .await?;
    let charlie = directory_identity(
        &fixture.scenario,
        "Hidden Charlie",
        Some(Some("Sortprobe Charlie")),
        "alpha-sortprobe",
        "member",
        "revoked",
        0,
    )
    .await?;
    let delta = directory_identity(
        &fixture.scenario,
        "Hidden Delta",
        Some(Some("Sortprobe Delta")),
        "beta-sortprobe",
        "member",
        "active",
        0,
    )
    .await?;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&mut request, &fixture, &boot).await?;
    let path = format!("/households/{slug}/admin/users?q=sortprobe");
    for (field, ascending, descending) in [
        (
            "name",
            vec![
                alpha.membership.id,
                bravo.membership.id,
                charlie.membership.id,
                delta.membership.id,
            ],
            vec![
                delta.membership.id,
                charlie.membership.id,
                bravo.membership.id,
                alpha.membership.id,
            ],
        ),
        (
            "email",
            vec![
                charlie.membership.id,
                delta.membership.id,
                bravo.membership.id,
                alpha.membership.id,
            ],
            vec![
                alpha.membership.id,
                bravo.membership.id,
                delta.membership.id,
                charlie.membership.id,
            ],
        ),
        (
            "role",
            vec![
                bravo.membership.id,
                charlie.membership.id,
                delta.membership.id,
                alpha.membership.id,
            ],
            vec![
                alpha.membership.id,
                charlie.membership.id,
                delta.membership.id,
                bravo.membership.id,
            ],
        ),
        (
            "status",
            vec![
                alpha.membership.id,
                delta.membership.id,
                charlie.membership.id,
                bravo.membership.id,
            ],
            vec![
                bravo.membership.id,
                charlie.membership.id,
                alpha.membership.id,
                delta.membership.id,
            ],
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
        (
            "role=member",
            vec![charlie.membership.id, delta.membership.id],
        ),
        (
            "status=active",
            vec![alpha.membership.id, delta.membership.id],
        ),
        ("status=suspended", vec![bravo.membership.id]),
        ("status=revoked", vec![charlie.membership.id]),
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
        .get(format!("/households/{slug}/admin/users?q=BRAVO").as_str())
        .await;
    assert_eq!(
        canonical_name.status_code(),
        200,
        "{}",
        canonical_name.text()
    );
    assert_eq!(
        listed_membership_ids(&canonical_name.text()),
        vec![bravo.membership.id]
    );
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn administration_overview_shows_only_current_household_status() -> TestResult {
    let (fixture, boot) = boot_request_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let slug = fixture.household.slug.clone();

    let person_id = fixture.account.person_id;
    let foreign_slug = fixture.foreign.household.slug.clone();
    let _pending = invite(
        &fixture.scenario,
        fixture.household.id,
        fixture.membership.id,
        "pending-admin",
        "member",
        false,
    )
    .await?;
    let _expired = invite(
        &fixture.scenario,
        fixture.household.id,
        fixture.membership.id,
        "expired-admin",
        "member",
        true,
    )
    .await?;
    let (other, other_membership) = new_home(&fixture.scenario, "Other household").await?;
    let other_slug = other.slug.clone();
    let now = chrono::Utc::now().naive_utc();
    let other_person = people::ActiveModel {
        name: Set("Other person".into()),
        person_type: Set(0),
        has_capacity: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    household_memberships::ActiveModel {
        household_id: Set(other.id),
        person_id: Set(other_person.id),
        role: Set("member".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    let other_invitation = invite(
        &fixture.scenario,
        other.id,
        other_membership.id,
        "other-admin",
        "member",
        false,
    )
    .await?;
    let now = chrono::Utc::now().naive_utc();
    let today = chrono::Utc::now().date_naive();
    for (active, start, end) in [
        (true, Some(today), Some(today)),
        (
            true,
            Some(today + chrono::Duration::days(1)),
            Some(today + chrono::Duration::days(3)),
        ),
        (false, None, None),
        (true, None, None),
    ] {
        schedules::ActiveModel {
            household_id: Set(fixture.household.id),
            person_id: Set(fixture.account.person_id),
            medication_id: Set(fixture.medication.id),
            active: Set(active),
            start_date: Set(start),
            end_date: Set(end),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&fixture.admin)
        .await?;
    }
    household_memberships::Entity::update_many()
        .col_expr(
            household_memberships::Column::JoinedAt,
            Expr::value(now - chrono::Duration::days(8)),
        )
        .filter(household_memberships::Column::Id.eq(fixture.membership.id))
        .exec(&fixture.admin)
        .await?;
    let cancelled = invite(
        &fixture.scenario,
        fixture.household.id,
        fixture.membership.id,
        "cancelled",
        "member",
        true,
    )
    .await?;
    let mut cancelled = cancelled.into_active_model();
    cancelled.revoked_at = Set(Some(now));
    cancelled.update(&fixture.admin).await?;
    let shared_person = accounts::Entity::find_by_id(fixture.account.id)
        .one(&fixture.admin)
        .await?
        .unwrap()
        .person_id;
    let shared_households = household_memberships::Entity::find()
        .filter(household_memberships::Column::PersonId.eq(shared_person))
        .filter(household_memberships::Column::HouseholdId.is_in([fixture.household.id, other.id]))
        .count(&fixture.admin)
        .await?;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&mut request, &fixture, &boot).await?;
    let overview = request
        .get(format!("/households/{slug}/admin").as_str())
        .await;
    let overview_status = overview.status_code();
    let overview_cache = overview.header("cache-control").to_owned();
    let html = overview.text();
    let selected_other = request
        .get(format!("/households/{other_slug}/admin").as_str())
        .await;
    let other_status = selected_other.status_code();
    let other_html = selected_other.text();
    let foreign = request
        .get(format!("/households/{foreign_slug}/admin").as_str())
        .await;
    let foreign_status = foreign.status_code().as_u16();
    household_memberships::Entity::update_many()
        .col_expr(household_memberships::Column::Role, Expr::value("member"))
        .filter(household_memberships::Column::Id.eq(fixture.membership.id))
        .exec(&fixture.admin)
        .await?;
    let denied = request
        .get(format!("/households/{slug}/admin").as_str())
        .await;
    let denied_status = denied.status_code();
    let mut row = fixture.membership.clone().into_active_model();
    row.role = Set("administrator".into());
    row.status = Set("revoked".into());
    row.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    row.update(&fixture.admin).await?;
    let revoked = request
        .get(format!("/households/{slug}/admin").as_str())
        .await;
    let revoked_status = revoked.status_code().as_u16();
    drop(request);
    close_request(fixture, boot).await;
    assert_eq!(overview_status, 200, "{html}");
    assert_eq!(overview_cache, "no-store");
    assert!(html.contains("Synthetic household administration"));
    assert!(html.contains("3 active members"));
    assert!(html.contains("3 people"));
    assert!(html.contains("1 pending invitation"));
    let document = scraper::Html::parse_document(&html);
    for (metric, count) in [
        ("expired-invitations", "1"),
        ("active-schedules", "1"),
        ("recent-memberships", "2"),
    ] {
        let selector = scraper::Selector::parse(&format!("[data-admin-metric='{metric}'] dd"))
            .map_err(|_| "metric selector")?;
        let text = document
            .select(&selector)
            .next()
            .ok_or("missing administration metric")?
            .text()
            .collect::<String>();
        assert_eq!(text.trim(), count, "{metric}");
    }
    assert!(html.contains("id=\"expired-invitations\""));
    assert!(html.contains(format!("href=\"/households/{slug}/admin/invitations\"").as_str()));
    assert!(!html.contains("Other household"));
    assert!(!html.contains(other_invitation.email.as_str()));
    assert_eq!(other_status, 200, "{other_html}");
    assert!(other_html.contains("Other household administration"));
    assert!(other_html.contains("2 active members"));
    assert!(other_html.contains("2 people"));
    assert!(other_html.contains("1 pending invitation"));
    assert!(!other_html.contains("Synthetic household"));
    assert_eq!(shared_person, person_id);
    assert_eq!(shared_households, 2);
    assert!([403, 404].contains(&foreign_status));
    assert_eq!(denied_status, 403);
    assert!([403, 404].contains(&revoked_status));
    Ok(())
}

#[tokio::test]
async fn administration_overview_shows_five_recent_scoped_safe_events() -> TestResult {
    let (fixture, boot) = boot_request_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let slug = fixture.household.slug.clone();
    let account_id = fixture.account.id;
    let actor_member_id = fixture.membership.id;
    let mut request = server(&boot);
    request.save_cookies();
    let mut recent = Vec::new();
    let mut foreign_recent = None;
    let result = async {
        select_household(&mut request, &fixture, &boot).await?;
        security_audit_events::Entity::delete_many().filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id)).exec(&fixture.admin).await?;
        let empty = request.get(format!("/households/{slug}/admin").as_str()).await;
        let empty_status = empty.status_code();
        let empty_html = empty.text();
        let stamp = "2030-01-15T12:00:00".parse().unwrap();
    for _ in 1..=6 { recent.push(event(&fixture.scenario,fixture.household.id,"household_membership.role_updated", (true, true),
        json!({"target_membership_id":fixture.membership.id,"new_role":"administrator","secret":"private-clinical-note"}),json!({"session_reference":"private-session"}),stamp).await?); }
    let (other,_) = new_home(&fixture.scenario,"Other household").await?;
    foreign_recent = Some(event(&fixture.scenario,other.id,"household_membership.role_updated", (false, false),
        json!({"secret":"foreign-clinical-note"}),json!({}),"2030-01-16T12:00:00".parse().unwrap()).await?);
        for days in [0,2] { event(&fixture.scenario,fixture.household.id,"api_request", (false, false),json!({}),json!({}),
        chrono::Utc::now().naive_utc()-chrono::Duration::days(days)).await?; }
        let populated = request.get(format!("/households/{slug}/admin").as_str()).await;
        let populated_status = populated.status_code();
        let populated_cache = populated.header("cache-control").to_owned();
        let populated_html = populated.text();
        Ok::<_, Box<dyn std::error::Error>>((empty_status, empty_html, populated_status, populated_cache, populated_html))
    }.await;
    drop(request);
    close_request(fixture, boot).await;
    let (empty_status, empty_html, populated_status, populated_cache, populated_html) = result?;
    assert_eq!(empty_status, 200, "{empty_html}");
    assert!(empty_html.contains("Recent activity"));
    assert!(empty_html.contains("No recent administrative activity"));
    assert_eq!(populated_status, 200, "{populated_html}");
    assert_eq!(populated_cache, "no-store");
    assert!(populated_html.contains("Recent activity"));
    let document = scraper::Html::parse_document(&populated_html);
    let metric = scraper::Selector::parse("[data-admin-metric='recent-audit-events'] dd")
        .map_err(|_| "audit metric selector")?;
    assert_eq!(
        document
            .select(&metric)
            .next()
            .ok_or("audit metric missing")?
            .text()
            .collect::<String>()
            .trim(),
        "1",
        "future and foreign events are outside the 24-hour window"
    );
    assert_eq!(populated_html.matches("View audit event").count(), 5);
    assert!(populated_html.contains(&format!("/admin/audit_logs/{}", recent[5].id)));
    assert!(!populated_html.contains(&format!("/admin/audit_logs/{}", recent[0].id)));
    assert!(!populated_html.contains(&format!("/admin/audit_logs/{}", foreign_recent.unwrap().id)));
    assert!(
        populated_html
            .contains(format!("Account #{account_id}, membership #{actor_member_id}").as_str())
    );
    assert!(populated_html.contains("Household role changed"));
    assert!(!populated_html.contains("private-clinical-note"));
    assert!(!populated_html.contains("private-session"));
    assert!(!populated_html.contains("foreign-clinical-note"));
    Ok(())
}

#[tokio::test]
async fn navigation_exposes_platform_tools_only_while_platform_authority_is_active() -> TestResult {
    let (fixture, boot) = boot_request_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let slug = fixture.household.slug.clone();

    let mut request = server(&boot);
    request.save_cookies();
    select_household(&mut request, &fixture, &boot).await?;
    let path = format!("/households/{slug}/profile/navigation");
    let ordinary = request.get(&path).await;
    let ordinary_status = ordinary.status_code();
    let ordinary_body = ordinary.text();
    let now = chrono::Utc::now().naive_utc();
    platform_admins::ActiveModel {
        account_id: Set(fixture.account.id),
        status: Set("active".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    let elevated = request.get(&path).await;
    let elevated_status = elevated.status_code();
    let elevated_body = elevated.text();
    platform_admins::Entity::update_many()
        .col_expr(platform_admins::Column::Status, Expr::value("disabled"))
        .filter(platform_admins::Column::AccountId.eq(fixture.account.id))
        .exec(&fixture.admin)
        .await?;
    let revoked = request.get(&path).await;
    let revoked_status = revoked.status_code();
    let revoked_body = revoked.text();
    drop(request);
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let slug = fixture.household.slug.clone();
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&mut request, &fixture, &boot).await?;
    let mut results = Vec::new();
    for language in ["en", "cy", "ga", "pt", "es"] {
        let response = request
            .get(&format!("/households/{slug}/admin?lang={language}"))
            .add_header("accept-language", "en")
            .await;
        results.push((language, response.status_code(), response.text()));
    }
    let header = request
        .get(format!("/households/{slug}/admin").as_str())
        .add_header("accept-language", "fr,es;q=0.9,en;q=0.1")
        .await
        .text();
    let invalid = request
        .get(format!("/households/{slug}/admin?lang=es&lang=en").as_str())
        .await
        .status_code();
    drop(request);
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let slug = fixture.household.slug.clone();
    let user_id = fixture.membership.id;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&mut request, &fixture, &boot).await?;
    let user_edit_path = format!("users/{user_id}/edit");
    let mut results = Vec::new();
    for language in ["en", "cy", "ga", "pt", "es"] {
        for path in [
            "users",
            user_edit_path.as_str(),
            "household/edit",
            "carer_relationships",
            "carer_relationships/new",
            "invitations",
        ] {
            let response = request
                .get(&format!("/households/{slug}/admin/{path}?lang={language}"))
                .add_header("accept-language", "cy")
                .await;
            results.push((language, path, response.status_code(), response.text()));
        }
    }
    drop(request);
    close_request(fixture, boot).await;
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
            if link.starts_with(format!("/households/{slug}/admin").as_str()) {
                assert!(
                    link.ends_with(&format!("?lang={language}")),
                    "{language} {path}: {link}"
                );
            }
        }
        if html.contains(format!("action=\"/households/{slug}/admin/").as_str()) {
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
    let (fixture, boot) = boot_request_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let slug = fixture.household.slug.clone();
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&mut request, &fixture, &boot).await?;
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
                .get(&format!("/households/{slug}/{path}"))
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
    close_request(fixture, boot).await;
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

#[tokio::test]
async fn member_detail_scopes_safe_history_and_preserves_navigation() -> TestResult {
    let (fixture, boot) = boot_request_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let slug = fixture.household.slug.clone();
    let account_id = fixture.account.id;
    let actor_member_id = fixture.membership.id;
    let absent_id = i64::MAX;
    let detail = directory_identity(
        &fixture.scenario,
        "Detail dependant",
        None,
        "detail",
        "member",
        "suspended",
        1,
    )
    .await?;
    let subject_member_id = detail.membership.id;
    let _subject_person_id = detail.person.id;
    let mut detail_events = Vec::new();
    for index in 1..=28 {
        detail_events.push(event(&fixture.scenario,fixture.household.id,"household_access.membership_changed", (true, index!=28),
        json!({"target_membership_id":detail.membership.id,"outcome":if index==28 {"rejected"}else if index==27 {"no_change"}else{"success"},"new_state":{"status":"suspended"},"secret":"private clinical sentinel"}),
        json!({}),"2030-01-15T12:00:00".parse().unwrap()).await?);
    }
    let excluded = event(
        &fixture.scenario,
        fixture.household.id,
        "api.request",
        (false, false),
        json!({"target_membership_id":detail.membership.id}),
        json!({}),
        chrono::Utc::now().naive_utc(),
    )
    .await?;
    event(
        &fixture.scenario,
        fixture.household.id,
        "household_access.membership_changed",
        (false, false),
        json!({"target_membership_id":detail.membership.id.to_string()}),
        json!({}),
        chrono::Utc::now().naive_utc(),
    )
    .await?;
    event(
        &fixture.scenario,
        fixture.household.id,
        "household_access.membership_changed",
        (false, false),
        json!({"target_membership_id":fixture.membership.id}),
        json!({}),
        chrono::Utc::now().naive_utc(),
    )
    .await?;
    let foreign_detail = directory_identity(
        &fixture.foreign,
        "Foreign secret identity",
        None,
        "foreign-detail",
        "member",
        "active",
        0,
    )
    .await?;
    let foreign_detail_member_id = foreign_detail.membership.id;
    let foreign_event = event(
        &fixture.scenario,
        fixture.foreign.household.id,
        "household_access.membership_changed",
        (false, false),
        json!({"target_membership_id":detail.membership.id}),
        json!({}),
        chrono::Utc::now().naive_utc(),
    )
    .await?;
    accounts::Entity::update_many()
        .col_expr(
            accounts::Column::Name,
            Expr::value("<script>alert(1)</script>"),
        )
        .filter(accounts::Column::Id.eq(fixture.account.id))
        .exec(&fixture.admin)
        .await?;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&mut request, &fixture, &boot).await?;
    let detail_response=request.get(format!("/households/{slug}/admin/memberships/{subject_member_id}?q=Detail&status=suspended&sort=email&direction=desc&page=2&lang=en").as_str()).await;
    assert_eq!(
        detail_response.status_code(),
        200,
        "{}",
        detail_response.text()
    );
    let html = detail_response.text();
    assert!(html.contains("Detail dependant"));
    assert!(html.contains(detail.email.as_str()));
    assert!(html.contains("Rejected attempt"));
    assert!(html.contains("No change"));
    assert!(!html.contains("private clinical sentinel"));
    assert_eq!(html.matches("data-member-event=").count(), 25);
    assert!(html.contains(&format!("data-member-event=\"{}\"", detail_events[27].id)));
    assert!(!html.contains(&format!("data-member-event=\"{}\"", excluded.id)));
    assert!(html.contains(format!("Account #{account_id}").as_str()));
    assert!(
        html.contains(format!("Account #{account_id}, membership #{actor_member_id}").as_str())
    );
    accounts::Entity::update_many()
        .col_expr(
            accounts::Column::Name,
            Expr::value("Renamed synthetic actor"),
        )
        .filter(accounts::Column::Id.eq(fixture.account.id))
        .exec(&fixture.admin)
        .await?;
    let renamed = request
        .get(format!("/households/{slug}/admin/memberships/{subject_member_id}").as_str())
        .await;
    assert_eq!(renamed.status_code(), 200);
    assert!(renamed.text().contains("Renamed synthetic actor"));
    assert!(
        renamed
            .text()
            .contains(format!("Account #{account_id}, membership #{actor_member_id}").as_str())
    );
    assert!(!html.contains("<script>alert(1)</script>"));
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(!html.contains(&format!("data-member-event=\"{}\"", foreign_event.id)));
    assert_eq!(
        request
            .get(
                format!("/households/{slug}/admin/memberships/{foreign_detail_member_id}").as_str()
            )
            .await
            .status_code(),
        404
    );
    assert_eq!(
        request
            .get(
                format!(
                    "/households/{slug}/admin/memberships/{subject_member_id}?history_cursor=bad"
                )
                .as_str()
            )
            .await
            .status_code(),
        422
    );

    assert!(html.contains("/admin/users?q=Detail&amp;role=all&amp;status=suspended&amp;sort=email&amp;direction=desc&amp;page=2&amp;lang=en"));
    let older = html
        .split(r#"data-member-older href=""#)
        .nth(1)
        .ok_or("older link missing")?
        .split('"')
        .next()
        .ok_or("older link missing")?
        .replace("&amp;", "&");
    assert_eq!(
        request
            .get(&older.replace(
                format!("/memberships/{subject_member_id}?").as_str(),
                format!("/memberships/{actor_member_id}?").as_str()
            ))
            .await
            .status_code(),
        422
    );
    let next = request.get(&older).await;
    assert_eq!(next.status_code(), 200, "{}", next.text());
    assert_eq!(next.text().matches("data-member-event=").count(), 3);
    assert!(!next.text().contains(&format!("{}\"", detail_events[3].id)));
    let csrf = authenticity_token(&html).ok_or("csrf missing")?;
    let changed = request
        .post(format!("/households/{slug}/admin/users/{subject_member_id}/restore").as_str())
        .form(&[
            ("confirmation", "restored"),
            ("authenticity_token", csrf),
            ("return_to", "member"),
            ("q", "Detail"),
            ("status", "suspended"),
            ("sort", "email"),
            ("direction", "desc"),
            ("page", "2"),
            ("lang", "en"),
        ])
        .await;
    assert_eq!(changed.status_code(), 303, "{}", changed.text());
    assert!(changed.header("location").to_str()?.starts_with(
        format!("/households/{slug}/admin/memberships/{subject_member_id}?").as_str()
    ));
    assert_eq!(
        request
            .get(format!("/households/{slug}/admin/memberships/{absent_id}").as_str())
            .await
            .status_code(),
        404
    );
    household_memberships::Entity::update_many()
        .col_expr(household_memberships::Column::Role, Expr::value("member"))
        .filter(household_memberships::Column::Id.eq(fixture.membership.id))
        .exec(&fixture.admin)
        .await?;
    assert_eq!(
        request
            .get(format!("/households/{slug}/admin/memberships/{subject_member_id}").as_str())
            .await
            .status_code(),
        403
    );
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn member_detail_rechecks_authority_and_keeps_context_on_errors() -> TestResult {
    let (fixture, boot) = boot_request_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let slug = fixture.household.slug.clone();

    let actor_member_id = fixture.membership.id;
    let minor_membership_id = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(fixture.household.id))
        .filter(household_memberships::Column::PersonId.eq(fixture.minor.id))
        .one(&fixture.admin)
        .await?
        .unwrap()
        .id;
    household_memberships::Entity::update_many()
        .col_expr(household_memberships::Column::Role, Expr::value("owner"))
        .filter(household_memberships::Column::Id.eq(fixture.membership.id))
        .exec(&fixture.admin)
        .await?;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&mut request, &fixture, &boot).await?;
    let before = audit_total(&fixture).await?;
    for lang in ["en", "es", "cy", "ga", "pt"] {
        let detail = request
            .get(&format!(
                "/households/{slug}/admin/memberships/{actor_member_id}?lang={lang}"
            ))
            .await;
        assert_eq!(detail.status_code(), 200, "{}", detail.text());
        assert!(detail.text().contains(&format!(r#"<html lang="{lang}""#)));
        assert_eq!(detail.text().matches("data-member-event=").count(), 0);
    }
    let after = audit_total(&fixture).await?;
    assert_eq!(before, after);
    let detail=request.get(format!("/households/{slug}/admin/memberships/{actor_member_id}?q=Synthetic&role=owner&sort=email&direction=desc&page=3&lang=en").as_str()).await;
    let html = detail.text();
    let csrf = authenticity_token(&html).ok_or("csrf missing")?;
    let rejected = request
        .post(format!("/households/{slug}/admin/users/{actor_member_id}/suspend").as_str())
        .form(&[
            ("confirmation", "suspended"),
            ("authenticity_token", csrf),
            ("return_to", "member"),
            ("q", "Synthetic"),
            ("role", "owner"),
            ("sort", "email"),
            ("direction", "desc"),
            ("page", "3"),
            ("lang", "en"),
        ])
        .await;
    assert_eq!(rejected.status_code(), 422, "{}", rejected.text());
    assert!(
        rejected
            .text()
            .contains("Last active owner cannot be removed")
    );
    assert!(rejected.text().contains("Rejected attempt"));
    assert!(rejected.text().contains("Requested change (not applied)"));
    assert!(rejected.text().contains(r#"name="q" value="Synthetic""#));
    assert!(
        rejected
            .text()
            .contains(r#"name="return_to" value="member""#)
    );
    assert!(rejected.text().contains("/admin/users?q=Synthetic&amp;role=owner&amp;status=all&amp;sort=email&amp;direction=desc&amp;page=3&amp;lang=en"));
    let role=request.get(format!("/households/{slug}/admin/memberships/{actor_member_id}/edit?q=Synthetic&return_to=member&page=3&lang=es").as_str()).await;
    assert_eq!(role.status_code(), 200, "{}", role.text());
    assert!(
        role.text()
            .contains(format!("/admin/memberships/{actor_member_id}?q=Synthetic").as_str())
    );
    let role_html = role.text();
    let csrf = authenticity_token(&role_html).ok_or("csrf missing")?;
    let role_save = request
        .post(
            format!("/households/{slug}/admin/memberships/{actor_member_id}/membership_role")
                .as_str(),
        )
        .form(&[
            ("role", "owner"),
            ("role_filter", "all"),
            ("authenticity_token", csrf),
            ("return_to", "member"),
            ("q", "Synthetic"),
            ("page", "3"),
            ("lang", "es"),
        ])
        .await;
    assert_eq!(role_save.status_code(), 303, "{}", role_save.text());
    assert!(
        role_save
            .header("location")
            .to_str()?
            .contains(format!("/admin/memberships/{actor_member_id}?q=Synthetic").as_str())
    );
    assert!(role_save.header("location").to_str()?.contains("lang=es"));
    let mut member = fixture.membership.clone().into_active_model();
    member.status = Set("revoked".into());
    member.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    member.update(&fixture.admin).await?;
    assert_eq!(
        request
            .get(format!("/households/{slug}/admin/memberships/{minor_membership_id}").as_str())
            .await
            .status_code(),
        403
    );
    fixture
        .membership_state(fixture.membership.permissions_version, false)
        .await;
    account_lockouts::ActiveModel {
        account_id: Set(fixture.account.id),
        key: Set(fixture.credential("detail-lock")),
        deadline: Set((chrono::Utc::now() + chrono::Duration::hours(1)).naive_utc()),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    let locked = request
        .get(format!("/households/{slug}/admin/memberships/{minor_membership_id}").as_str())
        .await;
    assert_eq!(locked.status_code(), 303);
    assert_eq!(locked.header("location").to_str()?, "/login");
    assert!(!locked.text().contains("Synthetic minor"));
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn invitation_management_filters_are_literal_bounded_and_preserve_action_context()
-> TestResult {
    let (fixture, boot) = boot_request_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let slug = fixture.household.slug.clone();

    let mut invitations = Vec::new();
    for index in 1..=52 {
        let invitation = invite(
            &fixture.scenario,
            fixture.household.id,
            fixture.membership.id,
            &format!("Synthetic-{index}"),
            "member",
            false,
        )
        .await?;
        let mut row = invitation.into_active_model();
        row.created_at = Set("2026-01-01T00:00:00".parse().unwrap());
        invitations.push(row.update(&fixture.admin).await?);
    }
    let literal_invitation = invite(
        &fixture.scenario,
        fixture.household.id,
        fixture.membership.id,
        "literal%_",
        "administrator",
        true,
    )
    .await?;

    let last_invitation_id = invitations[51].id;
    let previous_invitation_id = invitations[50].id;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&mut request, &fixture, &boot).await?;
    let path = format!("/households/{slug}/admin/invitations");
    let literal = request
        .get(&format!(
            "{path}?q=%25_&role=administrator&status=expired&lang=es"
        ))
        .await;
    assert_eq!(literal.status_code(), 200, "{}", literal.text());
    assert!(literal.text().contains(literal_invitation.email.as_str()));
    assert!(!literal.text().contains("Synthetic-"));
    let first = request
        .get(&format!(
            "{path}?q=SYNTHETIC&role=member&status=pending&lang=es"
        ))
        .await;
    assert_eq!(first.status_code(), 200, "{}", first.text());
    let html = first.text();
    assert_eq!(html.matches("class=\"card-title break-all\"").count(), 50);
    assert!(html.contains(invitations[51].email.as_str()));
    assert!(!html.contains(invitations[1].email.as_str()));
    let next = html
        .split("rel=\"next\" href=\"")
        .nth(1)
        .and_then(|value| value.split('"').next())
        .ok_or("next page missing")?
        .replace("&amp;", "&");
    let second = request.get(&next).await;
    assert_eq!(second.status_code(), 200, "{}", second.text());
    assert!(second.text().contains(invitations[1].email.as_str()));
    assert!(second.text().contains(invitations[0].email.as_str()));
    assert!(!second.text().contains(invitations[2].email.as_str()));
    assert!(!second.text().contains("rel=\"next\""));
    let token = authenticity_token(&html).ok_or("invitation CSRF missing")?;
    let changed = request
        .post(&format!("{path}/{last_invitation_id}/resend"))
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("authenticity_token", token),
            ("q", "SYNTHETIC"),
            ("role", "member"),
            ("status", "pending"),
            ("lang", "es"),
        ])
        .await;
    assert_eq!(changed.status_code(), 303, "{}", changed.text());
    let location = changed
        .headers()
        .get("location")
        .ok_or("redirect missing")?
        .to_str()?;
    for expected in [
        "q=SYNTHETIC",
        "role=member",
        "status=pending",
        "lang=es",
        "notice=resent",
    ] {
        assert!(location.contains(expected), "{location}");
    }
    let query = {
        use sea_orm::sea_query::ExprTrait;
        Query::update()
            .table("invitation_mail_attempts")
            .values([
                ("status", Expr::val("failed")),
                ("category", Expr::val("unavailable")),
                ("finished_at", Expr::val(chrono::Utc::now().naive_utc())),
            ])
            .and_where(Expr::col("household_id").eq(fixture.household.id))
            .to_owned()
    };
    fixture
        .admin
        .execute_raw(fixture.admin.get_database_backend().build(&query))
        .await?;
    let unavailable = request.get(&format!("{path}?q=SYNTHETIC&lang=en")).await;
    assert_eq!(unavailable.status_code(), 200, "{}", unavailable.text());
    assert!(
        unavailable
            .text()
            .contains("cancel this invitation and create a new one"),
        "{}",
        unavailable.text()
    );
    assert!(
        !unavailable
            .text()
            .contains("Use Resend to issue a new link")
    );
    let invalid_cursor = request.get(&next.replace("q=SYNTHETIC", "q=other")).await;
    assert_eq!(invalid_cursor.status_code(), 422);
    for query in [
        "q=a&q=b",
        "role=owner",
        "status=sent",
        "cursor=invalid",
        "q=a&lang=es&lang=en",
    ] {
        assert_eq!(
            request.get(&format!("{path}?{query}")).await.status_code(),
            422,
            "{query}"
        );
    }
    let invalid = request
        .post(&path)
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("authenticity_token", token),
            ("email", "invalid"),
            ("membership_role", "member"),
            ("q", "SYNTHETIC"),
            ("role", "member"),
            ("status", "pending"),
            ("lang", "es"),
        ])
        .await;
    assert_eq!(invalid.status_code(), 422, "{}", invalid.text());
    for field in ["q", "role", "status", "lang"] {
        assert!(invalid.text().contains(&format!("name=\"{field}\"")));
    }
    for language in ["en", "cy", "ga", "pt", "es"] {
        let local = request
            .get(&format!("{path}?q=SYNTHETIC&lang={language}"))
            .await;
        assert_eq!(local.status_code(), 200, "{}", local.text());
        assert!(local.text().contains(&format!("<html lang=\"{language}\"")));
        assert!(local.text().contains("Latest email attempt") == (language == "en"));
    }
    let before = attempt_total(&fixture).await?;
    fixture.member().await;
    fixture
        .membership_state(fixture.membership.permissions_version + 1, false)
        .await;
    let denied = request
        .post(&format!("{path}/{previous_invitation_id}/resend"))
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("authenticity_token", token), ("lang", "es")])
        .await;
    assert_eq!(denied.status_code(), 403, "{}", denied.text());
    assert_eq!(request.get(&path).await.status_code(), 403);
    let after = attempt_total(&fixture).await?;
    assert_eq!(before, after);
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn invitation_actions_reject_cross_filter_and_household_cursors_without_mutation()
-> TestResult {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::invitations::browser,
    };
    let (fixture, boot) = boot_request_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let slug = fixture.household.slug.clone();
    let household_id = fixture.household.id;
    let account_id = fixture.account.id;
    let (other, other_membership) =
        new_home(&fixture.scenario, "Other synthetic household").await?;
    let other_id = other.id;
    let mut invitations = Vec::new();
    for index in 1..=51 {
        invitations.push(
            invite(
                &fixture.scenario,
                fixture.household.id,
                fixture.membership.id,
                &format!("synthetic-{index}"),
                "member",
                false,
            )
            .await?,
        );
        invite(
            &fixture.scenario,
            other.id,
            other_membership.id,
            &format!("synthetic-{index}"),
            "member",
            false,
        )
        .await?;
    }

    let first_invitation_id = invitations[0].id;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&mut request, &fixture, &boot).await?;
    let page = request
        .get(format!("/households/{slug}/admin/invitations").as_str())
        .await;
    let html = page.text();
    let token = authenticity_token(&html).ok_or("invitation token missing")?;
    let mut cursors = Vec::new();
    for household_id in [household_id, other_id] {
        let scope = HouseholdScope {
            actor: Actor { account_id },
            household_id,
            request_id: "invitation-cursor-proof".into(),
        };
        let tenant = access::begin(&fixture.runtime, &scope, None).await?;
        let records = browser::browse(
            &tenant,
            None,
            &browser::BrowseFilters::parse("synthetic", "", "")?,
        )
        .await?;
        cursors.push(records.next_cursor.ok_or("bounded cursor missing")?);
        tenant.commit().await?;
    }
    for (cursor, search) in [(&cursors[0], "wrong-filter"), (&cursors[1], "synthetic")] {
        let before = invitation_state(&fixture, invitations[0].id).await?;
        let rejected = request
            .post(
                format!("/households/{slug}/admin/invitations/{first_invitation_id}/resend")
                    .as_str(),
            )
            .add_header("sec-fetch-site", "same-origin")
            .form(&[
                ("authenticity_token", token),
                ("q", search),
                ("cursor", cursor.as_str()),
                ("lang", "es"),
            ])
            .await;
        assert_eq!(rejected.status_code(), 422, "{}", rejected.text());
        let after = invitation_state(&fixture, invitations[0].id).await?;
        assert_eq!(before.0, after.0);
        assert_eq!(before.1, after.1);
        assert_eq!(before.2, after.2);
    }
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}
