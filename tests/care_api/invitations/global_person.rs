// SeaQuery has no PostgreSQL trigger/function DDL or catalogue ACL/regclass API.
// The exclusive fault probes retain narrow DDL and remove it before App close;
// generated row operations and transaction savepoints use SeaORM/SeaQuery.

use super::{Acceptance, Application, seed_acceptance};
use med_tracker::models::_entities::{
    accounts, api_change_events, carer_relationships, household_invitation_grants,
    household_invitations, household_memberships, people, person_access_grants, person_medications,
    schedules, users, versions,
};
use med_tracker::models::access::{self, Actor, HouseholdScope};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::Set,
    ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, IntoActiveModel, Order,
    PaginatorTrait, QueryFilter, SqlErr, Statement, TransactionTrait,
    sea_query::{Alias, Asterisk, Condition, Expr, ExprTrait, Func, Query, SelectStatement},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn require(condition: bool, message: &str) -> Result<(), DbErr> {
    if condition {
        Ok(())
    } else {
        Err(DbErr::Custom(message.to_owned()))
    }
}

fn database_code(error: &DbErr, expected: &str) -> bool {
    match error {
        DbErr::Exec(sea_orm::RuntimeErr::SqlxError(error))
        | DbErr::Query(sea_orm::RuntimeErr::SqlxError(error)) => error
            .as_database_error()
            .and_then(|error| error.code())
            .is_some_and(|code| code == expected),
        _ => false,
    }
}

#[tokio::test]
async fn canonical_schema_entity_relation_joins_exact_subject_participation() -> Result<(), DbErr> {
    use med_tracker::models::entities::{membership, person_medication};
    use sea_orm::EntityTrait;
    let app = Application::new().await;
    let result = async {
        let care=seed_member_care(&app).await?;

        let tenant = access::begin(&app.fixture.runtime,&HouseholdScope {
            actor:Actor{account_id:app.fixture.account.id},household_id:app.fixture.household.id,
            request_id:"synthetic-canonical-relation".to_owned()},None).await
            .map_err(|_|DbErr::Custom("Expected member relation scope".to_owned()))?;
        let joined = person_medication::Entity::find_by_id(care.assignment.id)
            .find_also_related(membership::Entity).one(tenant.transaction()).await?;
        tenant.rollback().await.map_err(|_|DbErr::Custom("Unable to close relation scope".to_owned()))?;
        require(joined.is_some_and(|(source, participation)| source.person_id==app.fixture.dependent.id
            && participation.is_some_and(|member|member.id==app.fixture.dependent_membership.id && member.person_id==source.person_id
                && member.household_id==source.household_id)),
            "Composite entity relation must join the exact Person and origin household participation")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn canonical_schema_recovery_capability_is_bounded_without_person_visibility()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let acceptance=seed_membership_invitation(&app).await?;
        app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::insert().into_table("platform_admins").columns(["account_id","status","created_at","updated_at"]).values_panic([Expr::val(app.fixture.account.id),Expr::val("active"),Expr::expr(Func::cust("now")),Expr::expr(Func::cust("now"))]).to_owned())).await?;
        let catalogue = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT p.prosecdef AND r.rolname='med_tracker_owner' AND NOT r.rolcanlogin AND NOT r.rolsuper AND NOT r.rolbypassrls AND p.proconfig @> ARRAY['search_path=pg_catalog'] AND has_function_privilege('med_tracker_app',p.oid,'EXECUTE') AND NOT EXISTS(SELECT 1 FROM aclexplode(COALESCE(p.proacl,acldefault('f',p.proowner))) acl WHERE acl.grantee=0 AND acl.privilege_type='EXECUTE') AND (SELECT relrowsecurity AND relforcerowsecurity FROM pg_class WHERE oid='public.people'::regclass) AS secured FROM pg_proc p JOIN pg_roles r ON r.oid=p.proowner WHERE p.oid='med_tracker.recovery_member_eligible(bigint)'::regprocedure"))
            .await?.ok_or_else(||DbErr::Custom("Expected recovery capability catalogue".to_owned()))?;
        require(catalogue.try_get("","secured")?, "Recovery capability must retain fixed non-login owner, RLS, search path and least-privileged ACL")?;
        let transaction = app.fixture.runtime.begin().await?;
        runtime_context(&transaction,app.fixture.account.id,acceptance.target_household.id,None).await?;
        let facts = transaction.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr_as(Expr::expr(Func::cust("med_tracker.recovery_member_eligible").args([Expr::val(acceptance.inviter_membership.id)])),"eligible").expr_as(Expr::expr(Func::cust("med_tracker.recovery_member_eligible").args([Expr::val(app.fixture.membership.id)])),"foreign").expr_as(Expr::expr(Func::cust("med_tracker.recovery_member_eligible").args([Expr::val(i64::MAX)])),"missing").expr_as(Expr::exists(Query::select().expr(Expr::val(1)).from("people").and_where(Expr::col("id").eq(Expr::val(acceptance.inviter_account.person_id))).to_owned()),"directly_visible").to_owned()))
            .await?.ok_or_else(||DbErr::Custom("Expected restricted recovery facts".to_owned()))?;
        require(facts.try_get("","eligible")? && !facts.try_get::<bool>("","foreign")?
            && !facts.try_get::<bool>("","missing")? && !facts.try_get::<bool>("","directly_visible")?,
            "A non-member administrator may inspect only eligibility, without direct Person roster visibility or recursion")?;
        for account in [String::new(),acceptance.inviter_account.id.to_string()] {
            let invalid = transaction.begin().await?;
            invalid.execute_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr(Expr::expr(Func::cust("set_config").args([Expr::val("med_tracker.current_account_id"),Expr::val(account.as_str()),Expr::val(true)]))).to_owned())).await?;
            let denied = invalid.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr(Expr::expr(Func::cust("med_tracker.recovery_member_eligible").args([Expr::val(acceptance.inviter_membership.id)]))).to_owned())).await;
            invalid.rollback().await?;
            require(denied.as_ref().err().is_some_and(|error|database_code(error,"42501")),
                "Missing or non-admin context must fail closed with insufficient privilege")?;
        }
        transaction.rollback().await?;
        app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("platform_admins").value("status",Expr::val("revoked")).and_where(Expr::col("account_id").eq(Expr::val(app.fixture.account.id))).to_owned())).await?;
        let denied = app.fixture.runtime.begin().await?;
        runtime_context(&denied,app.fixture.account.id,acceptance.target_household.id,None).await?;
        let result = denied.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr(Expr::expr(Func::cust("med_tracker.recovery_member_eligible").args([Expr::val(acceptance.inviter_membership.id)]))).to_owned())).await;
        denied.rollback().await?;
        require(result.as_ref().err().is_some_and(|error|database_code(error,"42501")),
            "Revoked administrator must lose even eligibility-only access")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn canonical_schema_account_person_constraints_are_atomic_and_mandatory() -> Result<(), DbErr>
{
    let app = Application::new().await;
    let result = async {
        let now = chrono::Utc::now().naive_utc();
        for (person, duplicate, code) in [
            (None, false, "23502"),
            (Some(app.fixture.adult.id), false, "23505"),
            (None, true, "23502"),
            (Some(app.fixture.adult.id), true, "23505"),
        ] {
            let transaction = app.fixture.runtime.begin().await?;
            runtime_context(
                &transaction,
                app.fixture.account.id,
                app.fixture.household.id,
                Some(app.fixture.membership.id),
            )
            .await?;
            let operation = if duplicate {
                let query = Query::insert()
                    .into_table("household_memberships")
                    .columns([
                        "household_id",
                        "person_id",
                        "role",
                        "status",
                        "joined_at",
                        "created_at",
                        "updated_at",
                    ])
                    .values_panic([
                        Expr::val(app.fixture.household.id),
                        Expr::val(person),
                        Expr::val("member"),
                        Expr::val("active"),
                        Expr::val(now),
                        Expr::val(now),
                        Expr::val(now),
                    ])
                    .to_owned();
                transaction
                    .execute_raw(transaction.get_database_backend().build(&query))
                    .await
            } else {
                let query = Query::insert()
                    .into_table("accounts")
                    .columns(["person_id", "email", "status", "created_at", "updated_at"])
                    .values_panic([
                        Expr::val(person),
                        Expr::val(case_email(&app, &uuid::Uuid::new_v4().to_string())),
                        Expr::val(2),
                        Expr::val(now.and_utc()),
                        Expr::val(now.and_utc()),
                    ])
                    .to_owned();
                transaction
                    .execute_raw(transaction.get_database_backend().build(&query))
                    .await
            };
            transaction.rollback().await?;
            require(
                operation
                    .as_ref()
                    .err()
                    .is_some_and(|error| database_code(error, code)),
                "Canonical identity cardinality must be enforced by the database",
            )?;
        }
        let reserved_person = reserved_id(&app, "public.people").await?;
        let dangling = app.fixture.runtime.begin().await?;
        dangling
            .execute_unprepared("SET LOCAL ROLE med_tracker_app")
            .await?;
        accounts::ActiveModel {
            person_id: Set(reserved_person),
            email: Set(case_email(&app, "deferred-person")),
            status: Set(2),
            created_at: Set(now.and_utc()),
            updated_at: Set(now.and_utc()),
            ..Default::default()
        }
        .insert(&dangling)
        .await?;
        let commit = dangling.commit().await;
        require(
            commit
                .as_ref()
                .err()
                .is_some_and(|error| database_code(error, "23503")),
            "Missing reserved Person must reject the deferred Account commit",
        )?;
        let valid = app.fixture.runtime.begin().await?;
        valid
            .execute_unprepared("SET LOCAL ROLE med_tracker_app")
            .await?;
        let account = accounts::ActiveModel {
            person_id: Set(reserved_person),
            email: Set(case_email(&app, "deferred-person")),
            status: Set(2),
            created_at: Set(now.and_utc()),
            updated_at: Set(now.and_utc()),
            ..Default::default()
        }
        .insert(&valid)
        .await?;
        set_runtime_value(
            &valid,
            "med_tracker.current_account_id",
            &account.id.to_string(),
        )
        .await?;
        let person = people::ActiveModel {
            id: Set(reserved_person),
            name: Set("Synthetic reserved self".into()),
            person_type: Set(0),
            has_capacity: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&valid)
        .await?;
        valid.commit().await?;
        let rolled_person = reserved_id(&app, "public.people").await?;
        let rollback = app.fixture.runtime.begin().await?;
        rollback
            .execute_unprepared("SET LOCAL ROLE med_tracker_app")
            .await?;
        let rolled_account = accounts::ActiveModel {
            person_id: Set(rolled_person),
            email: Set(case_email(&app, "rolled-back-person")),
            status: Set(2),
            created_at: Set(now.and_utc()),
            updated_at: Set(now.and_utc()),
            ..Default::default()
        }
        .insert(&rollback)
        .await?;
        set_runtime_value(
            &rollback,
            "med_tracker.current_account_id",
            &rolled_account.id.to_string(),
        )
        .await?;
        people::ActiveModel {
            id: Set(rolled_person),
            name: Set("Synthetic rolled back self".into()),
            person_type: Set(0),
            has_capacity: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&rollback)
        .await?;
        rollback.rollback().await?;
        require(
            accounts::Entity::find_by_id(account.id)
                .one(&app.fixture.admin)
                .await?
                .is_some_and(|row| row.person_id == person.id)
                && people::Entity::find_by_id(person.id)
                    .one(&app.fixture.admin)
                    .await?
                    .is_some()
                && accounts::Entity::find_by_id(rolled_account.id)
                    .one(&app.fixture.admin)
                    .await?
                    .is_none()
                && people::Entity::find_by_id(rolled_person)
                    .one(&app.fixture.admin)
                    .await?
                    .is_none(),
            "Account and reserved Person must commit or roll back together",
        )?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}
async fn seed_member_care(app: &Application) -> Result<MemberCare, DbErr> {
    let acceptance = seed_membership_invitation(app).await?;
    household_memberships::Entity::update_many()
        .col_expr(household_memberships::Column::Role, Expr::val("member"))
        .col_expr(
            household_memberships::Column::PermissionsVersion,
            Expr::col(household_memberships::Column::PermissionsVersion).add(1),
        )
        .filter(household_memberships::Column::Id.eq(app.fixture.membership.id))
        .exec(&app.fixture.admin)
        .await?;
    person_access_grants::Entity::update_many()
        .col_expr(
            person_access_grants::Column::RevokedAt,
            Expr::val(chrono::Utc::now().naive_utc()),
        )
        .filter(person_access_grants::Column::HouseholdMembershipId.eq(app.fixture.membership.id))
        .exec(&app.fixture.admin)
        .await?;
    let now = chrono::Utc::now().naive_utc();
    let assignment = person_medications::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(app.fixture.dependent.id),
        medication_id: Set(app.fixture.medication.id),
        dose_amount: Set(Some(sea_orm::prelude::Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".into())),
        position: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await?;
    let schedule = schedules::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(app.fixture.dependent.id),
        medication_id: Set(app.fixture.medication.id),
        active: Set(true),
        start_date: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1)),
        end_date: Set(chrono::NaiveDate::from_ymd_opt(2099, 12, 31)),
        dose_amount: Set(Some(sea_orm::prelude::Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".into())),
        schedule_type: Set(4),
        schedule_config: Set(json!({})),
        frequency: Set(Some("As needed".into())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await?;
    let inviter_user = users::Entity::find()
        .filter(users::Column::PersonId.eq(acceptance.inviter_account.person_id))
        .one(&app.fixture.admin)
        .await?
        .ok_or_else(|| DbErr::Custom("Generated inviter User missing".into()))?;
    Ok(MemberCare {
        acceptance,
        assignment,
        schedule,
        inviter_user,
    })
}
struct MemberCare {
    acceptance: Acceptance,
    assignment: person_medications::Model,
    schedule: schedules::Model,
    inviter_user: users::Model,
}
#[tokio::test]
async fn canonical_invitation_browser_offers_membership_without_care_authority() -> Result<(), DbErr>
{
    let app = Application::new().await;
    let result=async {
        let _care=seed_member_care(&app).await?;

        app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("household_memberships").value("role",Expr::val("administrator")).value("permissions_version",Expr::col("permissions_version").add(Expr::val(1))).and_where(Expr::col("id").eq(Expr::val(app.fixture.membership.id))).to_owned())).await?;
        let cookie=super::super::platform::sign_in(&app,app.fixture.account.id,app.fixture.account.email.as_str()).await;
        let (_,html)=super::super::platform::form_page(&app,&cookie,&format!("/households/{generated_slug}/admin/invitations",generated_slug=app.fixture.household.slug)).await;
        require(html.contains("name=\"email\"") && html.contains("name=\"membership_role\"")
            && !html.contains("name=\"relationship_type\"") && !html.contains("name=\"access_level\"")
            && !html.contains("name=\"dependent_ids[]\""),
            "Household invitation form must offer membership without promising guardian or shared-plan authority")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn canonical_invitation_browser_rejects_parent_manage_promises_without_effects()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let _care=seed_member_care(&app).await?;

        app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("household_memberships").value("role",Expr::val("administrator")).value("permissions_version",Expr::col("permissions_version").add(Expr::val(1))).and_where(Expr::col("id").eq(Expr::val(app.fixture.membership.id))).to_owned())).await?;
        let cookie=super::super::platform::sign_in(&app,app.fixture.account.id,app.fixture.account.email.as_str()).await;
        let (cookie,html)=super::super::platform::form_page(&app,&cookie,&format!("/households/{generated_slug}/admin/invitations",generated_slug=app.fixture.household.slug)).await;
        let csrf=super::super::platform::hidden_field(&html,"authenticity_token");
        let response=super::super::platform::post_form(&app,&cookie,&format!("/households/{generated_slug}/admin/invitations",generated_slug=app.fixture.household.slug),&[
            ("email",&case_email(&app,"synthetic-membership-only@example.test")),("membership_role","member"),
            ("relationship_type","parent"),("access_level","manage"),("dependent_ids[]",app.fixture.minor.id.to_string().as_str()),
            ("authenticity_token",&csrf)]).await;
        let status=response.status().as_u16();
        let body=response.text().await.map_err(|_|DbErr::Custom("Expected rejected invitation form".to_owned()))?;
        let state=app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr_as(Expr::col(("invitation","id")).count_distinct(),"invitations").expr_as(Expr::expr(Func::cust("count").args([Expr::col(("grant_record","id"))])),"grants").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("pg_loco_queue").and_where(Expr::expr(Func::cust("jsonb_extract_path_text").args([Expr::col("task_data"),Expr::val("to")])).eq(Expr::val(case_email(&app,"synthetic-membership-only@example.test").to_string()))).to_owned()),"messages").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("versions").and_where(Expr::col("item_type").eq(Expr::val("HouseholdInvitation"))).and_where(Expr::col("household_id").eq(app.fixture.household.id)).to_owned()),"versions").from_as("household_invitations","invitation").join_as(sea_orm::sea_query::JoinType::LeftJoin,"household_invitation_grants","grant_record",Expr::col(("grant_record","household_invitation_id")).eq(Expr::col(("invitation","id")))).and_where(Expr::col(("invitation","household_id")).eq(Expr::val(app.fixture.household.id)).and(Expr::col(("invitation","email")).eq(Expr::val(case_email(&app,"synthetic-membership-only@example.test").to_string())))).to_owned()))
            .await?.ok_or_else(||DbErr::Custom("Expected actual membership invitation state".to_owned()))?;
        require(status==422 && state.try_get::<i64>("","invitations")?==0
            && state.try_get::<i64>("","grants")?==0 && state.try_get::<i64>("","messages")?==0
            && state.try_get::<i64>("","versions")?==0,
            "Stale care-authority fields must return422 without invitation, grant, mail or version effects")?;
        require(body.contains("Household invitations cannot assign care authority.")
            && body.contains(&case_email(&app,"synthetic-membership-only@example.test")) && body.contains("href=\"#invitation-form\"")
            && !body.contains("href=\"#dependent_ids\""),
            "Clinical-field rejection must preserve the safe draft and point to the form, never removed controls")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn canonical_invitation_api_rejects_clinical_fields_without_effects() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let _care=seed_member_care(&app).await?;

        app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("household_memberships").value("role",Expr::val("administrator")).value("permissions_version",Expr::col("permissions_version").add(Expr::val(1))).and_where(Expr::col("id").eq(Expr::val(app.fixture.membership.id))).to_owned())).await?;
        let token=app.token().await;
        for (name,value) in [("dependent_ids",json!([app.fixture.minor.id])),("relationship_type",json!("parent")),("access_level",json!("manage"))] {
            let mut attributes=json!({"email":&case_email(&app,"synthetic-clinical-field@example.test"),"membership_role":"member"});
            attributes[name]=value;
            let response=app.client.post(format!("{}/api/v1/households/{generated_72001}/admin/invitations",app.origin,generated_72001=app.fixture.household.id))
                .bearer_auth(&token).json(&json!({"household_invitation":attributes})).send().await
                .map_err(|_|DbErr::Custom("Invitation authority negative transport failed".to_owned()))?;
            require(response.status().as_u16()==422,"Unsupported clinical invitation fields must be rejected explicitly")?;
        }
        let state=app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("household_invitations").and_where(Expr::col("household_id").eq(Expr::val(app.fixture.household.id)).and(Expr::col("email").eq(Expr::val(case_email(&app,"synthetic-clinical-field@example.test").to_string())))).to_owned()),"invitations").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("household_invitation_grants").and_where(Expr::col("household_id").eq(Expr::val(app.fixture.household.id))).to_owned()),"grants").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("pg_loco_queue").and_where(Expr::expr(Func::cust("jsonb_extract_path_text").args([Expr::col("task_data"),Expr::val("to")])).eq(Expr::val(case_email(&app,"synthetic-clinical-field@example.test").to_string()))).to_owned()),"messages").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("versions").and_where(Expr::col("item_type").eq(Expr::val("HouseholdInvitation"))).and_where(Expr::col("household_id").eq(app.fixture.household.id)).to_owned()),"versions").to_owned()))
            .await?.ok_or_else(||DbErr::Custom("Expected rejected invitation state".to_owned()))?;
        for field in ["invitations","grants","messages","versions"] {
            require(state.try_get::<i64>("",field)?==0,"Rejected clinical fields must leave no invitation, grant, mail or version effects")?;
        }
        Ok(())
    }.await;
    app.close().await;
    result
}

