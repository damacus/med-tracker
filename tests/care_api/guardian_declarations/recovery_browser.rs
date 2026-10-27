use super::{Application, require, withdrawal::declaration};
use crate::platform::{form_page, hidden_field, post_form, sign_in};
use med_tracker::models::_entities::{
    carer_relationships, household_memberships, people, security_audit_events,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, Set,
    sea_query::{Expr, ExprTrait, Func, Query},
};

#[tokio::test]
async fn guardian_recovery_browser_discloses_eligible_minor_and_requires_explicit_confirmation()
-> Result<(), DbErr> {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let slug = app.fixture.household.slug.clone();
    let result = async {
        owner(&app).await?;
        let subject = minor(&app, "Synthetic recovery child").await?;
        let cookie = sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;
        let (_, list) = form_page(
            &app,
            &cookie,
            &format!("/households/{slug}/admin/carer_relationships"),
        )
        .await;
        require(
            list.contains("/admin/carer_relationships/recover"),
            "Admin relationships must link to recovery",
        )?;
        let (_, page) = form_page(
            &app,
            &cookie,
            &format!("/households/{slug}/admin/carer_relationships/recover"),
        )
        .await;
        require(
            page.contains("Synthetic recovery child")
                && page.contains(&format!("/recover/{subject}")),
            "Recovery list must show eligible minor",
        )?;
        let (_, detail) = form_page(
            &app,
            &cookie,
            &format!("/households/{slug}/admin/carer_relationships/recover/{subject}"),
        )
        .await;
        require(
            detail.contains("Synthetic recovery child")
                && detail.contains("authenticity_token")
                && detail.contains("confirm_recovery")
                && detail.contains("snapshot_token"),
            "Recovery detail must show CSRF, snapshot and deliberate confirmation",
        )?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_browser_appoints_with_audit_and_new_guardian_can_withdraw()
-> Result<(), DbErr> {
    crate::authentication_fixture::reset_source_quota().await;
    let mut app = Application::new().await;
    let slug = app.fixture.household.slug.clone();
    let result = async {
        app.client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none())
            .build().map_err(|error| DbErr::Custom(error.to_string()))?;
owner(&app).await?;
let subject=minor(&app,"Synthetic browser orphan").await?;
let guardian=guardian(&app,"Synthetic newly appointed adult").await?;
let guardian_id=guardian.account.person_id.to_string();
        let cookie=sign_in(&app,app.fixture.account.id,&app.fixture.account.email).await;
        let path=&format!("/households/{slug}/admin/carer_relationships/recover/{subject}");
        let (cookies,page)=form_page(&app,&cookie,path).await;
        let csrf=hidden_field(&page,"authenticity_token");
        let snapshot=hidden_field(&page,"snapshot_token");
        let revision=hidden_field(&page,"snapshot_revision");
        let link=hidden_field(&page,"link_token");
        let form=[("authenticity_token",csrf.as_str()),("action","appoint"),("guardian_id",guardian_id.as_str()),
            ("snapshot_token",snapshot.as_str()),("snapshot_revision",revision.as_str()),
            ("link_token",link.as_str()),("reason","Manager confirmed a new guardian for this minor.")];
        let unconfirmed=post_form(&app,&cookies,path,&form).await;
        require(unconfirmed.status().as_u16()==422,"Recovery requires explicit server-side confirmation")?;
        let validation_page=unconfirmed.text().await.map_err(|error|DbErr::Custom(error.to_string()))?;
        require(validation_page.contains(&format!("<option value=\"{guardian_id}\" selected>")),
            "Appointment validation must retain the chosen guardian")?;
        let denied=post_form(&app,&cookies,path,&[("action","appoint"),("guardian_id",guardian_id.as_str()),
            ("snapshot_token",snapshot.as_str()),("snapshot_revision",revision.as_str()),
            ("link_token",link.as_str()),("reason","Manager confirmed a new guardian for this minor."),
            ("confirm_recovery","true")]).await;
        require(denied.status().as_u16()==403,"Recovery must reject a missing CSRF token")?;
        let malformed=post_form(&app,&cookies,path,&[form[0],form[1],form[2],
            ("snapshot_token","not-a-uuid"),form[4],form[5],form[6],("confirm_recovery","true")]).await;
        require(malformed.status().as_u16()==422,"Malformed hidden snapshot must be rejected")?;
        let malformed_page=malformed.text().await.map_err(|error|DbErr::Custom(error.to_string()))?;
        require(malformed_page.contains("href=\"#recovery-form\"")
            && malformed_page.contains("id=\"recovery-form\" tabindex=\"-1\""),
            "Hidden-field error must link to a focusable fallback when no restore form exists")?;
        let unknown_action=post_form(&app,&cookies,path,&[form[0],("action","unexpected"),form[2],form[3],form[4],form[5],form[6],("confirm_recovery","true")]).await;
        require(unknown_action.status().as_u16()==422,"Unknown action must be rejected")?;
        let unknown_page=unknown_action.text().await.map_err(|error|DbErr::Custom(error.to_string()))?;
        require(unknown_page.contains("href=\"#recovery-form\"")
            && !unknown_page.contains("href=\"#restore-reason\""),
            "Untrusted action must use the safe focusable fallback")?;
        let stale=post_form(&app,&cookies,path,&[form[0],form[1],form[2],
            ("snapshot_token","11111111-1111-4111-8111-111111111111"),form[4],form[5],form[6],
            ("confirm_recovery","true")]).await;
        require(stale.status().as_u16()==409,"Opaque stale confirmation must be rejected")?;
require(declarations(&app,subject).await?==0
    && carer_relationships::Entity::find().filter(carer_relationships::Column::PatientId.eq(subject)).count(&app.fixture.admin).await?==0
    && audits(&app,None).await?==0,
    "Missing confirmation, CSRF and stale snapshot must leave no authority or recovery audit")?;
        let valid=post_form(&app,&cookies,path,&[form[0],form[1],form[2],form[3],form[4],form[5],form[6],("confirm_recovery","true")]).await;
        require(valid.status().as_u16()==303,"Confirmed recovery should redirect after appointment")?;
let query=Query::select().columns(["id","revision"]).from("guardian_declarations")
    .and_where(Expr::col("subject_person_id").eq(subject))
    .and_where(Expr::col("guardian_person_id").eq(guardian.account.person_id))
    .and_where(Expr::col("source").eq("manager_appointment"))
    .and_where(Expr::col("declared_by_account_id").eq(app.fixture.account.id)).to_owned();
let row=app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&query)).await?
    .ok_or_else(||DbErr::Custom("Missing manager appointment".into()))?;
let id:i64=row.try_get("","id")?;
let revision:i64=row.try_get("","revision")?;
let links=carer_relationships::Entity::find().filter(carer_relationships::Column::GuardianDeclarationId.eq(id))
    .filter(carer_relationships::Column::Active.eq(true)).count(&app.fixture.admin).await?;
let audit=security_audit_events::Entity::find()
    .filter(security_audit_events::Column::EventType.eq("guardian_recovery.appointed"))
    .filter(security_audit_events::Column::ActorAccountId.eq(app.fixture.account.id))
    .filter(Expr::expr(Func::cust("jsonb_extract_path_text").args([Expr::col(security_audit_events::Column::Metadata),Expr::val("subject_person_id")])).eq(subject.to_string()))
    .count(&app.fixture.admin).await?;
require(links==1 && audit==1,"Appointment must persist exact authority and attributable audit")?;
        let guardian_cookie=sign_in(&app,guardian.account.id,&guardian.account.email).await;
        let (guardian_cookies,history)=form_page(&app,&guardian_cookie,"/profile/guardian-declarations").await;
        require(history.contains("Household manager appointment") && history.contains(&format!("/{id}/withdraw")),
            "New guardian must see truthful manager provenance and own withdrawal")?;
        let history_csrf=hidden_field(&history,"authenticity_token");
        let withdrawn=post_form(&app,&guardian_cookies,&format!("/profile/guardian-declarations/{id}/withdraw"),
            &[("authenticity_token",history_csrf.as_str()),("revision",revision.to_string().as_str()),("withdrawn","true")]).await;
        let withdrawn_status=withdrawn.status().as_u16();
        let withdrawn_body=withdrawn.text().await.map_err(|error| DbErr::Custom(error.to_string()))?;
        require(withdrawn_status==303,&format!("New guardian must be able to withdraw; status={withdrawn_status}; body={withdrawn_body}"))?;
        let (_,final_history)=form_page(&app,&guardian_cookie,"/profile/guardian-declarations").await;
        require(final_history.contains("Withdrawn") && !final_history.contains(&format!("/{id}/withdraw")),
            "Terminal withdrawal must be shown without a withdrawal action")?;
        guardian.close().await;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_browser_restores_exact_inactive_link_and_preserves_generic_denial()
-> Result<(), DbErr> {
    crate::authentication_fixture::reset_source_quota().await;
    let mut app = Application::new().await;
    let slug = app.fixture.household.slug.clone();
    let result=async {
        app.client=reqwest::Client::builder().redirect(reqwest::redirect::Policy::none())
            .build().map_err(|error|DbErr::Custom(error.to_string()))?;
        owner(&app).await?;
        let token=app.token().await;
        let (subject,declaration_id)=declaration(&app,&token).await?;
let link_id=carer_relationships::Entity::find().filter(carer_relationships::Column::GuardianDeclarationId.eq(declaration_id))
    .filter(carer_relationships::Column::HouseholdId.eq(app.fixture.household.id)).one(&app.fixture.admin).await?
    .ok_or_else(||DbErr::Custom("Missing initial guardian link".into()))?.id;

        carer_relationships::Entity::update_many().col_expr(carer_relationships::Column::Active,Expr::val(false))
            .filter(carer_relationships::Column::Id.eq(link_id)).exec(&app.fixture.admin).await?;
people::Entity::update_many().col_expr(people::Column::Name,Expr::val("Linked guardian <care>"))
    .filter(people::Column::Id.eq(app.fixture.adult.id)).exec(&app.fixture.admin).await?;
let guardian=guardian(&app,"Other eligible adult").await?;
let original_guardian=app.fixture.adult.id.to_string();
        let cookie=sign_in(&app,app.fixture.account.id,&app.fixture.account.email).await;
        let path=format!("/households/{slug}/admin/carer_relationships/recover/{subject}");
        let (cookies,page)=form_page(&app,&cookie,&path).await;
        require(page.contains("Restore existing guardian") && hidden_field(&page,"relationship_id")==link_id.to_string(),
            "Only the exact inactive declaration link should be offered for restoration")?;
        let restore_card=page.split("id=\"administration-form\"").next().unwrap_or("");
        require(restore_card.contains("Linked guardian &lt;care&gt;")
            && !restore_card.contains("Other eligible adult")
            && hidden_field(&page,"guardian_id")==original_guardian,
            "Restore card must visibly identify the exact escaped linked guardian, independent of another eligible adult")?;
        let csrf=hidden_field(&page,"authenticity_token");
        let snapshot=hidden_field(&page,"snapshot_token");
        let revision=hidden_field(&page,"snapshot_revision");
        let link_token=hidden_field(&page,"link_token");
        let declaration=declaration_id.to_string();
        let link=link_id.to_string();
        let form=[("authenticity_token",csrf.as_str()),("action","restore"),("guardian_id",original_guardian.as_str()),
            ("declaration_id",declaration.as_str()),("relationship_id",link.as_str()),
            ("snapshot_token",snapshot.as_str()),("snapshot_revision",revision.as_str()),
            ("link_token",link_token.as_str()),("reason","Manager restored an existing guardian link."),
            ("confirm_recovery","true")];
        let missing_confirm=post_form(&app,&cookies,&path,&form[..9]).await;
        require(missing_confirm.status().as_u16()==422,"Restore confirmation must be required")?;
        let confirmation_page=missing_confirm.text().await.map_err(|error|DbErr::Custom(error.to_string()))?;
        require(confirmation_page.contains("href=\"#restore-confirm\"")
            && confirmation_page.contains("id=\"restore-confirm\""),
            "Restore confirmation error must target the restore checkbox")?;
        let invalid_reason=post_form(&app,&cookies,&path,&[form[0],form[1],form[2],form[3],form[4],form[5],form[6],form[7],("reason","short"),form[9]]).await;
        require(invalid_reason.status().as_u16()==422,"Short restore reason must be rejected")?;
        let reason_page=invalid_reason.text().await.map_err(|error|DbErr::Custom(error.to_string()))?;
        require(reason_page.contains("href=\"#restore-reason\"")
            && reason_page.contains("id=\"restore-reason\""),
            "Restore reason error must target the restore textarea")?;
        let restored=post_form(&app,&cookies,&path,&form).await;
        require(restored.status().as_u16()==303,"Exact guardian restore must redirect after commit")?;
let restored_link=carer_relationships::Entity::find_by_id(link_id).one(&app.fixture.admin).await?
    .ok_or_else(||DbErr::Custom("Missing restored link".into()))?;
require(restored_link.active && declarations(&app,subject).await?==1
    && audits(&app,Some("guardian_recovery.restored")).await?==1,
    "Restore must reuse existing declaration/link and audit manager")?;
        let generic=post_form(&app,&cookies,&format!("/households/{slug}/admin/carer_relationships/{link_id}/activate"),
            &[("authenticity_token",csrf.as_str())]).await;
        require(generic.status().as_u16()==403,"Generic activation must remain forbidden")?;
        guardian.close().await;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_browser_shows_superseded_initial_history_without_withdrawal()
-> Result<(), DbErr> {
    crate::authentication_fixture::reset_source_quota().await;
    let mut app = Application::new().await;
    let slug = app.fixture.household.slug.clone();
    let result=async {
        app.client=reqwest::Client::builder().redirect(reqwest::redirect::Policy::none())
            .build().map_err(|error|DbErr::Custom(error.to_string()))?;
        owner(&app).await?;
        let token=app.token().await;
        let (subject,old_id)=declaration(&app,&token).await?;
carer_relationships::Entity::update_many().col_expr(carer_relationships::Column::Active,Expr::val(false))
    .filter(carer_relationships::Column::GuardianDeclarationId.eq(old_id)).exec(&app.fixture.admin).await?;
let guardian=guardian(&app,"Synthetic replacement guardian").await?;
let guardian_id=guardian.account.person_id.to_string();
        let cookie=sign_in(&app,app.fixture.account.id,&app.fixture.account.email).await;
        let path=format!("/households/{slug}/admin/carer_relationships/recover/{subject}");
        let (cookies,page)=form_page(&app,&cookie,&path).await;
        let csrf=hidden_field(&page,"authenticity_token");
        let snapshot=hidden_field(&page,"snapshot_token");
        let revision=hidden_field(&page,"snapshot_revision");
        let link=hidden_field(&page,"link_token");
        let appointed=post_form(&app,&cookies,&path,&[("authenticity_token",csrf.as_str()),
            ("action","appoint"),("guardian_id",guardian_id.as_str()),("snapshot_token",snapshot.as_str()),
            ("snapshot_revision",revision.as_str()),("link_token",link.as_str()),
            ("reason","Manager appointed another adult after link ended."),("confirm_recovery","true")]).await;
        require(appointed.status().as_u16()==303,"Different adult should be appointable while old link is inactive")?;
        let (_,history)=form_page(&app,&cookies,"/profile/guardian-declarations").await;
        require(history.contains("Your initial guardian declaration") && history.contains("Superseded by a later appointment")
            && !history.contains(&format!("/{old_id}/withdraw")),
            "Old guardian must see truthful superseded initial history with no terminal withdrawal control")?;
        guardian.close().await;
        Ok(())
    }.await;
    app.close().await;
    result
}

async fn owner(app: &Application) -> Result<(), DbErr> {
    household_memberships::Entity::update_many()
        .col_expr(household_memberships::Column::Role, Expr::val("owner"))
        .filter(household_memberships::Column::Id.eq(app.fixture.membership.id))
        .exec(&app.fixture.admin)
        .await?;
    Ok(())
}
async fn minor(app: &Application, name: &str) -> Result<i64, DbErr> {
    let now = chrono::Utc::now().naive_utc();
    let person = people::ActiveModel {
        name: Set(name.into()),
        date_of_birth: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1)),
        person_type: Set(1),
        has_capacity: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await?;
    participate(app, person.id).await?;
    Ok(person.id)
}
async fn participate(app: &Application, person: i64) -> Result<(), DbErr> {
    let now = chrono::Utc::now().naive_utc();
    household_memberships::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(person),
        role: Set("member".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await?;
    Ok(())
}
async fn guardian(app: &Application, name: &str) -> Result<crate::scenario::Scenario, DbErr> {
    let guardian = crate::scenario::Scenario::new().await;
    people::Entity::update_many()
        .col_expr(people::Column::Name, Expr::val(name))
        .filter(people::Column::Id.eq(guardian.account.person_id))
        .exec(&app.fixture.admin)
        .await?;
    participate(app, guardian.account.person_id).await?;
    Ok(guardian)
}
async fn declarations(app: &Application, subject: i64) -> Result<i64, DbErr> {
    let query = Query::select()
        .expr_as(Expr::col("id").count(), "count")
        .from("guardian_declarations")
        .and_where(Expr::col("subject_person_id").eq(subject))
        .to_owned();
    super::count(app, query).await
}
async fn audits(app: &Application, event: Option<&str>) -> Result<u64, DbErr> {
    let query = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(security_audit_events::Column::ActorAccountId.eq(app.fixture.account.id));
    let query = if let Some(event) = event {
        query.filter(security_audit_events::Column::EventType.eq(event))
    } else {
        query.filter(security_audit_events::Column::EventType.like("guardian_recovery.%"))
    };
    query.count(&app.fixture.admin).await
}
