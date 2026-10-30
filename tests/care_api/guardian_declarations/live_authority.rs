use super::super::platform::{form_page, hidden_field, post_form, sign_in};
use super::{Application, effects, require, withdrawal::declaration};
use med_tracker::models::_entities::{
    accounts, audit_ledger_entries, carer_relationships, household_memberships, people,
    person_access_grants, schedules, security_audit_events, versions,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, Set, Statement, TransactionTrait,
    sea_query::{Alias, Expr, ExprTrait, Func, Query},
};
use serde_json::{Value, json};
use std::time::Duration;

fn schedule_body(app: &Application, subject: i64) -> Value {
    json!({"schedule":{"person_id":subject.to_string(),"medication_id":app.fixture.medication.id.to_string(),"dose_amount":"2","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},"dose_cycle":"daily"}})
}

async fn schedule(
    app: &Application,
    token: &str,
    subject: i64,
) -> Result<reqwest::Response, DbErr> {
    let household = app.fixture.household.id;
    app.client
        .post(format!(
            "{}/api/v1/households/{household}/schedules",
            app.origin
        ))
        .bearer_auth(token)
        .json(&schedule_body(app, subject))
        .send()
        .await
        .map_err(|_| DbErr::Custom("Live guardian plan request failed before response".into()))
}

async fn manageable(app: &Application, token: &str, subject: i64) -> Result<bool, DbErr> {
    let household = app.fixture.household.id;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{household}/ui_capabilities",
            app.origin
        ))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| {
            DbErr::Custom("Live guardian capability request failed before response".into())
        })?;
    require(
        response.status().as_u16() == 200,
        "Live authority capability read must remain available",
    )?;
    let body: Value = response
        .json()
        .await
        .map_err(|_| DbErr::Custom("Capability response must be JSON".into()))?;
    let ids = body["data"]["people"]["manage_ids"]
        .as_array()
        .ok_or_else(|| DbErr::Custom("Capability response must expose Manage IDs".into()))?;
    Ok(ids.contains(&json!(subject)))
}

async fn authority(
    app: &Application,
    token: &str,
    subject: i64,
    expected: bool,
) -> Result<(), DbErr> {
    require(
        manageable(app, token, subject).await? == expected,
        "Aggregate authority must follow current guardian eligibility",
    )?;
    let before = effects(app).await?;
    let response = schedule(app, token, subject).await?;
    require(
        response.status().as_u16() == if expected { 201 } else { 403 },
        &format!(
            "Direct plan authority must match aggregate eligibility; received{}",
            response.status().as_u16()
        ),
    )?;
    if !expected {
        require(
            effects(app).await? == before,
            "Denied plan writes must preserve clinical, version and queue state",
        )?;
    }
    Ok(())
}