async fn read_member(app: &Application, token: &str, path: &str) -> Result<Value, DbErr> {
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{generated_72001}/{path}",
            app.origin,
            generated_72001 = app.fixture.household.id
        ))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| DbErr::Custom("Member care read transport failed".to_owned()))?;
    require(
        response.status().as_u16() == 200,
        "Active member care read must succeed",
    )?;
    response
        .json()
        .await
        .map_err(|_| DbErr::Custom("Member care response must be JSON".to_owned()))
}

async fn write_member_plan(app: &Application, token: &str, id: i64) -> Result<u16, DbErr> {
    let endpoint = format!(
        "{}/api/v1/households/{generated_72001}/person_medications/{id}",
        app.origin,
        generated_72001 = app.fixture.household.id
    );
    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| DbErr::Custom("Plan read transport failed".to_owned()))?;
    require(
        read.status().as_u16() == 200,
        "Plan write needs an actually visible current source",
    )?;
    let etag = read
        .headers()
        .get("etag")
        .cloned()
        .ok_or_else(|| DbErr::Custom("Expected current plan ETag".to_owned()))?;
    let response = app.client.patch(&endpoint).bearer_auth(token).header("if-match",etag)
        .json(&json!({"person_medication":{"dose_amount":"3","notes":"Synthetic self-authorised revision"}}))
        .send().await.map_err(|_| DbErr::Custom("Plan write transport failed".to_owned()))?;
    Ok(response.status().as_u16())
}

#[tokio::test]
async fn member_reader_detail_offers_both_source_kinds_without_grants() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let care=seed_member_care(&app).await?;

        let cookie = super::super::platform::sign_in(&app,app.fixture.account.id,app.fixture.account.email.as_str()).await;
        let (_, html) = super::super::platform::form_page(&app,&cookie,
            &format!("/households/{generated_slug}/medications/{generated_80001}",generated_80001=app.fixture.medication.id,generated_slug=app.fixture.household.slug)).await;
        require(html.contains(&format!("dose-confirm-person_medication-{generated_81504}",generated_81504=care.assignment.id))
            && html.contains(&format!("dose-confirm-schedule-{generated_83504}",generated_83504=care.schedule.id)),
            "Medication detail must offer Record for both another participant's source kinds without grants")?;
        let token = app.token().await;
        require(write_member_plan(&app,&token,care.assignment.id).await? == 403,
            "Detail Record must not grant another participant's plan editing")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn member_reader_report_selectors_include_participants_without_grants() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let care=seed_member_care(&app).await?;
        let acceptance=&care.acceptance;
        let cookie = super::super::platform::sign_in(&app,app.fixture.account.id,app.fixture.account.email.as_str()).await;
        let (_, html) = super::super::platform::form_page(&app,&cookie,
            &format!("/households/{generated_slug}/reports",generated_slug=app.fixture.household.slug)).await;
        require(html.contains(&format!("value=\"{generated_73003}\"",generated_73003=app.fixture.dependent.id)) && html.contains(&format!("value=\"{generated_73001}\"",generated_73001=app.fixture.adult.id))
            && !html.contains(&format!("value=\"{generated_73103}\"",generated_73103=acceptance.inviter_account.person_id)) && !html.contains(&format!("value=\"{generated_73104}\"",generated_73104=acceptance.invited_child.id)),
            "Report selectors must include local participants and canonical self, never foreign People")?;
        let tenant = access::begin(&app.fixture.runtime,&HouseholdScope {
            actor:Actor {account_id:app.fixture.account.id},household_id:app.fixture.household.id,
            request_id:"synthetic-member-report-selectors".to_owned()},None).await
            .map_err(|_|DbErr::Custom("Expected member report scope".to_owned()))?;
        let manageable = med_tracker::models::care::reports::accessible_people(&tenant,access::PersonAccess::Manage).await;
        tenant.rollback().await.map_err(|_|DbErr::Custom("Unable to close report scope".to_owned()))?;
        let manageable = manageable.map_err(|_|DbErr::Custom("Expected report Manage selector".to_owned()))?;
        require(manageable.len()==1 && manageable.iter().all(|person|person.id==app.fixture.adult.id),
            "Report Manage selector must retain canonical self without granting another participant Manage")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn member_self_authority_reaches_capabilities_chooser_and_stock_creation() -> Result<(), DbErr>
{
    let app = Application::new().await;
    let result = async {
        let care=seed_member_care(&app).await?;
        let acceptance=&care.acceptance;
        let token = app.token().await;
        let capabilities = read_member(&app,&token,"ui_capabilities").await?;
        require(capabilities["data"]["people"]["manage_ids"]==json!([app.fixture.adult.id])
            && capabilities["data"]["medications"]["create"]==true
            && capabilities["data"]["people"]["create"]==false,
            "Aggregate Manage authority must include active canonical self exactly once, never another participant")?;
        let cookie = super::super::platform::sign_in(&app,app.fixture.account.id,app.fixture.account.email.as_str()).await;
        let (_, chooser) = super::super::platform::form_page(&app,&cookie,&format!("/households/{generated_slug}/medications/workflow",generated_slug=app.fixture.household.slug)).await;
        require(chooser.contains(&format!("/people/{generated_73001}/treatments/assignments/new?source=workflow",generated_73001=app.fixture.adult.id))
            && !chooser.contains(&format!("/people/{generated_73003}/treatments/assignments/new?source=workflow",generated_73003=app.fixture.dependent.id)),
            "Assignment chooser must offer canonical self and no unauthorised participant")?;
        let response = app.client.post(format!("{}/api/v1/households/{generated_72001}/medications",app.origin,generated_72001=app.fixture.household.id))
            .bearer_auth(&token).json(&json!({"medication":{"name":"Synthetic self-owned stock",
                "location_id":app.fixture.location.id,"dose_amount":"2","dose_unit":"tablet","current_supply":"10","reorder_threshold":"2"}}))
            .send().await.map_err(|_| DbErr::Custom("Self stock creation transport failed".to_owned()))?;
        require(response.status().as_u16()==201, "Active self plan authority must preserve maintained medication creation")?;
        let foreign = app.client.post(format!("{}/api/v1/households/{generated_72002}/medications",app.origin,generated_72002=acceptance.target_household.id))
            .bearer_auth(&token).json(&json!({"medication":{"name":"Synthetic forbidden foreign stock"}}))
            .send().await.map_err(|_| DbErr::Custom("Foreign stock denial transport failed".to_owned()))?;
        require(foreign.status().as_u16()==403, "Self authority must not authorise another household")?;
        let row = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("medications").and_where(Expr::col("name").eq(Expr::val("Synthetic self-owned stock")).and(Expr::col("household_id").eq(Expr::val(app.fixture.household.id)))).to_owned()),"local").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("medications").and_where(Expr::col("name").eq(Expr::val("Synthetic forbidden foreign stock")).and(Expr::col("household_id").eq(acceptance.target_household.id))).to_owned()),"foreign").to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Expected scoped self stock effects".to_owned()))?;
        require(row.try_get::<i64>("","local")?==1 && row.try_get::<i64>("","foreign")?==0,
            "Self stock creation must remain local and foreign denial must have no effects")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn ordinary_member_care_routes_cannot_remove_or_invite_new_authority() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let _care=seed_member_care(&app).await?;

        let relationship=relationship(&app,app.fixture.adult.id,app.fixture.dependent.id,"parent",true).await?;

        let cookie=super::super::platform::sign_in(&app,app.fixture.account.id,app.fixture.account.email.as_str()).await;
        let (cookies,edit)=super::super::platform::form_page(&app,&cookie,&format!("/households/{generated_slug}/people/{generated_73001}/edit",generated_73001=app.fixture.adult.id,generated_slug=app.fixture.household.slug)).await;
        let csrf=super::super::platform::hidden_field(&edit,"authenticity_token");
        require(!csrf.is_empty(),"Ordinary-member negatives must use real authenticated CSRF")?;
        let row=app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr(Expr::col("updated_at")).from("carer_relationships").and_where(Expr::col("id").eq(Expr::val(relationship.id))).to_owned())).await?.ok_or_else(||DbErr::Custom("Expected existing care relationship".to_owned()))?;
        let version=row.try_get::<chrono::NaiveDateTime>("","updated_at")?.to_string();
        let before=care_route_effects(&app).await?;
        require(before.last()==Some(&0),"Denied parent invitation must begin without queued work")?;
        let path=&format!("/households/{generated_slug}/people/{generated_73003}/carer_relationships",generated_73003=app.fixture.dependent.id,generated_slug=app.fixture.household.slug);
        let removal=super::super::platform::post_form(&app,&cookies,&format!("{path}/{generated_86505}/remove",generated_86505=relationship.id),&[("authenticity_token",&csrf),("version",&version)]).await;
        require(removal.status().as_u16()==403,"Ordinary member must not remove another recorded carer through a forged valid-CSRF request")?;
        let invitation=super::super::platform::post_form(&app,&cookies,path,&[("authenticity_token",&csrf),("email",&case_email(&app,"synthetic-forbidden-parent@example.test")),("relationship_type","parent")]).await;
        require(invitation.status().as_u16()==403,"Existing relationship or parent label must not permit a new person-scoped authority invitation")?;
        let after=care_route_effects(&app).await?;
        require(after==before,"Denied care routes must preserve active relationships, grants, invitations, versions and zero queued outbox work")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn member_recorded_relationship_page_preserves_read_and_removal_without_new_authority()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let _care=seed_member_care(&app).await?;

        manager(&app).await?;
        let relationship=relationship(&app,app.fixture.adult.id,app.fixture.dependent.id,"parent",true).await?;

        let cookie = super::super::platform::sign_in(&app,app.fixture.account.id,app.fixture.account.email.as_str()).await;
        let path = &format!("/households/{generated_slug}/people/{generated_73003}/carer_relationships",generated_73003=app.fixture.dependent.id,generated_slug=app.fixture.household.slug);
        let detail = app.client.get(format!("{}/households/{generated_slug}/people/{generated_73003}",app.origin,generated_73003=app.fixture.dependent.id,generated_slug=app.fixture.household.slug)).header("cookie",&cookie).send().await.map_err(|_|DbErr::Custom("Existing relationship detail transport failed".to_owned()))?;
        require(detail.status().as_u16()==200,"Current participant detail must remain readable")?;
        let detail=detail.text().await.map_err(|_|DbErr::Custom("Existing relationship detail body missing".to_owned()))?;
        require(detail.contains(&format!("href=\"{path}\"")),"Person detail must link to authorised existing care relationship reads/removal independently of blocked assignment")?;
        let response = app.client.get(format!("{}{path}",app.origin)).header("cookie",&cookie)
            .send().await.map_err(|_| DbErr::Custom("Recorded relationship read transport failed".to_owned()))?;
        let status = response.status().as_u16();
        let cookies = super::super::platform::response_cookies(&cookie,&response);
        let page = response.text().await.map_err(|_| DbErr::Custom("Recorded relationship page missing".to_owned()))?;
        require(status==200, &format!("Existing relationship read must remain available through live View authority: status={status}"))?;
        require(page.contains("Synthetic adult") && page.contains(&format!("/{generated_86504}/remove",generated_86504=relationship.id))
            && !page.contains("name=\"carer_id\"") && !page.contains("name=\"email\""),
            "Existing relationship must be displayed/removable without advertising unsupported new authority")?;
        let csrf = super::super::platform::hidden_field(&page,"authenticity_token");
        let version = super::super::platform::hidden_field(&page,"version");
        require(!csrf.is_empty() && !version.is_empty(), "Removal must use real CSRF and record version")?;
        let assign = super::super::platform::post_form(&app,&cookies,path,
            &[("authenticity_token",&csrf),("carer_id",app.fixture.adult.id.to_string().as_str()),("relationship_type","parent")]).await;
        require(assign.status().as_u16()==403, "View-only relationship page must not enable parent-label authority creation")?;
        let removed = super::super::platform::post_form(&app,&cookies,&format!("{path}/{generated_86504}/remove",generated_86504=relationship.id),
            &[("authenticity_token",&csrf),("version",&version)]).await;
        require(removed.status().as_u16()==200, "Existing permitted relationship removal must succeed and refresh")?;
        let (_, refreshed) = super::super::platform::form_page(&app,&cookies,path).await;
        require(!refreshed.contains(&format!("/{generated_86504}/restore",generated_86504=relationship.id)), "Read-only page must not advertise blocked authority restoration")?;
        let row = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr(Expr::col("active")).expr(Expr::col("updated_at")).from("carer_relationships").and_where(Expr::col("id").eq(Expr::val(relationship.id))).to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Expected retained deactivated relationship".to_owned()))?;
        require(!row.try_get::<bool>("","active")?, "Removal must retain the inactive relationship record")?;
        let version = row.try_get::<chrono::NaiveDateTime>("","updated_at")?.to_string();
        let restore = super::super::platform::post_form(&app,&cookies,&format!("{path}/{generated_86504}/restore",generated_86504=relationship.id),
            &[("authenticity_token",&csrf),("version",&version)]).await;
        require(restore.status().as_u16()==403, "Relationship restoration must not recreate guardian/Manage authority")?;
        let row = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr(Expr::col("active")).expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("person_access_grants").and_where(Expr::col("person_id").eq(Expr::val(app.fixture.dependent.id)).and(Expr::col("revoked_at").is_null())).to_owned()),"grants").from("carer_relationships").and_where(Expr::col("id").eq(Expr::val(relationship.id))).to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Expected unchanged denied restoration".to_owned()))?;
        require(!row.try_get::<bool>("","active")? && row.try_get::<i64>("","grants")?==0,
            "Denied restore must preserve inactive record and no clinical authority")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn member_self_plan_write_does_not_authorise_another_participants_plan() -> Result<(), DbErr>
{
    let app = Application::new().await;
    let result = async {
        let care=seed_member_care(&app).await?;

        let token = app.token().await;
        let own = write_member_plan(&app,&token,app.fixture.assignment.id).await?;
        let other = write_member_plan(&app,&token,care.assignment.id).await?;
        require(own==200 && other==403,
            &format!("Verified self plan write must succeed, another participant must remain forbidden: self={own}, other={other}"))?;
        let state = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr_as(Expr::from(Query::select().expr(Expr::col("dose_amount").cast_as(Alias::new("text"))).from("person_medications").and_where(Expr::col("id").eq(Expr::val(app.fixture.assignment.id))).to_owned()),"own").expr_as(Expr::from(Query::select().expr(Expr::col("dose_amount").cast_as(Alias::new("text"))).from("person_medications").and_where(Expr::col("id").eq(Expr::val(care.assignment.id))).to_owned()),"other").expr_as(Expr::from(Query::select().expr(Expr::col("current_supply").cast_as(Alias::new("text"))).from("medications").and_where(Expr::col("id").eq(Expr::val(app.fixture.medication.id))).to_owned()),"supply").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("versions").and_where(Expr::col("item_type").eq(Expr::val("PersonMedication")).and(Expr::col("event").eq(Expr::val("update")))).and_where(Expr::col("household_id").eq(app.fixture.household.id)).to_owned()),"versions").to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Expected authorised plan effects".to_owned()))?;
        require(state.try_get::<String>("","own")?=="3.00"
            && state.try_get::<String>("","other")?=="2.00"
            && state.try_get::<String>("","supply")?=="10.00"
            && state.try_get::<i64>("","versions")?==1,
            "Only the self plan may change, with one audit and no stock deduction")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn member_manager_cannot_mint_manage_via_a_parent_label_grant() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let _care=seed_member_care(&app).await?;

        app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("household_memberships").value("role",Expr::val("administrator")).value("permissions_version",Expr::col("permissions_version").add(Expr::val(1))).and_where(Expr::col("id").eq(Expr::val(app.fixture.membership.id))).to_owned())).await?;
        let before = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr(Expr::col("permissions_version")).from("household_memberships").and_where(Expr::col("id").eq(Expr::val(app.fixture.membership.id))).to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Expected manager membership".to_owned()))?
            .try_get::<i32>("","permissions_version")?;
        let token = app.token().await;
        let response = app.client.post(format!("{}/api/v1/households/{generated_72001}/admin/person_access_grants",app.origin,generated_72001=app.fixture.household.id))
            .bearer_auth(&token).header("idempotency-key","synthetic-manager-is-not-guardian")
            .json(&json!({"person_access_grant":{"household_membership_id":app.fixture.membership.id,"person_id":app.fixture.dependent.id,"access_level":"manage","relationship_type":"parent"}}))
            .send().await.map_err(|_| DbErr::Custom("Manager grant request failed".to_owned()))?;
        let status = response.status().as_u16();
        let state = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr(Expr::col("permissions_version")).expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("person_access_grants").and_where(Expr::col("person_id").eq(Expr::val(app.fixture.dependent.id)).and(Expr::col("revoked_at").is_null())).to_owned()),"grants").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("carer_relationships").and_where(Expr::col("patient_id").eq(Expr::val(app.fixture.dependent.id)).and(Expr::col("active"))).to_owned()),"relationships").from("household_memberships").and_where(Expr::col("id").eq(Expr::val(app.fixture.membership.id))).to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Expected unchanged manager authority".to_owned()))?;
        let grants = state.try_get::<i64>("","grants")?;
        require(status==403 && state.try_get::<i32>("","permissions_version")?==before
            && state.try_get::<i64>("","grants")?==0 && state.try_get::<i64>("","relationships")?==0,
            &format!("Manager parent-label assertion must return403 without authority changes: status={status}, new_manage_grants={grants}"))?;
        let audit = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr_as(Expr::expr(Func::cust("count").args([Expr::col(("e","id"))])),"events").expr_as(Expr::expr(Func::cust("count").args([Expr::col(("l","id"))])),"ledger").expr_as(Expr::expr(Func::cust("count").args([Expr::col(("d","id"))])),"deliveries").from_as("security_audit_events","e").join_as(sea_orm::sea_query::JoinType::LeftJoin,"audit_ledger_entries","l",Expr::col(("l","source_table")).eq(Expr::val("security_audit_events")).and(Expr::col(("l","source_id")).eq(Expr::col(("e","id"))))).join_as(sea_orm::sea_query::JoinType::LeftJoin,"audit_export_deliveries","d",Expr::col(("d","audit_ledger_entry_id")).eq(Expr::col(("l","id")))).and_where(Expr::col(("e","household_id")).eq(Expr::val(app.fixture.household.id)).and(Expr::col(("e","actor_account_id")).eq(Expr::val(app.fixture.account.id))).and(Expr::col(("e","actor_membership_id")).eq(Expr::val(app.fixture.membership.id))).and(Expr::col(("e","event_type")).eq(Expr::val("household_access.person_grant_changed"))).and(Expr::expr(Func::cust("jsonb_extract_path_text").args([Expr::col(("e","metadata")),Expr::val("outcome")])).eq(Expr::val("rejected")))).to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Expected persisted grant rejection audit".to_owned()))?;
        require(audit.try_get::<i64>("","events")?==1 && audit.try_get::<i64>("","ledger")?==1
            && audit.try_get::<i64>("","deliveries")?==1,
            "Actual403 must retain one attributed grant rejection event, ledger entry and export delivery")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn member_manager_reads_retained_grants_without_departed_person_profile() -> Result<(), DbErr>
{
    let app = Application::new().await;
    let result = async {
        let _care=seed_member_care(&app).await?;

        manager(&app).await?;
        let now=chrono::Utc::now().naive_utc();
        let retained_grant=person_access_grants::ActiveModel {
            household_id:Set(app.fixture.household.id),household_membership_id:Set(app.fixture.membership.id),
            person_id:Set(app.fixture.dependent.id),access_level:Set("manage".into()),relationship_type:Set("parent".into()),
            revoked_at:Set(Some(now)),created_at:Set(now),updated_at:Set(now),..Default::default()
        }.insert(&app.fixture.admin).await?;
        revoke_participation(&app,app.fixture.dependent_membership.id).await?;
        people::Entity::update_many().col_expr(people::Column::Name,Expr::val("synthetic-future-departed-profile-marker"))
            .filter(people::Column::Id.eq(app.fixture.dependent.id)).exec(&app.fixture.admin).await?;

        let token = app.token().await;
        let response = app.client.get(format!("{}/api/v1/households/{generated_72001}/admin/person_access_grants",app.origin,generated_72001=app.fixture.household.id))
            .bearer_auth(&token).send().await
            .map_err(|_| DbErr::Custom("Retained grant read transport failed".to_owned()))?;
        let status = response.status().as_u16();
        let body: Value = response.json().await.map_err(|_| DbErr::Custom("Expected retained grant response".to_owned()))?;
        require(status==200, &format!("Retained local grant must remain readable after subject departure: status={status}"))?;
        let grant = body["data"].as_array().and_then(|rows| rows.iter().find(|row| row["id"]==retained_grant.id))
            .ok_or_else(|| DbErr::Custom("Retained revoked grant must not be silently omitted".to_owned()))?;
        require(grant["person_id"]==app.fixture.dependent.id && grant["household_membership_id"]==app.fixture.membership.id
            && grant["access_level"]=="manage" && grant["revoked_at"].is_string()
            && serde_json::Value::is_null(&grant["person_name"])
            && !body.to_string().contains("synthetic-future-departed-profile-marker"),
            "Grant identity and recorded state must survive without leaking the departed Person's future name")?;
        Ok::<_,DbErr>(body)
    }.await;
    app.close().await;
    let body = result?;
    let contract = super::super::contract::contract();
    let schema = &contract["components"]["schemas"]["PersonAccessGrantCollectionResponse"];
    super::super::contract::assert_value(contract, schema, &body, "retained grant collection");
    Ok(())
}