async fn legacy_fixture(app: &Application) -> Result<(), DbErr> {
    let mut account = app.fixture.account.clone().into_active_model();
    let mut preferences = app.fixture.account.preferences.clone();
    preferences["time_zone"] = json!("UTC");
    account.preferences = Set(preferences);
    account.update(&app.fixture.admin).await?;
    let now = chrono::Utc::now().naive_utc();
    person_access_grants::ActiveModel {
        household_id: Set(app.fixture.household.id),
        household_membership_id: Set(app.fixture.membership.id),
        person_id: Set(app.fixture.dependent.id),
        access_level: Set("manage".into()),
        relationship_type: Set("family_member".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await?;
    Ok(())
}

async fn unchanged_alternatives(app: &Application, token: &str) -> Result<(), DbErr> {
    authority(app, token, app.fixture.adult.id, true).await?;
    authority(app, token, app.fixture.dependent.id, false).await
}

async fn declaration_state(app: &Application, id: i64) -> Result<Value, DbErr> {
    let query = Query::select()
        .column("subject_person_id")
        .column("revision")
        .expr_as(Expr::col("withdrawn_at").is_not_null(), "withdrawn")
        .from("guardian_declarations")
        .and_where(Expr::col("id").eq(id))
        .to_owned();
    let declaration = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await?
        .ok_or_else(|| DbErr::Custom("Expected committed guardian provenance".into()))?;
    let subject: i64 = declaration.try_get("", "subject_person_id")?;
    let link = carer_relationships::Entity::find()
        .filter(carer_relationships::Column::GuardianDeclarationId.eq(id))
        .filter(carer_relationships::Column::HouseholdId.eq(app.fixture.household.id))
        .one(&app.fixture.admin)
        .await?
        .ok_or_else(|| DbErr::Custom("Expected committed guardian provenance".into()))?;
    let withdrawals = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("GuardianDeclaration"))
        .filter(versions::Column::ItemId.eq(id))
        .filter(versions::Column::Event.eq("withdraw"))
        .count(&app.fixture.admin)
        .await?;
    let grants = person_access_grants::Entity::find()
        .filter(person_access_grants::Column::PersonId.eq(subject))
        .count(&app.fixture.admin)
        .await?;
    Ok(
        json!({"withdrawn":declaration.try_get::<bool>("","withdrawn")?,"revision":declaration.try_get::<i64>("","revision")?,
    "local_active":link.active,"withdrawals":withdrawals,"subject_grants":grants}),
    )
}

async fn withdraw(app: &Application, token: &str, id: i64) -> Result<reqwest::Response, DbErr> {
    app.client
        .patch(format!(
            "{}/api/v1/auth/guardian-declarations/{id}",
            app.origin
        ))
        .bearer_auth(token)
        .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
        .send()
        .await
        .map_err(|_| DbErr::Custom("Live guardian withdrawal failed before response".into()))
}

#[tokio::test]
async fn guardian_authority_stops_at_eighteenth_birthday_without_retyping() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let token = app.token().await;
        legacy_fixture(&app).await?;
        let (subject, id) = declaration(&app, &token).await?;
        birthday(&app, subject, true).await?;
        authority(&app, &token, subject, true).await?;
        unchanged_alternatives(&app, &token).await?;
        let before = declaration_state(&app, id).await?;
        birthday(&app, subject, false).await?;
        authority(&app, &token, subject, false).await?;
        unchanged_alternatives(&app, &token).await?;
        require(
            declaration_state(&app, id).await? == before,
            "Birthday must not withdraw or manufacture declaration authority",
        )?;
        let person = people::Entity::find_by_id(subject)
            .one(&app.fixture.admin)
            .await?
            .ok_or_else(|| DbErr::Custom("Birthday subject must remain present".into()))?;
        let accounts = accounts::Entity::find()
            .filter(accounts::Column::PersonId.eq(subject))
            .count(&app.fixture.admin)
            .await?;
        require(
            person.person_type == 1 && accounts == 0,
            "Eighteenth birthday must not silently retype or create an Account",
        )?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_capacity_loss_stops_derived_authority_only() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let token = app.token().await;
        legacy_fixture(&app).await?;
        let (subject, id) = declaration(&app, &token).await?;
        authority(&app, &token, subject, true).await?;
        unchanged_alternatives(&app, &token).await?;
        let before = declaration_state(&app, id).await?;
        people::Entity::update_many()
            .col_expr(people::Column::HasCapacity, Expr::val(false))
            .filter(people::Column::Id.eq(app.fixture.adult.id))
            .exec(&app.fixture.admin)
            .await?;
        authority(&app, &token, subject, false).await?;
        unchanged_alternatives(&app, &token).await?;
        require(
            declaration_state(&app, id).await? == before,
            "Capacity loss must not rewrite or withdraw personal provenance",
        )?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn household_manager_removes_local_link_without_withdrawing_personal_declaration()
-> Result<(), DbErr> {
    let mut app = Application::new().await;
    let result=async {
        app.client=reqwest::Client::builder().timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none()).build()
            .map_err(|_|DbErr::Custom("Owned no-redirect browser client unavailable".into()))?;
        let token=app.token().await;
        legacy_fixture(&app).await?;
        let (subject,id)=declaration(&app,&token).await?;
        authority(&app,&token,subject,true).await?;
let manager=crate::scenario::Scenario::new().await;
let now=chrono::Utc::now().naive_utc();
household_memberships::ActiveModel {
    household_id:Set(app.fixture.household.id),person_id:Set(manager.account.person_id),
    role:Set("administrator".into()),status:Set("active".into()),joined_at:Set(now),
    created_at:Set(now),updated_at:Set(now),..Default::default()
}.insert(&app.fixture.admin).await?;
let cookie=sign_in(&app,manager.account.id,&manager.account.email).await;
let slug=&app.fixture.household.slug;
        let (cookies,page)=form_page(&app,&cookie,&format!("/households/{slug}/people/{subject}/carer_relationships")).await;
        let csrf=hidden_field(&page,"authenticity_token");
        let version=hidden_field(&page,"version");
        require(!csrf.is_empty() && !version.is_empty(),"Local manager must reach the actual protected removal form")?;
let link=carer_relationships::Entity::find().filter(carer_relationships::Column::GuardianDeclarationId.eq(id))
    .filter(carer_relationships::Column::HouseholdId.eq(app.fixture.household.id)).one(&app.fixture.admin).await?
    .ok_or_else(||DbErr::Custom("Expected active local declaration link".into()))?.id;

        let removed=post_form(&app,&cookies,&format!("/households/{slug}/people/{subject}/carer_relationships/{link}/remove"),
            &[("authenticity_token",csrf.as_str()),("version",version.as_str()),("locale","en")]).await;
        require(removed.status().as_u16()==303,"A different live household manager must be able to remove the local care link")?;
        manager.close().await;
        authority(&app,&token,subject,false).await?;
        unchanged_alternatives(&app,&token).await?;
        let state=declaration_state(&app,id).await?;
        require(state["local_active"]==false && state["withdrawn"]==false && state["revision"]==1
            && state["withdrawals"]==0 && state["subject_grants"]==0,
            "Manager removal must end only the local link, not personal declaration or revive legacy authority")?;
        let own=app.client.get(format!("{}/api/v1/auth/guardian-declarations",app.origin)).bearer_auth(&token).send().await
            .map_err(|_|DbErr::Custom("Own declaration refresh failed before response".into()))?;
        require(own.status().as_u16()==200,"Guardian must retain own declaration read after local removal")?;
        let own:Value=own.json().await.map_err(|_|DbErr::Custom("Own declaration refresh must be JSON".into()))?;
        let expected_id=id.to_string();
        require(own["data"].as_array().is_some_and(|rows|rows.iter().any(|row|row["id"].as_str()==Some(expected_id.as_str()) && serde_json::Value::is_null(&row["withdrawn_at"]))),
            "Personal declaration must remain Recorded after household-local removal")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn committed_withdrawal_denies_subsequent_plan_without_effects() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let token = app.token().await;
        legacy_fixture(&app).await?;
        let (subject, id) = declaration(&app, &token).await?;
        authority(&app, &token, subject, true).await?;
        let withdrawn = withdraw(&app, &token, id).await?;
        require(
            withdrawn.status().as_u16() == 200,
            "Guardian withdrawal must commit before the next plan request",
        )?;
        let committed = declaration_state(&app, id).await?;
        require(
            committed["withdrawn"] == true
                && committed["revision"] == 2
                && committed["withdrawals"] == 1,
            "Withdrawal-first scenario must reach its persisted terminal event",
        )?;
        authority(&app, &token, subject, false).await?;
        require(
            declaration_state(&app, id).await? == committed,
            "Denied subsequent plan must preserve the committed withdrawal",
        )?;
        unchanged_alternatives(&app, &token).await?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

struct PendingRequest(tokio::task::JoinHandle<Result<reqwest::Response, reqwest::Error>>);

impl PendingRequest {
    async fn finish(&mut self) -> Result<reqwest::Response, DbErr> {
        tokio::time::timeout(Duration::from_secs(5), &mut self.0)
            .await
            .map_err(|_| {
                DbErr::Custom(
                    "Owned concurrent request exceeded its bounded completion window".into(),
                )
            })?
            .map_err(|_| DbErr::Custom("Owned concurrent request task did not complete".into()))?
            .map_err(|_| DbErr::Custom("Owned concurrent request failed before response".into()))
    }
}

impl Drop for PendingRequest {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn blocked_waiter(app: &Application, blocker: i32) -> Result<i32, DbErr> {
    for _ in 0..100 {
        let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1::int=ANY(pg_blocking_pids(pid)) ORDER BY pid LIMIT 1",
            [blocker.into()])).await?;
        if let Some(row) = row {
            return row.try_get("", "pid");
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    Err(DbErr::Custom(
        "Expected deterministic PostgreSQL blocking witness was not reached".into(),
    ))
}

async fn second_native_token(app: &Application) -> Result<String, DbErr> {
    super::security::native_token(app, app.fixture.account.id).await
}

#[tokio::test]
async fn plan_declaration_share_blocks_withdrawal_until_plan_and_audit_commit() -> Result<(), DbErr>
{
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        legacy_fixture(&app).await?;
        let (subject,id)=declaration(&app,&token).await?;
        authority(&app,&token,subject,true).await?;
        unchanged_alternatives(&app,&token).await?;
        let withdrawal_token=second_native_token(&app).await?;
        let before=effects(&app).await?;
        app.fixture.admin.execute_unprepared(
            "CREATE FUNCTION public.synthetic_guardian_plan_audit_gate() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type IN ('Schedule','PersonSchedule') AND NEW.event='create' THEN PERFORM pg_advisory_xact_lock(1746879441,93); END IF; RETURN NEW; END $$;
             CREATE TRIGGER synthetic_guardian_plan_audit_gate BEFORE INSERT ON public.versions FOR EACH ROW EXECUTE FUNCTION public.synthetic_guardian_plan_audit_gate()"
        ).await?;
        let gate=app.fixture.admin.begin().await?;
        let gate_pid=gate.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT pg_backend_pid() AS pid"))
            .await?.ok_or_else(||DbErr::Custom("Owned gate backend PID missing".into()))?.try_get::<i32>("","pid")?;
        gate.execute_unprepared("SELECT pg_advisory_xact_lock(1746879441,93)").await?;
        let client=app.client.clone();
        let endpoint=format!("{}/api/v1/households/{}/schedules",app.origin,app.fixture.household.id);
        let plan_token=token.clone();
        let body=schedule_body(&app, subject);
        let mut plan=PendingRequest(tokio::spawn(async move {client.post(endpoint).bearer_auth(plan_token).json(&body).send().await}));
        let plan_pid=blocked_waiter(&app,gate_pid).await?;
        let client=app.client.clone();
        let endpoint=format!("{}/api/v1/auth/guardian-declarations/{id}",app.origin);
        let mut withdrawal=PendingRequest(tokio::spawn(async move {client.patch(endpoint).bearer_auth(withdrawal_token)
            .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}})).send().await}));
        let withdrawal_pid=blocked_waiter(&app,plan_pid).await?;
        let locks=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM pg_locks WHERE pid=$1 AND relation='public.guardian_declarations'::regclass AND mode='RowShareLock' AND granted) AS declaration_reached,
              EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='public.security_audit_events'::regclass AND confrelid='public.accounts'::regclass AND contype='f') AS audit_account_fk",
            [withdrawal_pid.into()])).await?.ok_or_else(||DbErr::Custom("Expected declaration/audit lock facts".into()))?;
        require(locks.try_get::<bool>("","declaration_reached")? && locks.try_get::<bool>("","audit_account_fk")?,
            "Independent withdrawal must reach declaration locking while the plan retains real Account audit FK semantics")?;
        require(!plan.0.is_finished() && !withdrawal.0.is_finished(),"Both actual requests must be pending before releasing the plan audit")?;
        require(declaration_state(&app,id).await?["withdrawn"]==false,"Blocked withdrawal must not become visible before plan commit")?;
        gate.commit().await?;
        let saved=plan.finish().await?;
        require(saved.status().as_u16()==201,"Plan holding declaration SHARE must finish with its audit before withdrawal")?;
        let plan_request=saved.headers().get("x-request-id").and_then(|value|value.to_str().ok())
            .filter(|value|!value.is_empty()).ok_or_else(||DbErr::Custom("Committed plan request ID missing".into()))?.to_owned();
        let saved:Value=saved.json().await.map_err(|_|DbErr::Custom("Concurrent plan result must be JSON".into()))?;
        let plan_id=saved["data"]["id"].as_i64().ok_or_else(||DbErr::Custom("Concurrent plan ID missing".into()))?;
        let withdrawn=withdrawal.finish().await?;
        require(withdrawn.status().as_u16()==200,"Waiting withdrawal must commit after the authorised plan and audit")?;
        let version_ids=Query::select().column("id").from("versions")
            .and_where(Expr::col("item_type").is_in(["Schedule","PersonSchedule","SharedPersonSchedule"]))
            .and_where(Expr::col("item_id").is_in([plan_id,subject]))
            .and_where(Expr::col("event").is_in(["create","update"])).to_owned();
        let plan_versions=versions::Entity::find().filter(versions::Column::Id.in_subquery(version_ids.clone()))
            .count(&app.fixture.admin).await?;
        let canonical_versions=versions::Entity::find()
            .filter(versions::Column::ItemType.is_in(["PersonSchedule","SharedPersonSchedule"]))
            .filter(versions::Column::ItemId.is_in([plan_id,subject]))
            .filter(versions::Column::Event.eq("create"))
            .count(&app.fixture.admin).await?;
        let execution_versions=versions::Entity::find()
            .filter(versions::Column::ItemType.eq("Schedule"))
            .filter(versions::Column::ItemId.eq(plan_id))
            .filter(versions::Column::Event.eq("create"))
            .count(&app.fixture.admin).await?;
        let plan_ledger=audit_ledger_entries::Entity::find()
            .filter(audit_ledger_entries::Column::SourceTable.eq("versions"))
            .filter(audit_ledger_entries::Column::SourceId.in_subquery(version_ids))
            .count(&app.fixture.admin).await?;
        let plans=schedules::Entity::find_by_id(plan_id).filter(schedules::Column::PersonId.eq(subject))
            .count(&app.fixture.admin).await?;
        let account_fk_audits=security_audit_events::Entity::find()
            .filter(security_audit_events::Column::RequestId.eq(&plan_request))
            .filter(security_audit_events::Column::ActorAccountId.eq(app.fixture.account.id))
            .filter(security_audit_events::Column::EventType.eq("api.request"))
            .filter(Expr::expr(Func::cust("jsonb_extract_path_text").args([
                Expr::col(security_audit_events::Column::Metadata),Expr::val("status")])).eq("201"))
            .count(&app.fixture.admin).await?;
        require(plans==1 && plan_versions==2 && plan_ledger==2 && canonical_versions==1 && execution_versions==1 && account_fk_audits==1,
            &format!("The authorised plan, version/ledger and real Account-FK request audit must remain committed after withdrawal: plans={plans}, plan_versions={plan_versions}, plan_ledger={plan_ledger}, canonical_versions={canonical_versions}, execution_versions={execution_versions}, account_fk_audits={account_fk_audits}"))?;
        let state=declaration_state(&app,id).await?;
        require(state["withdrawn"]==true && state["withdrawals"]==1,"Concurrent withdrawal must commit exactly one terminal event")?;
        require(effects(&app).await?["people"]==before["people"],"Concurrent authority transition must not manufacture identities")?;
        authority(&app,&token,subject,false).await?;
        Ok(())
    }.await;
    let cleanup = app
        .fixture
        .admin
        .execute_unprepared(
            "DROP TRIGGER IF EXISTS synthetic_guardian_plan_audit_gate ON public.versions;
     DROP FUNCTION IF EXISTS public.synthetic_guardian_plan_audit_gate()",
        )
        .await;
    app.close().await;
    cleanup?;

    result
}

async fn birthday(app: &Application, subject: i64, tomorrow: bool) -> Result<(), DbErr> {
    let today = Expr::expr(
        Func::cust("timezone").args([Expr::val("UTC"), Expr::expr(Func::cust("clock_timestamp"))]),
    )
    .cast_as(Alias::new("date"));
    let birthday = today.sub(Func::cust("make_interval").arg(Expr::val(18)));
    let birthday = if tomorrow {
        birthday.add(Func::cust("make_interval").args([
            Expr::val(0),
            Expr::val(0),
            Expr::val(0),
            Expr::val(1),
        ]))
    } else {
        birthday
    };
    people::Entity::update_many()
        .col_expr(
            people::Column::DateOfBirth,
            birthday.cast_as(Alias::new("date")),
        )
        .filter(people::Column::Id.eq(subject))
        .exec(&app.fixture.admin)
        .await?;
    Ok(())
}