#[tokio::test]
async fn member_manager_cannot_mint_manage_via_relationship_delegation() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let _care=seed_member_care(&app).await?;

        app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("household_memberships").value("role",Expr::val("administrator")).value("permissions_version",Expr::col("permissions_version").add(Expr::val(1))).and_where(Expr::col("id").eq(Expr::val(app.fixture.membership.id))).to_owned())).await?;
        let tenant = access::begin(&app.fixture.runtime,&HouseholdScope {
            actor:Actor { account_id:app.fixture.account.id },household_id:app.fixture.household.id,request_id:"synthetic-manager-delegation".to_owned(),
        },None).await.map_err(|_| DbErr::Custom("Expected manager scope".to_owned()))?;
        let assigned = med_tracker::models::care::administration::delegation::assign(&tenant,app.fixture.adult.id,app.fixture.dependent.id,"parent",None).await;
        tenant.rollback().await.map_err(|_| DbErr::Custom("Unable to close delegation test scope".to_owned()))?;
        require(matches!(assigned,Err(med_tracker::models::errors::OperationError::Forbidden)),
            "Household manager must not turn relationship labels into guardian or shared-plan authority")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn member_rosters_include_current_participants_and_sources_without_grants()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let care=seed_member_care(&app).await?;
        let acceptance=&care.acceptance;
        let token = app.token().await;
        let people = read_member(&app, &token, "people").await?;
        require(people["data"].as_array().is_some_and(|rows| rows.len()==3
            && rows.iter().any(|row| row["id"]==app.fixture.dependent.id)
            && rows.iter().all(|row| row["id"]!=acceptance.inviter_account.person_id && row["id"]!=acceptance.invited_child.id)),
            "Member roster must include all local participants and exclude foreign linked/accountless People")?;
        for (path, id) in [("schedules",care.schedule.id),("person_medications",care.assignment.id),("medications",app.fixture.medication.id)] {
            let response = read_member(&app, &token, path).await?;
            require(response["data"].as_array().is_some_and(|rows| rows.iter().any(|row| row["id"]==id)),
                &format!("Member {path} roster must expose local care without a Person grant"))?;
        }
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn member_sync_snapshot_preserves_household_visibility_without_grants() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let _care=seed_member_care(&app).await?;

        let token = app.token().await;
        let visible = read_member(&app, &token, "people").await?;
        let allowed = visible["data"].as_array()
            .ok_or_else(|| DbErr::Custom("Expected current participant roster".to_owned()))?;
        let snapshot = read_member(&app, &token, "sync/snapshot").await?;
        let records = &snapshot["data"]["records"];
        require(records["people"].as_array().is_some_and(|rows| rows.len()==3
            && rows.iter().all(|row| allowed.iter().any(|person| person["portable_id"]==row["portable_id"]))),
            "Member snapshot must include all current local People and no foreign People without grants")?;
        require(records["schedules"].as_array().is_some_and(|rows| rows.len()==1)
            && records["person_medications"].as_array().is_some_and(|rows| rows.len()==2),
            "Member snapshot must retain both local treatment source kinds without grants")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn member_dashboard_offers_record_but_not_manage_for_another_participant() -> Result<(), DbErr>
{
    let app = Application::new().await;
    let result = async {
        let care = seed_member_care(&app).await?;

        let tenant = access::begin(
            &app.fixture.runtime,
            &HouseholdScope {
                actor: Actor {
                    account_id: app.fixture.account.id,
                },
                household_id: app.fixture.household.id,
                request_id: "synthetic-member-dashboard".to_owned(),
            },
            None,
        )
        .await
        .map_err(|_| DbErr::Custom("Expected ordinary member scope".to_owned()))?;
        let manageable = access::require_person_access(
            &tenant,
            app.fixture.dependent.id,
            access::PersonAccess::Manage,
        )
        .await;
        let dashboard = med_tracker::models::care::dashboard::read(
            &tenant,
            chrono_tz::UTC,
            Some(app.fixture.dependent.id),
            app.fixture.household.slug.as_str(),
            &json!({}),
        )
        .await;
        tenant
            .rollback()
            .await
            .map_err(|_| DbErr::Custom("Unable to close member dashboard scope".to_owned()))?;
        require(
            matches!(
                manageable,
                Err(med_tracker::models::errors::OperationError::Forbidden)
            ),
            "Ordinary membership must not grant shared-plan Manage",
        )?;
        let dashboard = dashboard.map_err(|_| {
            DbErr::Custom(
                "Member dashboard must resolve another local participant without a grant"
                    .to_owned(),
            )
        })?;
        require(
            dashboard["rows"].as_array().is_some_and(|rows| {
                rows.iter()
                    .any(|row| row["source_id"] == care.assignment.id && row["can_record"] == true)
            }),
            "Member dashboard must offer Record and keep Manage separate",
        )?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn member_records_another_participants_dose_once_without_a_grant() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let care=seed_member_care(&app).await?;

        let token = app.token().await;
        let body = json!({"medication_take":{"client_uuid":app.fixture.client_uuid.to_string(),
            "source_type":"person_medication","source_id":care.assignment.id.to_string().as_str(),"taken_at":"2026-10-05T10:00:00Z",
            "dose_amount":"2","dose_unit":"tablet","taken_from_medication_id":app.fixture.medication.id}});
        for expected in [201,200] {
            let response = app.client.post(format!("{}/api/v1/households/{generated_72001}/medication_takes", app.origin,generated_72001=app.fixture.household.id))
                .bearer_auth(&token).json(&body).send().await
                .map_err(|_| DbErr::Custom("Member dose request failed".to_owned()))?;
            require(response.status().as_u16()==expected, "Member must record and replay another participant's dose without a Person grant")?;
        }
        let state = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr_as(Expr::col("current_supply").cast_as(Alias::new("text")),"supply").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("medication_takes").and_where(Expr::col("person_medication_id").eq(Expr::val(care.assignment.id))).to_owned()),"takes").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("versions").and_where(Expr::col("item_type").eq(Expr::val("MedicationTake")).and(Expr::col("event").eq(Expr::val("create")))).and_where(Expr::col("household_id").eq(app.fixture.household.id)).to_owned()),"versions").from("medications").and_where(Expr::col("id").eq(Expr::val(app.fixture.medication.id))).to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Expected member dose effects".to_owned()))?;
        require(state.try_get::<String>("","supply")?=="8.00"
            && state.try_get::<i64>("","takes")?==1 && state.try_get::<i64>("","versions")?==1,
            "Member dose replay must preserve exactly one take, audit and stock deduction")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn member_membership_contract_covers_accountless_and_inactive_people() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let care=seed_member_care(&app).await?;
        let acceptance=&care.acceptance;
        manager(&app).await?;
        let linked=participate(&app,app.fixture.household.id,acceptance.inviter_account.person_id).await?;

        let token = app.token().await;
        let before = read_member(&app,&token,"admin/memberships").await?;
        let rows = before["data"].as_array()
            .ok_or_else(|| DbErr::Custom("Expected canonical membership collection".to_owned()))?;
        let accountless = rows.iter().find(|row| row["person_id"]==app.fixture.dependent.id)
            .ok_or_else(|| DbErr::Custom("Accountless participation must be represented".to_owned()))?;
        require(serde_json::Value::is_null(&accountless["account_id"]) && serde_json::Value::is_null(&accountless["email"])
            && serde_json::Value::is_null(&accountless["user_id"]) && accountless["person_name"].is_string(),
            "Accountless participant must retain clinical identity without fabricated Account/User fields")?;
        let linked_row = rows.iter().find(|row| row["id"]==linked.id)
            .ok_or_else(|| DbErr::Custom("Expected real account-linked participation".to_owned()))?;
        require(linked_row["account_id"]==acceptance.inviter_account.id && linked_row["email"].is_string() && linked_row["user_id"]==care.inviter_user.id,
            "Active linked participation must preserve actual Account and User IDs")?;
        let identity = identity_state(&app,acceptance).await?;
        for (person_id,expected) in [(Value::Null,422),(json!(app.fixture.dependent.id),422),(json!(acceptance.inviter_account.person_id),200)] {
            let response = app.client.patch(format!("{}/api/v1/households/{generated_72001}/admin/memberships/{generated_74505}",app.origin,generated_72001=app.fixture.household.id,generated_74505=linked.id))
                .bearer_auth(&token).json(&json!({"household_membership":{"person_id":person_id}}))
                .send().await.map_err(|_| DbErr::Custom("Membership identity contract transport failed".to_owned()))?;
            require(response.status().as_u16()==expected,
                "Membership must reject null/relink and preserve unchanged-ID echo compatibility")?;
        }
        require(read_member(&app,&token,"admin/memberships").await?==before
            && identity_state(&app,acceptance).await?==identity,
            "Null/relink denials and unchanged-ID echo must not mutate canonical identity or membership state")?;
        (async { app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("household_memberships").value("status",Expr::val("revoked")).value("revoked_at",Expr::expr(Func::cust("now"))).value("permissions_version",Expr::col("permissions_version").add(Expr::val(1))).and_where(Expr::col("id").eq(Expr::val(linked.id))).to_owned())).await?;
app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("accounts").value("name",Expr::val("synthetic-future-account-marker")).value("email",Expr::val(case_email(&app,"synthetic-future-account-marker@example.test").to_string())).and_where(Expr::col("id").eq(Expr::val(acceptance.inviter_account.id))).to_owned())).await?;
let result=app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("people").value("name",Expr::val("synthetic-future-person-marker")).and_where(Expr::col("id").eq(Expr::val(acceptance.inviter_account.person_id))).to_owned())).await?;Ok::<_,DbErr>(result) }).await?;
        let after = read_member(&app,&token,"admin/memberships").await?;
        let inactive = after["data"].as_array().and_then(|rows| rows.iter().find(|row| row["id"]==linked.id))
            .ok_or_else(|| DbErr::Custom("Inactive participation must not be omitted".to_owned()))?;
        require(inactive["person_id"]==acceptance.inviter_account.person_id && serde_json::Value::is_null(&inactive["account_id"])
            && serde_json::Value::is_null(&inactive["email"]) && inactive["user_id"]==care.inviter_user.id
            && serde_json::Value::is_null(&inactive["person_name"])
            && !after.to_string().contains("synthetic-future-account-marker")
            && !after.to_string().contains("synthetic-future-person-marker"),
            "Inactive participation must retain its actual historical User ID without future Account/profile visibility")?;
        Ok::<_,DbErr>((before,after))
    }.await;
    app.close().await;
    let (before, after) = result?;
    let contract = super::super::contract::contract();
    let schema = &contract["components"]["schemas"]["HouseholdMembershipCollectionResponse"];
    for body in [before, after] {
        super::super::contract::assert_value(
            contract,
            schema,
            &body,
            "canonical membership collection",
        );
    }
    Ok(())
}

#[tokio::test]
async fn member_membership_relationship_labels_preserve_retained_ids() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let care=seed_member_care(&app).await?;
        let acceptance=&care.acceptance;
        manager(&app).await?;
        let linked=participate(&app,app.fixture.household.id,acceptance.inviter_account.person_id).await?;
        let active_relationship=relationship(&app,acceptance.inviter_account.person_id,app.fixture.dependent.id,"parent",true).await?;
        let inactive_relationship=relationship(&app,acceptance.inviter_account.person_id,app.fixture.minor.id,"family_member",false).await?;
        revoke_participation(&app,linked.id).await?;
        revoke_participation(&app,app.fixture.dependent_membership.id).await?;
        people::Entity::update_many().col_expr(people::Column::Name,Expr::expr(Func::cust("concat").args([
            Expr::val("synthetic-retained-future-profile-marker-"),Expr::col(people::Column::Id).cast_as(Alias::new("text"))])))
            .filter(people::Column::Id.is_in([acceptance.inviter_account.person_id,app.fixture.dependent.id])).exec(&app.fixture.admin).await?;

        let cookie = super::super::platform::sign_in(&app,app.fixture.account.id,app.fixture.account.email.as_str()).await;
        let path = &format!("/households/{generated_slug}/admin/carer_relationships",generated_slug=app.fixture.household.slug);
        let response = app.client.get(format!("{}{path}",app.origin)).header("cookie",&cookie)
            .send().await.map_err(|_| DbErr::Custom("Retained admin relationship read failed".to_owned()))?;
        let status = response.status().as_u16();
        let cookies = super::super::platform::response_cookies(&cookie,&response);
        let body = response.text().await.map_err(|_| DbErr::Custom("Expected retained admin relationship page".to_owned()))?;
        require(status==200 && body.contains(&format!("#{generated_73103}",generated_73103=acceptance.inviter_account.person_id)) && body.contains(&format!("#{generated_73003}",generated_73003=app.fixture.dependent.id))
            && body.contains("Person not currently in this household")
            && !body.contains("synthetic-retained-future-profile-marker"),
            "Retained relationships need stable Person IDs and truthful unavailable labels without future profile reads")?;
        require(!body.contains("/admin/carer_relationships/new") && !body.contains(&format!("/{generated_86506}/activate",generated_86506=inactive_relationship.id)),
            "Read-only administration must not advertise unsupported relationship authority creation/restoration")?;
        let csrf = super::super::platform::hidden_field(&body,"authenticity_token");
        require(!csrf.is_empty(), "Retained relationship removal needs real CSRF")?;
        let removed = super::super::platform::post_form(&app,&cookies,&format!("{path}/{generated_86505}/deactivate",generated_86505=active_relationship.id),
            &[("authenticity_token",&csrf)]).await;
        require(removed.status().as_u16()==200, "Permitted manager deactivation must remain usable for retained relationships")?;
        let (_, refreshed) = super::super::platform::form_page(&app,&cookies,path).await;
        require(refreshed.contains(&format!("#{generated_73103}",generated_73103=acceptance.inviter_account.person_id)) && refreshed.contains(&format!("#{generated_73003}",generated_73003=app.fixture.dependent.id))
            && !refreshed.contains(&format!("/{generated_86505}/activate",generated_86505=active_relationship.id)) && !refreshed.contains("synthetic-retained-future-profile-marker"),
            "Deactivation refresh must preserve retained IDs without offering authority restoration")?;
        let row = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr(Expr::col("active")).from("carer_relationships").and_where(Expr::col("id").eq(Expr::val(active_relationship.id))).to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Retained relationship must not be deleted".to_owned()))?;
        require(!row.try_get::<bool>("","active")?, "Retained relationship must be inactive after permitted removal")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[derive(PartialEq, Eq)]
struct IdentityState {
    people: i64,
    accounts: i64,
    users: i64,
    api_sessions: i64,
    api_keys: i64,
    oauth_grants: i64,
    passwords: String,
    passkeys: String,
    providers: String,
    account_bindings: String,
}

async fn identity_state(
    app: &Application,
    acceptance: &Acceptance,
) -> Result<IdentityState, DbErr> {
    let households = [app.fixture.household.id, acceptance.target_household.id];
    let participants = Query::select()
        .column("person_id")
        .from("household_memberships")
        .and_where(Expr::col("household_id").is_in(households))
        .to_owned();
    let account_query = Query::select()
        .column("id")
        .from("accounts")
        .cond_where(
            Condition::any()
                .add(Expr::col("person_id").in_subquery(participants.clone()))
                .add(Expr::col("email").like(format!("%{}@example.test", app.fixture.client_uuid))),
        )
        .to_owned();
    let people = Query::select()
        .column("person_id")
        .from("accounts")
        .and_where(Expr::col("id").in_subquery(account_query.clone()))
        .to_owned();
    let persons = Condition::any()
        .add(Expr::col("id").in_subquery(participants))
        .add(Expr::col("id").in_subquery(people.clone()));
    let people = count_rows(app, "people", persons).await?;
    let accounts = count_rows(
        app,
        "accounts",
        Condition::all().add(Expr::col("id").in_subquery(account_query.clone())),
    )
    .await?;
    let users = count_rows(
        app,
        "users",
        Condition::all().add(Expr::col("person_id").in_subquery(people_query(app, acceptance))),
    )
    .await?;
    let api_sessions = count_rows(
        app,
        "api_sessions",
        Condition::all().add(Expr::col("account_id").in_subquery(account_query.clone())),
    )
    .await?;
    let api_keys = count_rows(
        app,
        "identity_api_keys",
        Condition::all().add(Expr::col("reference_id").in_subquery(account_query.clone())),
    )
    .await?;
    let oauth_grants = count_rows(
        app,
        "oauth_grants",
        Condition::all().add(Expr::col("account_id").in_subquery(account_query.clone())),
    )
    .await?;
    let credentials = scoped_rows(
        app,
        "auth_credentials",
        Condition::all().add(Expr::col("user_id").in_subquery(account_query.clone())),
    )
    .await?;
    let passwords = checksum(&credentials)?;
    let providers = checksum(&credentials)?;
    let passkeys = checksum(
        &scoped_rows(
            app,
            "identity_passkeys",
            Condition::all().add(Expr::col("account_id").in_subquery(account_query.clone())),
        )
        .await?,
    )?;
    let bindings = Query::select()
        .columns(["id", "person_id"])
        .from("accounts")
        .and_where(Expr::col("id").in_subquery(account_query))
        .order_by("id", Order::Asc)
        .to_owned();
    let account_bindings = app
        .fixture
        .admin
        .query_all_raw(app.fixture.admin.get_database_backend().build(&bindings))
        .await?
        .into_iter()
        .map(|row| {
            Ok((
                row.try_get::<i64>("", "id")?,
                row.try_get::<i64>("", "person_id")?,
            ))
        })
        .collect::<Result<Vec<_>, DbErr>>()?;
    Ok(IdentityState {
        people,
        accounts,
        users,
        api_sessions,
        api_keys,
        oauth_grants,
        passwords,
        passkeys,
        providers,
        account_bindings: checksum(&account_bindings)?,
    })
}
async fn accept(app: &Application, access: &str, token: &str) -> Result<(u16, Value), DbErr> {
    let response = app
        .client
        .post(format!("{}/api/v1/invitations/accept", app.origin))
        .bearer_auth(access)
        .json(&json!({"token":token}))
        .send()
        .await
        .map_err(|_| DbErr::Custom("Invitation HTTP transport failed".to_owned()))?;
    let status = response.status().as_u16();
    let body = response
        .json()
        .await
        .map_err(|_| DbErr::Custom("Invitation response was not JSON".to_owned()))?;
    Ok((status, body))
}

async fn acceptance_effects(app: &Application, acceptance: &Acceptance) -> Result<Vec<i64>, DbErr> {
    let household = acceptance.target_household.id;
    let mut effects = Vec::new();
    for table in [
        "household_memberships",
        "household_invitations",
        "versions",
        "security_audit_events",
        "api_change_events",
        "person_access_grants",
        "carer_relationships",
    ] {
        let mut condition = Condition::all().add(Expr::col("household_id").eq(household));
        if table == "household_invitations" {
            condition = condition
                .add(Expr::col("id").eq(acceptance.invitation.id))
                .add(Expr::col("accepted_at").is_not_null());
        }
        if table == "security_audit_events" {
            condition = condition.add(Expr::col("event_type").is_in([
                "household_access.membership_created",
                "household_access.person_grant_changed",
            ]));
        }
        effects.push(count_rows(app, table, condition).await?);
    }
    let query = Query::select()
        .expr_as(
            Func::cust("coalesce")
                .args([Expr::col("permissions_version").sum(), Expr::val(0)])
                .cast_as(Alias::new("bigint")),
            "count",
        )
        .from("household_memberships")
        .and_where(Expr::col("household_id").eq(household))
        .to_owned();
    effects.push(
        app.fixture
            .admin
            .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?
            .ok_or_else(|| DbErr::Custom("Expected invitation effects".into()))?
            .try_get("", "count")?,
    );
    effects.push(
        count_rows(
            app,
            "pg_loco_queue",
            Condition::all().add(
                Expr::expr(
                    Func::cust("jsonb_extract_path_text")
                        .args([Expr::col("task_data"), Expr::val("household_id")]),
                )
                .eq(household.to_string()),
            ),
        )
        .await?,
    );
    Ok(effects)
}
async fn seed_membership_invitation(app: &Application) -> Result<Acceptance, DbErr> {
    let acceptance = seed_acceptance(app).await;
    household_invitation_grants::Entity::delete_by_id(acceptance.invitation_grant.id)
        .exec(&app.fixture.admin)
        .await?;
    Ok(acceptance)
}
async fn claim_target_state(app: &Application, acceptance: &Acceptance) -> Result<Value, DbErr> {
    let person = people::Entity::find_by_id(acceptance.invited_child.id)
        .one(&app.fixture.admin)
        .await?
        .ok_or_else(|| DbErr::Custom("Expected existing claim target".into()))?;
    let participations = scoped_rows(
        app,
        "household_memberships",
        Condition::all().add(Expr::col("person_id").eq(person.id)),
    )
    .await?;
    let relationships = scoped_rows(
        app,
        "carer_relationships",
        Condition::any()
            .add(Expr::col("patient_id").eq(person.id))
            .add(Expr::col("carer_id").eq(person.id)),
    )
    .await?;
    let grants = scoped_rows(
        app,
        "person_access_grants",
        Condition::all().add(Expr::col("person_id").eq(person.id)),
    )
    .await?;
    Ok(
        json!({"person":person,"participations":participations,"relationships":relationships,"grants":grants}),
    )
}
async fn original_account_state(app: &Application) -> Result<String, DbErr> {
    let account = accounts::Entity::find_by_id(app.fixture.account.id)
        .one(&app.fixture.admin)
        .await?
        .ok_or_else(|| DbErr::Custom("Expected original Account".into()))?;
    checksum(&account)
}
#[tokio::test]
async fn existing_account_invitation_reuses_its_person_across_households() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let acceptance=seed_membership_invitation(&app).await?;
        let access=&acceptance.bearer;
        let before = identity_state(&app,&acceptance).await?;
        let original_person = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr_as(Expr::expr(Func::cust("to_jsonb").args([Expr::col("p")])),"state").from_as("people","p").and_where(Expr::col(("p","id")).eq(Expr::val(app.fixture.adult.id))).to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Expected original Person".to_owned()))?
            .try_get::<Value>("", "state")?;
        let (status, first) = accept(&app,access,&acceptance.token).await?;
        require(status == 200, "Valid existing-account invitation must be accepted")?;
        require(first["data"]["person_id"] == app.fixture.adult.id.to_string().as_str(),
            &format!("Invitation acceptance must reuse original Person {generated_73001}, not clone it",generated_73001=app.fixture.adult.id))?;
        require(first["data"]["role"] == "member", "Invitation must create an ordinary member")?;
        require(identity_state(&app,&acceptance).await? == before,
            "Joining must preserve Person/Account identity, credential counts, passwords, passkeys and provider links")?;
        let state = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr_as(Expr::from(Query::select().expr(Expr::expr(Func::cust("to_jsonb").args([Expr::col("p")]))).from_as("people","p").and_where(Expr::col(("p","id")).eq(Expr::val(app.fixture.adult.id))).to_owned()),"person").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("versions").and_where(Expr::col("item_type").eq(Expr::val("Person")).and(Expr::col("item_id").eq(Expr::val(app.fixture.adult.id)))).to_owned()),"versions").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("api_change_events").and_where(Expr::col("household_id").eq(Expr::val(acceptance.target_household.id)).and(Expr::col("record_type").eq(Expr::val("Person"))).and(Expr::col("record_id").eq(Expr::val(app.fixture.adult.id))).and(Expr::col("action").eq(Expr::val("create")))).to_owned()),"creates").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("api_change_events").and_where(Expr::col("household_id").eq(Expr::val(acceptance.target_household.id)).and(Expr::col("record_type").eq(Expr::val("Person"))).and(Expr::col("record_id").eq(Expr::val(app.fixture.adult.id))).and(Expr::col("action").eq(Expr::val("update")))).to_owned()),"updates").to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Expected participation visibility effect".to_owned()))?;
        require(state.try_get::<Value>("", "person")? == original_person
            && state.try_get::<i64>("", "versions")? == 0,
            "Joining must not mutate or fabricate creation history for the existing Person")?;
        require(state.try_get::<i64>("", "creates")? == 0
            && state.try_get::<i64>("", "updates")? == 1,
            "Joining must emit one supported update visibility notification, not a Person create")?;
        let mobile = app.token().await;
        let visible = app.client.get(format!("{}/api/v1/households/{generated_72002}/people/{generated_73001}", app.origin,generated_72002=acceptance.target_household.id,generated_73001=app.fixture.adult.id))
            .bearer_auth(&mobile).send().await.map_err(|_| DbErr::Custom("Expected joined household Person request".to_owned()))?;
        let visible_status = visible.status().as_u16();
        let visible: Value = visible.json().await.map_err(|_| DbErr::Custom("Expected joined Person response".to_owned()))?;
        require(visible_status == 200 && visible["data"]["id"] == app.fixture.adult.id,
            "The new household must resolve the original Person through the maintained HTTP contract")?;
        let membership = first["data"]["membership_id"].as_str()
            .and_then(|value| value.parse::<i64>().ok())
            .ok_or_else(|| DbErr::Custom("Expected accepted membership ID".to_owned()))?;
        let committed_effects = acceptance_effects(&app,&acceptance).await?;
        let (replay_status, replay) = accept(&app,access,&acceptance.token).await?;
        require(replay_status == 200 && first == replay,
            "Repeated acceptance must return the same membership and Person")?;
        require(identity_state(&app,&acceptance).await? == before,
            "Invitation replay must not duplicate identity")?;
        require(acceptance_effects(&app,&acceptance).await? == committed_effects,
            "Invitation replay must not duplicate membership, versions, audit, changes or jobs")?;
        let scope = HouseholdScope {
            actor: Actor { account_id: app.fixture.account.id },
            household_id: acceptance.target_household.id,
            request_id: "synthetic-shared-person-join".to_owned(),
        };
        let tenant = access::begin(&app.fixture.runtime, &scope, None).await
            .map_err(|_| DbErr::Custom("Accepted membership must provide B scope".to_owned()))?;
        require(tenant.membership().role == "member", "Accepted reader must not rely on manager authority")?;
        let visible = tenant.transaction().query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr_as(Expr::exists(Query::select().expr(Expr::val(1)).from("people").and_where(Expr::col("id").eq(Expr::val(app.fixture.adult.id))).to_owned()),"visible").expr_as(Expr::exists(Query::select().expr(Expr::val(1)).from("people").and_where(Expr::col("id").eq(Expr::val(app.fixture.minor.id))).to_owned()),"foreign").to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Expected subject visibility".to_owned()))?;
        let foreign: bool = visible.try_get("", "foreign")?;
        let visible: bool = visible.try_get("", "visible")?;
        require(visible, "B must see the original global Person")?;
        require(!foreign, "Joining B must not reveal A's other accountless subject")?;
        app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("household_memberships").value("status",Expr::val("revoked")).value("revoked_at",Expr::expr(Func::cust("now"))).value("permissions_version",Expr::col("permissions_version").add(Expr::val(1))).and_where(Expr::col("id").eq(Expr::val(membership))).to_owned())).await?;
        let recheck = access::recheck(&tenant).await;
        let retained_foreign = tenant.transaction().query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr_as(Expr::exists(Query::select().expr(Expr::val(1)).from("people").and_where(Expr::col("id").eq(Expr::val(acceptance.inviter_account.person_id))).to_owned()),"visible").to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Expected post-revocation visibility".to_owned()))?;
        let retained_foreign: bool = retained_foreign.try_get("", "visible")?;
        tenant.rollback().await
            .map_err(|_| DbErr::Custom("Unable to close accepted scope".to_owned()))?;
        require(matches!(recheck, Err(med_tracker::models::errors::OperationError::Forbidden)),
            "An already-open B scope must reject actor membership revocation")?;
        require(!retained_foreign, "Revoked B scope must not retain visibility of another B Person")?;
        let (revoked_status, _) = accept(&app,access,&acceptance.token).await?;
        require(revoked_status == 422, "Acceptance replay must not restore revoked membership")?;
        require(identity_state(&app,&acceptance).await? == before,
            "Revoked replay must not create replacement identity")?;
        require(matches!(access::begin(&app.fixture.runtime, &scope, None).await,
            Err(med_tracker::models::errors::OperationError::Forbidden)),
            "Revoked B membership must not authorise a fresh scope")?;
        let origin = access::begin(&app.fixture.runtime, &HouseholdScope {
            household_id: app.fixture.household.id, ..scope
        }, None).await
            .map_err(|_| DbErr::Custom("A membership must survive departure from B".to_owned()))?;
        origin.rollback().await
            .map_err(|_| DbErr::Custom("Unable to close origin scope".to_owned()))?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn clinical_rows_require_retained_household_participation() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let acceptance=seed_membership_invitation(&app).await?;
        let tenant = access::begin(&app.fixture.runtime, &HouseholdScope {
            actor: Actor { account_id: app.fixture.account.id }, household_id: app.fixture.household.id,
            request_id: "synthetic-clinical-participation-integrity".to_owned(),
        }, None).await.map_err(|_| DbErr::Custom("Expected restricted household scope".to_owned()))?;
        let attempted = tenant.transaction().begin().await?;
        let foreign = attempted.execute_raw(app.fixture.admin.get_database_backend().build(&Query::insert().into_table("person_medications").columns(["household_id","person_id","medication_id","dose_amount","dose_unit","position","created_at","updated_at"]).values_panic([Expr::val(app.fixture.household.id),Expr::val(acceptance.inviter_account.person_id),Expr::val(app.fixture.medication.id),Expr::val(1),Expr::val("tablet"),Expr::val(0),Expr::expr(Func::cust("now")),Expr::expr(Func::cust("now"))]).to_owned())).await;
        attempted.rollback().await?;
        require(matches!(foreign.as_ref().err().and_then(DbErr::sql_err),
            Some(SqlErr::ForeignKeyConstraintViolation(message)) if message.contains("fk_person_medications_person_id_household")),
            "Foreign clinical subject must fail the household participation FK with SQLSTATE 23503")?;
        let constraints = tenant.transaction().query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS count FROM pg_catalog.pg_constraint c
             WHERE c.conname IN ('fk_carer_relationships_carer_household','fk_carer_relationships_patient_household',
                 'fk_health_events_person_id_household','fk_household_invitation_grants_person_household',
                 'fk_location_memberships_person_id_household','fk_notification_events_person_id_household',
                 'fk_notification_preferences_person_id_household','fk_person_access_grants_person_id_household',
                 'fk_person_medications_person_id_household','fk_schedules_person_id_household','fk_review_prompts_person_household')
               AND c.contype='f' AND c.confrelid='public.household_memberships'::regclass
               AND c.confdeltype='a' AND c.convalidated
               AND (SELECT pg_catalog.array_agg(a.attname::text ORDER BY k.ordinality)
                    FROM pg_catalog.unnest(c.confkey) WITH ORDINALITY k(attnum,ordinality)
                    JOIN pg_catalog.pg_attribute a ON a.attrelid=c.confrelid AND a.attnum=k.attnum)
                   = ARRAY['person_id','household_id']"))
            .await?.ok_or_else(|| DbErr::Custom("Expected clinical participation constraints".to_owned()))?;
        require(constraints.try_get::<i64>("", "count")? == 11,
            "All eleven clinical references must preserve household participation and non-cascading retention")?;
        let now=chrono::Utc::now().naive_utc();
        let clinical=person_medications::ActiveModel {household_id:Set(app.fixture.household.id),
            person_id:Set(app.fixture.dependent.id),medication_id:Set(app.fixture.medication.id),
            dose_amount:Set(Some(sea_orm::prelude::Decimal::ONE)),dose_unit:Set(Some("tablet".into())),position:Set(0),
            created_at:Set(now),updated_at:Set(now),..Default::default()}.insert(tenant.transaction()).await?;

        tenant.commit().await.map_err(|_| DbErr::Custom("Expected valid participation reference commit".to_owned()))?;
        app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("household_memberships").value("status",Expr::val("revoked")).value("revoked_at",Expr::expr(Func::cust("now"))).value("permissions_version",Expr::col("permissions_version").add(Expr::val(1))).and_where(Expr::col("household_id").eq(Expr::val(app.fixture.household.id)).and(Expr::col("person_id").eq(Expr::val(app.fixture.dependent.id)))).to_owned())).await?;
        let tenant = access::begin(&app.fixture.runtime, &HouseholdScope {
            actor: Actor { account_id: app.fixture.account.id }, household_id: app.fixture.household.id,
            request_id: "synthetic-departed-clinical-reference".to_owned(),
        }, None).await.map_err(|_| DbErr::Custom("Expected retained origin household scope".to_owned()))?;
        let row = tenant.transaction().query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr(Expr::col("person_id")).expr(Expr::col("household_id")).from("person_medications").and_where(Expr::col("id").eq(Expr::val(clinical.id))).to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Departure must retain existing clinical origin reference".to_owned()))?;
        require(row.try_get::<i64>("", "person_id")? == app.fixture.dependent.id && row.try_get::<i64>("", "household_id")? == app.fixture.household.id,
            "Departure must not move or delete the recorded clinical reference")?;
        let attempted = tenant.transaction().begin().await?;
        let removal = attempted.execute_raw(app.fixture.admin.get_database_backend().build(&Query::delete().from_table("household_memberships").and_where(Expr::col("household_id").eq(Expr::val(app.fixture.household.id)).and(Expr::col("person_id").eq(Expr::val(app.fixture.dependent.id)))).to_owned())).await;
        attempted.rollback().await?;
        require(matches!(removal.as_ref().err().and_then(DbErr::sql_err),
            Some(SqlErr::ForeignKeyConstraintViolation(message)) if message.contains("fk_person_medications_person_id_household")),
            "Referenced departed participation must remain retained, never cascade clinical rows")?;
        tenant.rollback().await.map_err(|_| DbErr::Custom("Unable to close retained clinical reference scope".to_owned()))?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn invalid_invitation_or_inviter_never_partially_joins_an_existing_account()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let acceptance=seed_membership_invitation(&app).await?;
        let access=&acceptance.bearer;
        let identity = identity_state(&app,&acceptance).await?;
        for mutation in 0..4 {
            let mut invitation=household_invitations::Entity::find_by_id(acceptance.invitation.id).one(&app.fixture.admin).await?
                .ok_or_else(||DbErr::Custom("Expected generated invitation".into()))?.into_active_model();
            let now=chrono::Utc::now().naive_utc();
            match mutation {
                0=>{invitation.expires_at=Set(now-chrono::Duration::seconds(1));}
                1=>{invitation.expires_at=Set(now+chrono::Duration::days(1));invitation.revoked_at=Set(Some(now));}
                2=>{invitation.revoked_at=Set(None);}
                _=>{}
            }
            invitation.update(&app.fixture.admin).await?;
            if mutation>=2 {
                let mut inviter=household_memberships::Entity::find_by_id(acceptance.inviter_membership.id).one(&app.fixture.admin).await?
                    .ok_or_else(||DbErr::Custom("Expected current inviter".into()))?.into_active_model();
                inviter.role=Set(if mutation==2 {"member"} else {"owner"}.into());
                inviter.permissions_version=Set(inviter.permissions_version.unwrap()+1);
                if mutation==3 {inviter.status=Set("revoked".into());inviter.revoked_at=Set(Some(now));}
                inviter.update(&app.fixture.admin).await?;
            }
            let before = acceptance_effects(&app,&acceptance).await?;
            let (status, body) = accept(&app,access,&acceptance.token).await?;
            require(status == 422 && body["error"]["code"] == "invitation_unavailable",
                "Expired/revoked invitations and revoked/demoted inviters must be rejected")?;
            require(acceptance_effects(&app,&acceptance).await? == before,
                "Rejected invitation must not change membership, acceptance, audit or operational effects")?;
            require(identity_state(&app,&acceptance).await? == identity,
                "Rejected invitation must preserve all identity and credentials")?;
        }
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn acceptance_audit_failure_rolls_back_identity_and_membership_then_retry_commits_once()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let acceptance=seed_membership_invitation(&app).await?;
        let access=&acceptance.bearer;
        let identity = identity_state(&app,&acceptance).await?;
        let effects = acceptance_effects(&app,&acceptance).await?;
        app.fixture.admin.execute_unprepared(
            "CREATE SEQUENCE public.synthetic_global_join_audit_reached;
             GRANT USAGE,SELECT ON SEQUENCE public.synthetic_global_join_audit_reached TO med_tracker_app,med_tracker_owner;
             CREATE FUNCTION public.synthetic_reject_global_join_audit() RETURNS trigger LANGUAGE plpgsql AS $$
             BEGIN
               IF NEW.event_type='household_access.membership_created' THEN
                 PERFORM nextval('public.synthetic_global_join_audit_reached');
                 RAISE EXCEPTION 'synthetic join audit unavailable' USING ERRCODE='23514';
               END IF;
               RETURN NEW;
             END $$;
             CREATE TRIGGER synthetic_global_join_audit BEFORE INSERT ON public.security_audit_events
             FOR EACH ROW EXECUTE FUNCTION public.synthetic_reject_global_join_audit()"
        ).await?;
        let (status, body) = accept(&app,access,&acceptance.token).await?;
        let row = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr_as(Expr::col("is_called"),"reached").from("synthetic_global_join_audit_reached").to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Expected join audit marker".to_owned()))?;
        require(row.try_get("", "reached")?, "Forced acceptance audit failure must be reached")?;
        require(identity_state(&app,&acceptance).await? == identity,
            "Failed acceptance must roll back Person/Account/credential changes")?;
        require(acceptance_effects(&app,&acceptance).await? == effects,
            "Failed acceptance must roll back membership, accepted state, versions, audits, changes and jobs")?;
        require(status == 500 && body["error"]["code"] == "internal_error",
            "Audit failure must return the existing generic server failure")?;
        app.fixture.admin.execute_unprepared(
            "DROP TRIGGER synthetic_global_join_audit ON public.security_audit_events"
        ).await?;
        let (retry_status, retry) = accept(&app,access,&acceptance.token).await?;
        require(retry_status == 200 && retry["data"]["person_id"] == app.fixture.adult.id.to_string().as_str(),
            "Retry must join using the original Person")?;
        let committed = acceptance_effects(&app,&acceptance).await?;
        let row = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr_as(Expr::col(Asterisk).count(),"count").from("security_audit_events").and_where(Expr::col("household_id").eq(Expr::val(acceptance.target_household.id)).and(Expr::col("event_type").eq(Expr::val("household_access.membership_created")))).to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Expected membership acceptance audit".to_owned()))?;
        require(row.try_get::<i64>("", "count")? == 1,
            "Committed retry must produce exactly one membership audit")?;
        let (replay_status, replay) = accept(&app,access,&acceptance.token).await?;
        require(replay_status == 200 && replay == retry,
            "Committed retry must remain replayable")?;
        require(acceptance_effects(&app,&acceptance).await? == committed
            && identity_state(&app,&acceptance).await? == identity,
            "Replay after retry must not duplicate identity or effects")?;
        Ok(())
    }.await;
    let cleanup=app.fixture.admin.execute_unprepared("DROP TRIGGER IF EXISTS synthetic_global_join_audit ON public.security_audit_events; DROP FUNCTION IF EXISTS public.synthetic_reject_global_join_audit(); DROP SEQUENCE IF EXISTS public.synthetic_global_join_audit_reached").await;
    app.close().await;
    cleanup?;
    result
}

#[tokio::test]
async fn invitation_fields_cannot_claim_an_existing_accountless_person() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let acceptance=seed_membership_invitation(&app).await?;
        let access=&acceptance.bearer;
        app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("people").value("name",Expr::from(Query::select().expr(Expr::col("name")).from("people").and_where(Expr::col("id").eq(Expr::val(app.fixture.adult.id))).to_owned())).value("email",Expr::from(Query::select().expr(Expr::col("email")).from("accounts").and_where(Expr::col("id").eq(Expr::val(app.fixture.account.id))).to_owned())).and_where(Expr::col("id").eq(Expr::val(acceptance.invited_child.id))).to_owned())).await?;
        let identity = identity_state(&app,&acceptance).await?;
        let original_target = claim_target_state(&app,&acceptance).await?;
        let response = app.client.post(format!("{}/api/v1/invitations/accept", app.origin))
            .bearer_auth(access)
            .json(&json!({"token":acceptance.token.as_str(),"person_id":acceptance.invited_child.id.to_string().as_str(),
                "account_id":acceptance.inviter_account.id.to_string().as_str(),"dependent_ids":[acceptance.invited_child.id],"relationship_type":"parent"}))
            .send().await.map_err(|_| DbErr::Custom("Claim injection transport failed".to_owned()))?;
        let status = response.status().as_u16();
        let body: Value = response.json().await
            .map_err(|_| DbErr::Custom("Claim injection response was not JSON".to_owned()))?;
        require(status == 200 && body["data"]["person_id"] == app.fixture.adult.id.to_string().as_str(),
            "Valid invitation may join only the authenticated Account's existing Person")?;
        require(claim_target_state(&app,&acceptance).await? == original_target,
            "Matching identifiers or injected fields must preserve existing Person, participation, relationships and grants exactly")?;
        require(identity_state(&app,&acceptance).await? == identity,
            "Claim injection must not create or alter authentication identity")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn signup_ignores_injected_existing_person_and_creates_a_distinct_self() -> Result<(), DbErr>
{
    let app = Application::new().await;
    let result = async {
        let acceptance=seed_membership_invitation(&app).await?;
        (async { app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("household_invitations").value("email",Expr::val(case_email(&app,"synthetic-claim-signup@example.test").to_string())).and_where(Expr::col("id").eq(Expr::val(acceptance.invitation.id))).to_owned())).await?;
let result=app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("people").value("name",Expr::val("Synthetic matching signup")).value("email",Expr::val(case_email(&app,"synthetic-existing-contact@example.test").to_string())).and_where(Expr::col("id").eq(Expr::val(acceptance.invited_child.id))).to_owned())).await?;Ok::<_,DbErr>(result) }).await?;
        let target = claim_target_state(&app,&acceptance).await?;
        let original_account = original_account_state(&app).await?;
        let before = identity_state(&app,&acceptance).await?;
        let response = app.client.post(format!("{}/api/auth/onboarding/signup", app.origin))
            .header("origin", app.context.config.server.full_url())
            .json(&json!({"name":"Synthetic matching signup","email":&case_email(&app,"synthetic-claim-signup@example.test"),
                "date_of_birth":"1990-04-12","credential":"passkey",
                "invitation_token":acceptance.token.as_str(),"person_id":acceptance.invited_child.id,"account_id":app.fixture.account.id}))
            .send().await.map_err(|_| DbErr::Custom("Signup claim transport failed".to_owned()))?;
        let status = response.status().as_u16();
        let body: Value = response.json().await
            .map_err(|_| DbErr::Custom(format!("Signup claim HTTP {status} response was not JSON")))?;
        require(status == 200 && body["status"] == true,
            &format!("Valid signup must retain its existing ignored-field contract (HTTP {status})"))?;
        require(claim_target_state(&app,&acceptance).await? == target,
            "Signup must not adopt or alter the matching existing accountless Person or authority")?;
        require(original_account_state(&app).await? == original_account,
            "Injected Account ID must not change the original Account")?;
        let after = identity_state(&app,&acceptance).await?;
        require(after.accounts == before.accounts + 1 && after.people == before.people + 1
            && after.users == before.users + 1,
            "Legitimate signup must create exactly one distinct Account, Person and real User")?;
        require(after.api_sessions == before.api_sessions && after.api_keys == before.api_keys
            && after.oauth_grants == before.oauth_grants && after.passkeys == before.passkeys
            && after.providers == before.providers,
            "Passkey enrolment signup must not mint unrelated clinical credentials or provider links")?;
        let row = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr_as(Expr::col(("p","id")).ne(Expr::val(acceptance.invited_child.id)),"distinct_person").from_as("accounts","a").join_as(sea_orm::sea_query::JoinType::InnerJoin,"people","p",Expr::col(("p","id")).eq(Expr::col(("a","person_id")))).and_where(Expr::col(("a","email")).eq(Expr::val(case_email(&app,"synthetic-claim-signup@example.test").to_string())).and(Expr::col(("a","status")).eq(Expr::val(1)))).to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Expected new signup identity".to_owned()))?;
        require(row.try_get("", "distinct_person")?, "Signup must have a distinct new Person")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn profile_rejects_injected_person_and_account_ids_without_claiming() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result =
        async {
            let acceptance=seed_membership_invitation(&app).await?;
            people::Entity::update_many()
                .col_expr(people::Column::DateOfBirth, Expr::val(chrono::NaiveDate::from_ymd_opt(1985, 2, 3)))
                .filter(people::Column::Id.eq(app.fixture.adult.id))
                .exec(&app.fixture.admin).await?;
            let token = app.token().await;
            let endpoint = format!("{}/api/v1/households/{generated_72001}/profile", app.origin,generated_72001=app.fixture.household.id);
            let read = app
                .client
                .get(&endpoint)
                .bearer_auth(&token)
                .send()
                .await
                .map_err(|_| DbErr::Custom("Profile prerequisite transport failed".to_owned()))?;
            require(
                read.status().as_u16() == 200,
                "Existing profile must be readable before injection",
            )?;
            let profile: Value = read
                .json()
                .await
                .map_err(|_| DbErr::Custom("Profile prerequisite was not JSON".to_owned()))?;
            require(
                profile["data"]["person_id"] == app.fixture.adult.id.to_string().as_str() && profile["data"]["account_id"] == app.fixture.account.id.to_string().as_str(),
                "Profile prerequisite must resolve the actual caller identity",
            )?;
            let target = claim_target_state(&app,&acceptance).await?;
            let identity = identity_state(&app,&acceptance).await?;
            let original_account = original_account_state(&app).await?;
            let response = app.client.patch(&endpoint).bearer_auth(&token)
            .json(&json!({"profile":{"person_id":acceptance.invited_child.id.to_string().as_str(),"account_id":acceptance.inviter_account.id.to_string().as_str(),"time_zone":"UTC"}}))
            .send().await.map_err(|_| DbErr::Custom("Profile claim transport failed".to_owned()))?;
            let status = response.status().as_u16();
            let body: Value = response
                .json()
                .await
                .map_err(|_| DbErr::Custom("Profile claim response was not JSON".to_owned()))?;
            require(
                status == 422 && body["error"]["code"] == "unprocessable_content",
                "Profile must preserve unsupported-field rejection",
            )?;
            require(
                claim_target_state(&app,&acceptance).await? == target
                    && original_account_state(&app).await? == original_account
                    && identity_state(&app,&acceptance).await? == identity,
                "Rejected profile injection must preserve target authority and caller identity",
            )?;
            let fresh = app
                .client
                .get(&endpoint)
                .bearer_auth(&token)
                .send()
                .await
                .map_err(|_| DbErr::Custom("Fresh profile transport failed".to_owned()))?;
            require(
                fresh.status().as_u16() == 200,
                "Caller profile must remain available",
            )?;
            require(
                fresh
                    .json::<Value>()
                    .await
                    .map_err(|_| DbErr::Custom("Fresh profile was not JSON".to_owned()))?
                    == profile,
                "Rejected target IDs must not change the saved profile",
            )?;
            Ok(())
        }
        .await;
    app.close().await;
    result
}

async fn signup_control(target_email: &str, expected_status: u16) -> Result<(), DbErr> {
    let app = Application::new().await;
    let target_email = case_email(&app, target_email);
    let result = async {
        let acceptance=seed_membership_invitation(&app).await?;
        app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("people").value("name",Expr::val("Synthetic signup control")).value("email",Expr::val(target_email)).and_where(Expr::col("id").eq(Expr::val(acceptance.invited_child.id))).to_owned())).await?;
        let effects = acceptance_effects(&app,&acceptance).await?;
        app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("household_invitations").value("email",Expr::val(case_email(&app,"synthetic-signup-control@example.test").to_string())).and_where(Expr::col("id").eq(Expr::val(acceptance.invitation.id))).to_owned())).await?;
        let target = claim_target_state(&app,&acceptance).await?;
        let original_account = original_account_state(&app).await?;
        let before = identity_state(&app,&acceptance).await?;
        let response = app.client.post(format!("{}/api/auth/onboarding/signup", app.origin))
            .header("origin", app.context.config.server.full_url())
            .json(&json!({"name":"Synthetic signup control","email":&case_email(&app,"synthetic-signup-control@example.test"),
                "date_of_birth":"1990-04-12","credential":"passkey",
                "invitation_token":acceptance.token.as_str()}))
            .send().await.map_err(|_| DbErr::Custom("Signup control transport failed".to_owned()))?;
        let status = response.status().as_u16();
        require(claim_target_state(&app,&acceptance).await? == target
            && original_account_state(&app).await? == original_account,
            "Signup control must never attach or alter the existing target or Account")?;
        let after = identity_state(&app,&acceptance).await?;
        if status == 200 {
            require(after.accounts == before.accounts + 1 && after.people == before.people + 1
                && after.users == before.users + 1,
                "Valid distinct-email signup must create one new Account/Person/real User")?;
            let body: Value = response.json().await
                .map_err(|_| DbErr::Custom("Signup control success was not JSON".to_owned()))?;
            require(body["status"] == true, "Signup control must return normal success")?;
            let account=accounts::Entity::find().filter(accounts::Column::Email.eq(case_email(&app,"synthetic-signup-control@example.test")))
                .filter(accounts::Column::Status.eq(1)).one(&app.fixture.admin).await?
                .ok_or_else(||DbErr::Custom("Expected genuine signup provenance".into()))?;
            let user=users::Entity::find().filter(users::Column::PersonId.eq(account.person_id)).one(&app.fixture.admin).await?
                .ok_or_else(||DbErr::Custom("Expected real signup User".into()))?;
            let provenance=versions::Entity::find().filter(versions::Column::ItemType.eq("Person"))
                .filter(versions::Column::ItemId.eq(account.person_id)).filter(versions::Column::Event.eq("create"))
                .all(&app.fixture.admin).await?;
            let attributed=provenance.iter().all(|version|version.audit_context["actor_account_id"].to_string().trim_matches('"')==account.id.to_string()
                && version.audit_context["actor_user_id"].to_string().trim_matches('"')==user.id.to_string());
            let changes=api_change_events::Entity::find().filter(api_change_events::Column::HouseholdId.eq(acceptance.target_household.id))
                .filter(api_change_events::Column::RecordType.eq("Person")).filter(api_change_events::Column::RecordId.eq(account.person_id))
                .filter(api_change_events::Column::Action.eq("create")).count(&app.fixture.admin).await?;
            require(provenance.len()==1 && attributed && changes==1,
                "Genuine invited signup must preserve one actual User-attributed Person creation version and create notification")?;
        } else {
            require(after == before, "Rejected signup must roll back all identity and credential state")?;
            require(acceptance_effects(&app,&acceptance).await? == effects,
                "Rejected signup must leave invitation, memberships and acceptance effects unchanged")?;
        }
        require(status == expected_status,
            &format!("Signup control returned HTTP {status}, expected {expected_status}"))?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn signup_without_injection_matching_contact_email_preserves_existing_identity()
-> Result<(), DbErr> {
    signup_control("synthetic-signup-control@example.test", 500).await
}

#[tokio::test]
async fn signup_without_injection_distinct_contact_email_creates_new_identity() -> Result<(), DbErr>
{
    signup_control("synthetic-other-contact@example.test", 200).await
}

#[tokio::test]
async fn invited_signup_person_audit_failure_rolls_back_identity_and_acceptance()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let acceptance=seed_membership_invitation(&app).await?;
        household_invitations::Entity::update_many().col_expr(household_invitations::Column::Email,Expr::val(case_email(&app,"synthetic-audit-signup@example.test")))
            .filter(household_invitations::Column::Id.eq(acceptance.invitation.id)).exec(&app.fixture.admin).await?;
        app.fixture.admin.execute_unprepared(
            "CREATE SEQUENCE public.synthetic_signup_person_audit_reached;
             GRANT USAGE,SELECT ON SEQUENCE public.synthetic_signup_person_audit_reached TO med_tracker_app,med_tracker_owner;
             CREATE FUNCTION public.synthetic_reject_signup_person_audit() RETURNS trigger LANGUAGE plpgsql AS $$
             BEGIN
               IF NEW.item_type='Person' AND NEW.event='create' THEN
                 PERFORM nextval('public.synthetic_signup_person_audit_reached');
                 RAISE EXCEPTION 'synthetic person audit unavailable' USING ERRCODE='23514';
               END IF;
               RETURN NEW;
             END $$;
             CREATE TRIGGER synthetic_signup_person_audit BEFORE INSERT ON public.versions
             FOR EACH ROW EXECUTE FUNCTION public.synthetic_reject_signup_person_audit()"
        ).await?;
        let before = identity_state(&app,&acceptance).await?;
        let effects = acceptance_effects(&app,&acceptance).await?;
        let response = app.client.post(format!("{}/api/auth/onboarding/signup", app.origin))
            .header("origin", app.context.config.server.full_url())
            .json(&json!({"name":"Synthetic audited signup","email":&case_email(&app,"synthetic-audit-signup@example.test"),
                "date_of_birth":"1990-04-12","credential":"passkey",
                "invitation_token":acceptance.token.as_str()}))
            .send().await.map_err(|_| DbErr::Custom("Signup audit rollback transport failed".to_owned()))?;
        let status = response.status().as_u16();
        let body = response.text().await.map_err(|_| DbErr::Custom("Signup audit rollback response failed".to_owned()))?;
        let reached = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr_as(Expr::col("is_called"),"reached").from("synthetic_signup_person_audit_reached").to_owned()))
            .await?.ok_or_else(|| DbErr::Custom("Expected signup Person audit marker".to_owned()))?;
        require(reached.try_get::<bool>("", "reached")?, "Signup must attempt its genuine Person creation audit")?;
        require(identity_state(&app,&acceptance).await? == before
            && acceptance_effects(&app,&acceptance).await? == effects,
            "Person audit failure must atomically roll back Account, Person, actual User, participation, invitation, audit, changes and queued work")?;
        require(status == 500 && !body.contains("synthetic person audit unavailable"),
            "Signup audit failure must return a safe server error")?;
        Ok(())
    }.await;
    let cleanup=app.fixture.admin.execute_unprepared("DROP TRIGGER IF EXISTS synthetic_signup_person_audit ON public.versions; DROP FUNCTION IF EXISTS public.synthetic_reject_signup_person_audit(); DROP SEQUENCE IF EXISTS public.synthetic_signup_person_audit_reached").await;
    app.close().await;
    cleanup?;
    result
}

fn people_query(app: &Application, acceptance: &Acceptance) -> SelectStatement {
    let participants = Query::select()
        .column("person_id")
        .from("household_memberships")
        .and_where(
            Expr::col("household_id")
                .is_in([app.fixture.household.id, acceptance.target_household.id]),
        )
        .to_owned();
    let accounts = Query::select()
        .column("person_id")
        .from("accounts")
        .and_where(Expr::col("email").like(format!("%{}@example.test", app.fixture.client_uuid)))
        .to_owned();
    Query::select()
        .column("id")
        .from("people")
        .cond_where(
            Condition::any()
                .add(Expr::col("id").in_subquery(participants))
                .add(Expr::col("id").in_subquery(accounts)),
        )
        .to_owned()
}
async fn scoped_rows(
    app: &Application,
    table: &str,
    condition: Condition,
) -> Result<Vec<Value>, DbErr> {
    let query = Query::select()
        .expr_as(
            Func::cust("to_jsonb").arg(Expr::col(Alias::new("r"))),
            "record",
        )
        .from_as(Alias::new(table), "r")
        .cond_where(condition)
        .order_by("id", Order::Asc)
        .to_owned();
    app.fixture
        .admin
        .query_all_raw(app.fixture.admin.get_database_backend().build(&query))
        .await?
        .into_iter()
        .map(|row| row.try_get("", "record"))
        .collect()
}
async fn care_route_effects(app: &Application) -> Result<Vec<i64>, DbErr> {
    let mut effects = Vec::new();
    for table in [
        "carer_relationships",
        "person_access_grants",
        "household_invitations",
        "household_invitation_grants",
        "versions",
    ] {
        let mut condition =
            Condition::all().add(Expr::col("household_id").eq(app.fixture.household.id));
        if table == "carer_relationships" {
            condition = condition.add(Expr::col("active").eq(true));
        }
        effects.push(count_rows(app, table, condition).await?);
    }
    effects.push(
        count_rows(
            app,
            "pg_loco_queue",
            Condition::all().add(
                Expr::expr(
                    Func::cust("jsonb_extract_path_text")
                        .args([Expr::col("task_data"), Expr::val("household_id")]),
                )
                .eq(app.fixture.household.id.to_string()),
            ),
        )
        .await?,
    );
    Ok(effects)
}
async fn count_rows(app: &Application, table: &str, condition: Condition) -> Result<i64, DbErr> {
    let query = Query::select()
        .expr_as(Expr::col(Asterisk).count(), "count")
        .from(Alias::new(table))
        .cond_where(condition)
        .to_owned();
    app.fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await?
        .ok_or_else(|| DbErr::Custom("Expected scoped count".into()))?
        .try_get("", "count")
}
fn checksum(value: &impl serde::Serialize) -> Result<String, DbErr> {
    let bytes = serde_json::to_vec(value).map_err(|error| DbErr::Custom(error.to_string()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}
fn case_email(app: &Application, label: &str) -> String {
    format!(
        "{}-{}@example.test",
        label.trim_end_matches("@example.test"),
        app.fixture.client_uuid
    )
}

async fn reserved_id(app: &Application, table: &str) -> Result<i64, DbErr> {
    let query = Query::select()
        .expr_as(
            Func::cust("nextval").arg(
                Func::cust("pg_get_serial_sequence").args([Expr::val(table), Expr::val("id")]),
            ),
            "id",
        )
        .to_owned();
    app.fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await?
        .ok_or_else(|| DbErr::Custom("Expected reserved identity sequence".into()))?
        .try_get("", "id")
}
async fn set_runtime_value<C: ConnectionTrait>(
    connection: &C,
    key: &str,
    value: &str,
) -> Result<(), DbErr> {
    let query = Query::select()
        .expr(Func::cust("set_config").args([Expr::val(key), Expr::val(value), Expr::val(true)]))
        .to_owned();
    connection
        .execute_raw(connection.get_database_backend().build(&query))
        .await?;
    Ok(())
}
async fn runtime_context<C: ConnectionTrait>(
    connection: &C,
    account: i64,
    household: i64,
    membership: Option<i64>,
) -> Result<(), DbErr> {
    // SeaQuery has no SET LOCAL ROLE statement; this attack still uses a real restricted role.
    connection
        .execute_unprepared("SET LOCAL ROLE med_tracker_app")
        .await?;
    set_runtime_value(
        connection,
        "med_tracker.current_account_id",
        &account.to_string(),
    )
    .await?;
    set_runtime_value(
        connection,
        "med_tracker.current_household_id",
        &household.to_string(),
    )
    .await?;
    set_runtime_value(
        connection,
        "med_tracker.current_membership_id",
        &membership.map(|id| id.to_string()).unwrap_or_default(),
    )
    .await
}

async fn manager(app: &Application) -> Result<(), DbErr> {
    household_memberships::Entity::update_many()
        .col_expr(
            household_memberships::Column::Role,
            Expr::val("administrator"),
        )
        .col_expr(
            household_memberships::Column::PermissionsVersion,
            Expr::col(household_memberships::Column::PermissionsVersion).add(1),
        )
        .filter(household_memberships::Column::Id.eq(app.fixture.membership.id))
        .exec(&app.fixture.admin)
        .await?;
    Ok(())
}
async fn participate(
    app: &Application,
    household: i64,
    person: i64,
) -> Result<household_memberships::Model, DbErr> {
    let now = chrono::Utc::now().naive_utc();
    household_memberships::ActiveModel {
        household_id: Set(household),
        person_id: Set(person),
        role: Set("member".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
}
async fn relationship(
    app: &Application,
    carer: i64,
    patient: i64,
    kind: &str,
    active: bool,
) -> Result<carer_relationships::Model, DbErr> {
    let now = chrono::Utc::now().naive_utc();
    carer_relationships::ActiveModel {
        household_id: Set(app.fixture.household.id),
        carer_id: Set(carer),
        patient_id: Set(patient),
        relationship_type: Set(Some(kind.into())),
        active: Set(active),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
}
async fn revoke_participation(app: &Application, id: i64) -> Result<(), DbErr> {
    household_memberships::Entity::update_many()
        .col_expr(household_memberships::Column::Status, Expr::val("revoked"))
        .col_expr(
            household_memberships::Column::RevokedAt,
            Expr::val(chrono::Utc::now().naive_utc()),
        )
        .col_expr(
            household_memberships::Column::PermissionsVersion,
            Expr::col(household_memberships::Column::PermissionsVersion).add(1),
        )
        .filter(household_memberships::Column::Id.eq(id))
        .exec(&app.fixture.admin)
        .await?;
    Ok(())
}
