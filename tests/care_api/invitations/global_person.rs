use super::{Application, seed_acceptance};
use med_tracker::models::access::{self, Actor, HouseholdScope};
use sea_orm::{ConnectionTrait, DbBackend, DbErr, SqlErr, Statement, TransactionTrait};
use serde_json::{Value, json};

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
        seed_member_care(&app).await?;
        let tenant = access::begin(&app.fixture.runtime,&HouseholdScope {
            actor:Actor{account_id:71001},household_id:72001,
            request_id:"synthetic-canonical-relation".to_owned()},None).await
            .map_err(|_|DbErr::Custom("Expected member relation scope".to_owned()))?;
        let joined = person_medication::Entity::find_by_id(81504)
            .find_also_related(membership::Entity).one(tenant.transaction()).await?;
        tenant.rollback().await.map_err(|_|DbErr::Custom("Unable to close relation scope".to_owned()))?;
        require(joined.is_some_and(|(source, participation)| source.person_id==73003
            && participation.is_some_and(|member|member.id==74981 && member.person_id==source.person_id
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
        seed_membership_invitation(&app).await?;
        app.fixture.admin.execute_unprepared("INSERT INTO public.platform_admins(account_id,status,created_at,updated_at) VALUES(71001,'active',now(),now())").await?;
        let catalogue = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT p.prosecdef AND r.rolname='med_tracker_owner' AND NOT r.rolcanlogin AND NOT r.rolsuper AND NOT r.rolbypassrls AND p.proconfig @> ARRAY['search_path=pg_catalog'] AND has_function_privilege('med_tracker_app',p.oid,'EXECUTE') AND NOT EXISTS(SELECT 1 FROM aclexplode(COALESCE(p.proacl,acldefault('f',p.proowner))) acl WHERE acl.grantee=0 AND acl.privilege_type='EXECUTE') AND (SELECT relrowsecurity AND relforcerowsecurity FROM pg_class WHERE oid='public.people'::regclass) AS secured FROM pg_proc p JOIN pg_roles r ON r.oid=p.proowner WHERE p.oid='med_tracker.recovery_member_eligible(bigint)'::regprocedure"))
            .await?.ok_or_else(||DbErr::Custom("Expected recovery capability catalogue".to_owned()))?;
        require(catalogue.try_get("","secured")?, "Recovery capability must retain fixed non-login owner, RLS, search path and least-privileged ACL")?;
        let transaction = app.fixture.runtime.begin().await?;
        transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app; SELECT set_config('med_tracker.current_account_id','71001',true),set_config('med_tracker.current_household_id','72002',true),set_config('med_tracker.current_membership_id','',true)").await?;
        let facts = transaction.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT med_tracker.recovery_member_eligible(74002) AS eligible,med_tracker.recovery_member_eligible(74001) AS foreign,med_tracker.recovery_member_eligible(999999) AS missing,EXISTS(SELECT 1 FROM public.people WHERE id=73103) AS directly_visible"))
            .await?.ok_or_else(||DbErr::Custom("Expected restricted recovery facts".to_owned()))?;
        require(facts.try_get("","eligible")? && !facts.try_get::<bool>("","foreign")?
            && !facts.try_get::<bool>("","missing")? && !facts.try_get::<bool>("","directly_visible")?,
            "A non-member administrator may inspect only eligibility, without direct Person roster visibility or recursion")?;
        for account in ["", "71002"] {
            transaction.execute_unprepared("SAVEPOINT invalid_recovery_context").await?;
            transaction.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                "SELECT set_config('med_tracker.current_account_id',$1,true)",[account.into()])).await?;
            let denied = transaction.query_one_raw(Statement::from_string(DbBackend::Postgres,
                "SELECT med_tracker.recovery_member_eligible(74002)" )).await;
            transaction.execute_unprepared("ROLLBACK TO SAVEPOINT invalid_recovery_context; RELEASE SAVEPOINT invalid_recovery_context").await?;
            require(denied.as_ref().err().is_some_and(|error|database_code(error,"42501")),
                "Missing or non-admin context must fail closed with insufficient privilege")?;
        }
        transaction.rollback().await?;
        app.fixture.admin.execute_unprepared("UPDATE public.platform_admins SET status='revoked' WHERE account_id=71001").await?;
        let denied = app.fixture.runtime.begin().await?;
        denied.execute_unprepared("SET LOCAL ROLE med_tracker_app; SELECT set_config('med_tracker.current_account_id','71001',true),set_config('med_tracker.current_household_id','72002',true)").await?;
        let result = denied.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT med_tracker.recovery_member_eligible(74002)")).await;
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
        for (sql,code) in [
            ("INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES(71951,NULL,'synthetic-null-person@example.test',2,now(),now())", "23502"),
            ("INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES(71951,73001,'synthetic-duplicate-person@example.test',2,now(),now())", "23505"),
            ("INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74951,72001,NULL,'member','active',now(),now(),now())", "23502"),
            ("INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74951,72001,73001,'member','active',now(),now(),now())", "23505"),
        ] {
            let transaction = app.fixture.runtime.begin().await?;
            transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app; SELECT set_config('med_tracker.current_account_id','71001',true),set_config('med_tracker.current_household_id','72001',true),set_config('med_tracker.current_membership_id','74001',true)").await?;
            let operation = transaction.execute_unprepared(sql).await;
            transaction.rollback().await?;
            require(operation.as_ref().err().is_some_and(|error|database_code(error,code)),
                "Canonical identity cardinality must be enforced by the database")?;
        }
        let dangling = app.fixture.runtime.begin().await?;
        dangling.execute_unprepared("SET LOCAL ROLE med_tracker_app; INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES(71951,73951,'synthetic-deferred-person@example.test',2,now(),now())").await?;
        let commit = dangling.commit().await;
        require(commit.as_ref().err().is_some_and(|error|database_code(error,"23503")),
            "Missing reserved Person must reject the deferred Account commit")?;
        let valid = app.fixture.runtime.begin().await?;
        valid.execute_unprepared("SET LOCAL ROLE med_tracker_app; INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES(71951,73951,'synthetic-deferred-person@example.test',2,now(),now()); SELECT set_config('med_tracker.current_account_id','71951',true); INSERT INTO public.people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73951,'Synthetic reserved self',0,true,now(),now())").await?;
        valid.commit().await?;
        let rollback = app.fixture.runtime.begin().await?;
        rollback.execute_unprepared("SET LOCAL ROLE med_tracker_app; INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES(71952,73952,'synthetic-rolled-back-person@example.test',2,now(),now()); SELECT set_config('med_tracker.current_account_id','71952',true); INSERT INTO public.people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73952,'Synthetic rolled back self',0,true,now(),now())").await?;
        rollback.rollback().await?;
        let persisted = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM public.accounts WHERE id=71951 AND person_id=73951) AND EXISTS(SELECT 1 FROM public.people WHERE id=73951) AND NOT EXISTS(SELECT 1 FROM public.accounts WHERE id=71952) AND NOT EXISTS(SELECT 1 FROM public.people WHERE id=73952) AS atomic"))
            .await?.ok_or_else(||DbErr::Custom("Expected canonical identity commit state".to_owned()))?;
        require(persisted.try_get("","atomic")?, "Account and reserved Person must commit or roll back together")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

async fn seed_member_care(app: &Application) -> Result<(), DbErr> {
    seed_membership_invitation(app).await?;
    app.fixture.admin.execute_unprepared(
        "UPDATE public.household_memberships SET role='member',permissions_version=permissions_version+1 WHERE id=74001;
         UPDATE public.person_access_grants SET revoked_at=now() WHERE household_membership_id=74001;
         INSERT INTO public.person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at)
         VALUES(81504,72001,73003,80001,2,'tablet',1,now(),now());
         INSERT INTO public.schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,frequency,created_at,updated_at)
         VALUES(83504,72001,73003,80001,true,'2020-01-01','2099-12-31',2,'tablet',4,'{}','As needed',now(),now())"
    ).await?;
    Ok(())
}

#[tokio::test]
async fn canonical_invitation_browser_offers_membership_without_care_authority() -> Result<(), DbErr>
{
    let app = Application::new().await;
    let result=async {
        seed_member_care(&app).await?;
        app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='administrator',permissions_version=permissions_version+1 WHERE id=74001").await?;
        let cookie=super::super::platform::sign_in(&app,71001,"persistence@example.test").await;
        let (_,html)=super::super::platform::form_page(&app,&cookie,"/households/persistence-fixture/admin/invitations").await;
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
        seed_member_care(&app).await?;
        app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='administrator',permissions_version=permissions_version+1 WHERE id=74001").await?;
        let cookie=super::super::platform::sign_in(&app,71001,"persistence@example.test").await;
        let (cookie,html)=super::super::platform::form_page(&app,&cookie,"/households/persistence-fixture/admin/invitations").await;
        let csrf=super::super::platform::hidden_field(&html,"authenticity_token");
        let response=super::super::platform::post_form(&app,&cookie,"/households/persistence-fixture/admin/invitations",&[
            ("email","synthetic-membership-only@example.test"),("membership_role","member"),
            ("relationship_type","parent"),("access_level","manage"),("dependent_ids[]","73002"),
            ("authenticity_token",&csrf)]).await;
        let status=response.status().as_u16();
        let body=response.text().await.map_err(|_|DbErr::Custom("Expected rejected invitation form".to_owned()))?;
        let state=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(DISTINCT invitation.id) AS invitations,count(grant_record.id) AS grants,(SELECT count(*) FROM pg_loco_queue WHERE task_data->>'to'='synthetic-membership-only@example.test') AS messages,(SELECT count(*) FROM versions WHERE item_type='HouseholdInvitation') AS versions FROM public.household_invitations invitation LEFT JOIN public.household_invitation_grants grant_record ON grant_record.household_invitation_id=invitation.id WHERE invitation.household_id=72001 AND invitation.email='synthetic-membership-only@example.test'"))
            .await?.ok_or_else(||DbErr::Custom("Expected actual membership invitation state".to_owned()))?;
        require(status==422 && state.try_get::<i64>("","invitations")?==0
            && state.try_get::<i64>("","grants")?==0 && state.try_get::<i64>("","messages")?==0
            && state.try_get::<i64>("","versions")?==0,
            "Stale care-authority fields must return422 without invitation, grant, mail or version effects")?;
        require(body.contains("Household invitations cannot assign care authority.")
            && body.contains("synthetic-membership-only@example.test") && body.contains("href=\"#invitation-form\"")
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
        seed_member_care(&app).await?;
        app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='administrator',permissions_version=permissions_version+1 WHERE id=74001").await?;
        let token=app.token().await;
        for (name,value) in [("dependent_ids",json!([73002])),("relationship_type",json!("parent")),("access_level",json!("manage"))] {
            let mut attributes=json!({"email":"synthetic-clinical-field@example.test","membership_role":"member"});
            attributes[name]=value;
            let response=app.client.post(format!("{}/api/v1/households/72001/admin/invitations",app.origin))
                .bearer_auth(&token).json(&json!({"household_invitation":attributes})).send().await
                .map_err(|_|DbErr::Custom("Invitation authority negative transport failed".to_owned()))?;
            require(response.status().as_u16()==422,"Unsupported clinical invitation fields must be rejected explicitly")?;
        }
        let state=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM household_invitations WHERE household_id=72001 AND email='synthetic-clinical-field@example.test') AS invitations,(SELECT count(*) FROM household_invitation_grants WHERE household_id=72001) AS grants,(SELECT count(*) FROM pg_loco_queue WHERE task_data->>'to'='synthetic-clinical-field@example.test') AS messages,(SELECT count(*) FROM versions WHERE item_type='HouseholdInvitation') AS versions"))
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
        .get(format!("{}/api/v1/households/72001/{path}", app.origin))
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
        "{}/api/v1/households/72001/person_medications/{id}",
        app.origin
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
        seed_member_care(&app).await?;
        let cookie = super::super::platform::sign_in(&app,71001,"persistence@example.test").await;
        let (_, html) = super::super::platform::form_page(&app,&cookie,
            "/households/persistence-fixture/medications/80001").await;
        require(html.contains("dose-confirm-person_medication-81504")
            && html.contains("dose-confirm-schedule-83504"),
            "Medication detail must offer Record for both another participant's source kinds without grants")?;
        let token = app.token().await;
        require(write_member_plan(&app,&token,81504).await? == 403,
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
        seed_member_care(&app).await?;
        let cookie = super::super::platform::sign_in(&app,71001,"persistence@example.test").await;
        let (_, html) = super::super::platform::form_page(&app,&cookie,
            "/households/persistence-fixture/reports").await;
        require(html.contains("value=\"73003\"") && html.contains("value=\"73001\"")
            && !html.contains("value=\"73103\"") && !html.contains("value=\"73104\""),
            "Report selectors must include local participants and canonical self, never foreign People")?;
        let tenant = access::begin(&app.fixture.runtime,&HouseholdScope {
            actor:Actor {account_id:71001},household_id:72001,
            request_id:"synthetic-member-report-selectors".to_owned()},None).await
            .map_err(|_|DbErr::Custom("Expected member report scope".to_owned()))?;
        let manageable = med_tracker::models::care::reports::accessible_people(&tenant,access::PersonAccess::Manage).await;
        tenant.rollback().await.map_err(|_|DbErr::Custom("Unable to close report scope".to_owned()))?;
        let manageable = manageable.map_err(|_|DbErr::Custom("Expected report Manage selector".to_owned()))?;
        require(manageable.len()==1 && manageable.iter().all(|person|person.id==73001),
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
        seed_member_care(&app).await?;
        let token = app.token().await;
        let capabilities = read_member(&app,&token,"ui_capabilities").await?;
        require(capabilities["data"]["people"]["manage_ids"]==json!([73001])
            && capabilities["data"]["medications"]["create"]==true
            && capabilities["data"]["people"]["create"]==false,
            "Aggregate Manage authority must include active canonical self exactly once, never another participant")?;
        let cookie = super::super::platform::sign_in(&app,71001,"persistence@example.test").await;
        let (_, chooser) = super::super::platform::form_page(&app,&cookie,"/households/persistence-fixture/medications/workflow").await;
        require(chooser.contains("/people/73001/treatments/assignments/new?source=workflow")
            && !chooser.contains("/people/73003/treatments/assignments/new?source=workflow"),
            "Assignment chooser must offer canonical self and no unauthorised participant")?;
        let response = app.client.post(format!("{}/api/v1/households/72001/medications",app.origin))
            .bearer_auth(&token).json(&json!({"medication":{"name":"Synthetic self-owned stock",
                "location_id":79001,"dose_amount":"2","dose_unit":"tablet","current_supply":"10","reorder_threshold":"2"}}))
            .send().await.map_err(|_| DbErr::Custom("Self stock creation transport failed".to_owned()))?;
        require(response.status().as_u16()==201, "Active self plan authority must preserve maintained medication creation")?;
        let foreign = app.client.post(format!("{}/api/v1/households/72002/medications",app.origin))
            .bearer_auth(&token).json(&json!({"medication":{"name":"Synthetic forbidden foreign stock"}}))
            .send().await.map_err(|_| DbErr::Custom("Foreign stock denial transport failed".to_owned()))?;
        require(foreign.status().as_u16()==403, "Self authority must not authorise another household")?;
        let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.medications WHERE name='Synthetic self-owned stock' AND household_id=72001) AS local,
                    (SELECT count(*) FROM public.medications WHERE name='Synthetic forbidden foreign stock') AS foreign"))
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
        seed_member_care(&app).await?;
        app.fixture.admin.execute_unprepared("INSERT INTO public.carer_relationships(id,household_id,carer_id,patient_id,relationship_type,active,created_at,updated_at) VALUES(86505,72001,73001,73003,'parent',true,now(),now())").await?;
        let cookie=super::super::platform::sign_in(&app,71001,"persistence@example.test").await;
        let (cookies,edit)=super::super::platform::form_page(&app,&cookie,"/households/persistence-fixture/people/73001/edit").await;
        let csrf=super::super::platform::hidden_field(&edit,"authenticity_token");
        require(!csrf.is_empty(),"Ordinary-member negatives must use real authenticated CSRF")?;
        let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT updated_at FROM public.carer_relationships WHERE id=86505")).await?.ok_or_else(||DbErr::Custom("Expected existing care relationship".to_owned()))?;
        let version=row.try_get::<chrono::NaiveDateTime>("","updated_at")?.to_string();
        let effects="SELECT (SELECT count(*) FROM public.carer_relationships WHERE active) AS active,(SELECT count(*) FROM public.person_access_grants) AS grants,(SELECT count(*) FROM public.household_invitations) AS invitations,(SELECT count(*) FROM public.household_invitation_grants) AS invitation_grants,(SELECT count(*) FROM public.versions) AS versions,(SELECT count(*) FROM public.pg_loco_queue) AS jobs";
        let fields=["active","grants","invitations","invitation_grants","versions","jobs"];
        let before=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,effects)).await?.ok_or_else(||DbErr::Custom("Expected original care-route effects".to_owned()))?;
        let before=fields.iter().map(|field|before.try_get::<i64>("",field)).collect::<Result<Vec<_>,_>>()?;
        require(before.last()==Some(&0),"Denied parent invitation must begin without queued work")?;
        let path="/households/persistence-fixture/people/73003/carer_relationships";
        let removal=super::super::platform::post_form(&app,&cookies,&format!("{path}/86505/remove"),&[("authenticity_token",&csrf),("version",&version)]).await;
        require(removal.status().as_u16()==403,"Ordinary member must not remove another recorded carer through a forged valid-CSRF request")?;
        let invitation=super::super::platform::post_form(&app,&cookies,path,&[("authenticity_token",&csrf),("email","synthetic-forbidden-parent@example.test"),("relationship_type","parent")]).await;
        require(invitation.status().as_u16()==403,"Existing relationship or parent label must not permit a new person-scoped authority invitation")?;
        let after=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,effects)).await?.ok_or_else(||DbErr::Custom("Expected denied care-route effects".to_owned()))?;
        let after=fields.iter().map(|field|after.try_get::<i64>("",field)).collect::<Result<Vec<_>,_>>()?;
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
        seed_member_care(&app).await?;
        app.fixture.admin.execute_unprepared(
            "UPDATE public.household_memberships SET role='administrator',permissions_version=permissions_version+1 WHERE id=74001;
             INSERT INTO public.carer_relationships(id,household_id,carer_id,patient_id,relationship_type,active,created_at,updated_at)
             VALUES(86504,72001,73001,73003,'parent',true,now(),now())"
        ).await?;
        let cookie = super::super::platform::sign_in(&app,71001,"persistence@example.test").await;
        let path = "/households/persistence-fixture/people/73003/carer_relationships";
        let detail = app.client.get(format!("{}/households/persistence-fixture/people/73003",app.origin)).header("cookie",&cookie).send().await.map_err(|_|DbErr::Custom("Existing relationship detail transport failed".to_owned()))?;
        require(detail.status().as_u16()==200,"Current participant detail must remain readable")?;
        let detail=detail.text().await.map_err(|_|DbErr::Custom("Existing relationship detail body missing".to_owned()))?;
        require(detail.contains(&format!("href=\"{path}\"")),"Person detail must link to authorised existing care relationship reads/removal independently of blocked assignment")?;
        let response = app.client.get(format!("{}{path}",app.origin)).header("cookie",&cookie)
            .send().await.map_err(|_| DbErr::Custom("Recorded relationship read transport failed".to_owned()))?;
        let status = response.status().as_u16();
        let cookies = super::super::platform::response_cookies(&cookie,&response);
        let page = response.text().await.map_err(|_| DbErr::Custom("Recorded relationship page missing".to_owned()))?;
        require(status==200, &format!("Existing relationship read must remain available through live View authority: status={status}"))?;
        require(page.contains("Synthetic adult") && page.contains("/86504/remove")
            && !page.contains("name=\"carer_id\"") && !page.contains("name=\"email\""),
            "Existing relationship must be displayed/removable without advertising unsupported new authority")?;
        let csrf = super::super::platform::hidden_field(&page,"authenticity_token");
        let version = super::super::platform::hidden_field(&page,"version");
        require(!csrf.is_empty() && !version.is_empty(), "Removal must use real CSRF and record version")?;
        let assign = super::super::platform::post_form(&app,&cookies,path,
            &[("authenticity_token",&csrf),("carer_id","73001"),("relationship_type","parent")]).await;
        require(assign.status().as_u16()==403, "View-only relationship page must not enable parent-label authority creation")?;
        let removed = super::super::platform::post_form(&app,&cookies,&format!("{path}/86504/remove"),
            &[("authenticity_token",&csrf),("version",&version)]).await;
        require(removed.status().as_u16()==200, "Existing permitted relationship removal must succeed and refresh")?;
        let (_, refreshed) = super::super::platform::form_page(&app,&cookies,path).await;
        require(!refreshed.contains("/86504/restore"), "Read-only page must not advertise blocked authority restoration")?;
        let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT active,updated_at FROM public.carer_relationships WHERE id=86504"))
            .await?.ok_or_else(|| DbErr::Custom("Expected retained deactivated relationship".to_owned()))?;
        require(!row.try_get::<bool>("","active")?, "Removal must retain the inactive relationship record")?;
        let version = row.try_get::<chrono::NaiveDateTime>("","updated_at")?.to_string();
        let restore = super::super::platform::post_form(&app,&cookies,&format!("{path}/86504/restore"),
            &[("authenticity_token",&csrf),("version",&version)]).await;
        require(restore.status().as_u16()==403, "Relationship restoration must not recreate guardian/Manage authority")?;
        let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT active,(SELECT count(*) FROM public.person_access_grants WHERE person_id=73003 AND revoked_at IS NULL) AS grants
             FROM public.carer_relationships WHERE id=86504"))
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
        seed_member_care(&app).await?;
        let token = app.token().await;
        let own = write_member_plan(&app,&token,81001).await?;
        let other = write_member_plan(&app,&token,81504).await?;
        require(own==200 && other==403,
            &format!("Verified self plan write must succeed, another participant must remain forbidden: self={own}, other={other}"))?;
        let state = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT dose_amount::text FROM public.person_medications WHERE id=81001) AS own,
                    (SELECT dose_amount::text FROM public.person_medications WHERE id=81504) AS other,
                    (SELECT current_supply::text FROM public.medications WHERE id=80001) AS supply,
                    (SELECT count(*) FROM public.versions WHERE item_type='PersonMedication' AND event='update') AS versions"))
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
        seed_member_care(&app).await?;
        app.fixture.admin.execute_unprepared("UPDATE public.household_memberships SET role='administrator',permissions_version=permissions_version+1 WHERE id=74001").await?;
        let before = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT permissions_version FROM public.household_memberships WHERE id=74001"))
            .await?.ok_or_else(|| DbErr::Custom("Expected manager membership".to_owned()))?
            .try_get::<i32>("","permissions_version")?;
        let token = app.token().await;
        let response = app.client.post(format!("{}/api/v1/households/72001/admin/person_access_grants",app.origin))
            .bearer_auth(&token).header("idempotency-key","synthetic-manager-is-not-guardian")
            .json(&json!({"person_access_grant":{"household_membership_id":74001,"person_id":73003,"access_level":"manage","relationship_type":"parent"}}))
            .send().await.map_err(|_| DbErr::Custom("Manager grant request failed".to_owned()))?;
        let status = response.status().as_u16();
        let state = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT permissions_version,
                (SELECT count(*) FROM public.person_access_grants WHERE person_id=73003 AND revoked_at IS NULL) AS grants,
                (SELECT count(*) FROM public.carer_relationships WHERE patient_id=73003 AND active) AS relationships
             FROM public.household_memberships WHERE id=74001"))
            .await?.ok_or_else(|| DbErr::Custom("Expected unchanged manager authority".to_owned()))?;
        let grants = state.try_get::<i64>("","grants")?;
        require(status==403 && state.try_get::<i32>("","permissions_version")?==before
            && state.try_get::<i64>("","grants")?==0 && state.try_get::<i64>("","relationships")?==0,
            &format!("Manager parent-label assertion must return403 without authority changes: status={status}, new_manage_grants={grants}"))?;
        let audit = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(e.id) AS events,count(l.id) AS ledger,count(d.id) AS deliveries
             FROM public.security_audit_events e
             LEFT JOIN public.audit_ledger_entries l ON l.source_table='security_audit_events' AND l.source_id=e.id
             LEFT JOIN public.audit_export_deliveries d ON d.audit_ledger_entry_id=l.id
             WHERE e.household_id=72001 AND e.actor_account_id=71001 AND e.actor_membership_id=74001
               AND e.event_type='household_access.person_grant_changed' AND e.metadata->>'outcome'='rejected'"))
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
        seed_member_care(&app).await?;
        app.fixture.admin.execute_unprepared(
            "UPDATE public.household_memberships SET role='administrator',permissions_version=permissions_version+1 WHERE id=74001;
             INSERT INTO public.person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,revoked_at,created_at,updated_at)
             VALUES(78504,72001,74001,73003,'manage','parent',now(),now(),now());
             UPDATE public.household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74981;
             UPDATE public.people SET name='synthetic-future-departed-profile-marker' WHERE id=73003"
        ).await?;
        let token = app.token().await;
        let response = app.client.get(format!("{}/api/v1/households/72001/admin/person_access_grants",app.origin))
            .bearer_auth(&token).send().await
            .map_err(|_| DbErr::Custom("Retained grant read transport failed".to_owned()))?;
        let status = response.status().as_u16();
        let body: Value = response.json().await.map_err(|_| DbErr::Custom("Expected retained grant response".to_owned()))?;
        require(status==200, &format!("Retained local grant must remain readable after subject departure: status={status}"))?;
        let grant = body["data"].as_array().and_then(|rows| rows.iter().find(|row| row["id"]==78504))
            .ok_or_else(|| DbErr::Custom("Retained revoked grant must not be silently omitted".to_owned()))?;
        require(grant["person_id"]==73003 && grant["household_membership_id"]==74001
            && grant["access_level"]=="manage" && grant["revoked_at"].is_string()
            && grant["person_name"].is_null()
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
        seed_member_care(&app).await?;
        app.fixture.admin.execute_unprepared("UPDATE public.household_memberships SET role='administrator',permissions_version=permissions_version+1 WHERE id=74001").await?;
        let tenant = access::begin(&app.fixture.runtime,&HouseholdScope {
            actor:Actor { account_id:71001 },household_id:72001,request_id:"synthetic-manager-delegation".to_owned(),
        },None).await.map_err(|_| DbErr::Custom("Expected manager scope".to_owned()))?;
        let assigned = med_tracker::models::care::administration::delegation::assign(&tenant,73001,73003,"parent",None).await;
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
        seed_member_care(&app).await?;
        let token = app.token().await;
        let people = read_member(&app, &token, "people").await?;
        require(people["data"].as_array().is_some_and(|rows| rows.len()==3
            && rows.iter().any(|row| row["id"]==73003)
            && rows.iter().all(|row| row["id"]!=73103 && row["id"]!=73104)),
            "Member roster must include all local participants and exclude foreign linked/accountless People")?;
        for (path, id) in [("schedules",83504),("person_medications",81504),("medications",80001)] {
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
        seed_member_care(&app).await?;
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
        seed_member_care(&app).await?;
        let tenant = access::begin(
            &app.fixture.runtime,
            &HouseholdScope {
                actor: Actor { account_id: 71001 },
                household_id: 72001,
                request_id: "synthetic-member-dashboard".to_owned(),
            },
            None,
        )
        .await
        .map_err(|_| DbErr::Custom("Expected ordinary member scope".to_owned()))?;
        let manageable =
            access::require_person_access(&tenant, 73003, access::PersonAccess::Manage).await;
        let dashboard = med_tracker::models::care::dashboard::read(
            &tenant,
            chrono_tz::UTC,
            Some(73003),
            "persistence-fixture",
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
                    .any(|row| row["source_id"] == 81504 && row["can_record"] == true)
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
        seed_member_care(&app).await?;
        let token = app.token().await;
        let body = json!({"medication_take":{"client_uuid":"f96488cd-1c35-4565-9bfa-a38fe7039d83",
            "source_type":"person_medication","source_id":"81504","taken_at":"2026-10-05T10:00:00Z",
            "dose_amount":"2","dose_unit":"tablet","taken_from_medication_id":80001}});
        for expected in [201,200] {
            let response = app.client.post(format!("{}/api/v1/households/72001/medication_takes", app.origin))
                .bearer_auth(&token).json(&body).send().await
                .map_err(|_| DbErr::Custom("Member dose request failed".to_owned()))?;
            require(response.status().as_u16()==expected, "Member must record and replay another participant's dose without a Person grant")?;
        }
        let state = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT current_supply::text AS supply,
                (SELECT count(*) FROM public.medication_takes WHERE person_medication_id=81504) AS takes,
                (SELECT count(*) FROM public.versions WHERE item_type='MedicationTake' AND event='create') AS versions
             FROM public.medications WHERE id=80001"))
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
        seed_member_care(&app).await?;
        app.fixture.admin.execute_unprepared(
            "UPDATE public.household_memberships SET role='administrator',permissions_version=permissions_version+1 WHERE id=74001;
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at)
             VALUES(74505,72001,73103,'member','active',now(),now(),now())"
        ).await?;
        let token = app.token().await;
        let before = read_member(&app,&token,"admin/memberships").await?;
        let rows = before["data"].as_array()
            .ok_or_else(|| DbErr::Custom("Expected canonical membership collection".to_owned()))?;
        let accountless = rows.iter().find(|row| row["person_id"]==73003)
            .ok_or_else(|| DbErr::Custom("Accountless participation must be represented".to_owned()))?;
        require(accountless["account_id"].is_null() && accountless["email"].is_null()
            && accountless["user_id"].is_null() && accountless["person_name"].is_string(),
            "Accountless participant must retain clinical identity without fabricated Account/User fields")?;
        let linked = rows.iter().find(|row| row["id"]==74505)
            .ok_or_else(|| DbErr::Custom("Expected real account-linked participation".to_owned()))?;
        require(linked["account_id"]==71002 && linked["email"].is_string() && linked["user_id"]==77103,
            "Active linked participation must preserve actual Account and User IDs")?;
        let identity = identity_state(&app.fixture.admin).await?;
        for (person_id,expected) in [(Value::Null,422),(json!(73003),422),(json!(73103),200)] {
            let response = app.client.patch(format!("{}/api/v1/households/72001/admin/memberships/74505",app.origin))
                .bearer_auth(&token).json(&json!({"household_membership":{"person_id":person_id}}))
                .send().await.map_err(|_| DbErr::Custom("Membership identity contract transport failed".to_owned()))?;
            require(response.status().as_u16()==expected,
                "Membership must reject null/relink and preserve unchanged-ID echo compatibility")?;
        }
        require(read_member(&app,&token,"admin/memberships").await?==before
            && identity_state(&app.fixture.admin).await?==identity,
            "Null/relink denials and unchanged-ID echo must not mutate canonical identity or membership state")?;
        app.fixture.admin.execute_unprepared(
            "UPDATE public.household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74505;
             UPDATE public.accounts SET name='synthetic-future-account-marker',email='synthetic-future-account-marker@example.test' WHERE id=71002;
             UPDATE public.people SET name='synthetic-future-person-marker' WHERE id=73103"
        ).await?;
        let after = read_member(&app,&token,"admin/memberships").await?;
        let inactive = after["data"].as_array().and_then(|rows| rows.iter().find(|row| row["id"]==74505))
            .ok_or_else(|| DbErr::Custom("Inactive participation must not be omitted".to_owned()))?;
        require(inactive["person_id"]==73103 && inactive["account_id"].is_null()
            && inactive["email"].is_null() && inactive["user_id"]==77103
            && inactive["person_name"].is_null()
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
        seed_member_care(&app).await?;
        app.fixture.admin.execute_unprepared(
            "UPDATE public.household_memberships SET role='administrator',permissions_version=permissions_version+1 WHERE id=74001;
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at)
             VALUES(74505,72001,73103,'member','active',now(),now(),now());
             INSERT INTO public.carer_relationships(id,household_id,carer_id,patient_id,relationship_type,active,created_at,updated_at)
             VALUES(86505,72001,73103,73003,'parent',true,now(),now()),
                   (86506,72001,73103,73002,'family_member',false,now(),now());
             UPDATE public.household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id IN (74505,74981);
             UPDATE public.people SET name='synthetic-retained-future-profile-marker-'||id WHERE id IN (73103,73003)"
        ).await?;
        let cookie = super::super::platform::sign_in(&app,71001,"persistence@example.test").await;
        let path = "/households/persistence-fixture/admin/carer_relationships";
        let response = app.client.get(format!("{}{path}",app.origin)).header("cookie",&cookie)
            .send().await.map_err(|_| DbErr::Custom("Retained admin relationship read failed".to_owned()))?;
        let status = response.status().as_u16();
        let cookies = super::super::platform::response_cookies(&cookie,&response);
        let body = response.text().await.map_err(|_| DbErr::Custom("Expected retained admin relationship page".to_owned()))?;
        require(status==200 && body.contains("#73103") && body.contains("#73003")
            && body.contains("Person not currently in this household")
            && !body.contains("synthetic-retained-future-profile-marker"),
            "Retained relationships need stable Person IDs and truthful unavailable labels without future profile reads")?;
        require(!body.contains("/admin/carer_relationships/new") && !body.contains("/86506/activate"),
            "Read-only administration must not advertise unsupported relationship authority creation/restoration")?;
        let csrf = super::super::platform::hidden_field(&body,"authenticity_token");
        require(!csrf.is_empty(), "Retained relationship removal needs real CSRF")?;
        let removed = super::super::platform::post_form(&app,&cookies,&format!("{path}/86505/deactivate"),
            &[("authenticity_token",&csrf)]).await;
        require(removed.status().as_u16()==200, "Permitted manager deactivation must remain usable for retained relationships")?;
        let (_, refreshed) = super::super::platform::form_page(&app,&cookies,path).await;
        require(refreshed.contains("#73103") && refreshed.contains("#73003")
            && !refreshed.contains("/86505/activate") && !refreshed.contains("synthetic-retained-future-profile-marker"),
            "Deactivation refresh must preserve retained IDs without offering authority restoration")?;
        let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT active FROM public.carer_relationships WHERE id=86505"))
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

async fn identity_state(db: &impl ConnectionTrait) -> Result<IdentityState, DbErr> {
    let row = db.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM public.people) AS people,
                (SELECT count(*) FROM public.accounts) AS accounts,
                (SELECT count(*) FROM public.users) AS users,
                (SELECT count(*) FROM public.api_sessions) AS api_sessions,
                (SELECT count(*) FROM public.identity_api_keys) AS api_keys,
                (SELECT count(*) FROM public.oauth_grants) AS oauth_grants,
                (SELECT encode(digest(COALESCE(string_agg(id::text || ':' || COALESCE(password,''), ',' ORDER BY id),''),'sha256'),'hex') FROM public.auth_credentials) AS passwords,
                (SELECT encode(digest(COALESCE(jsonb_agg(to_jsonb(p) ORDER BY p.id)::text,'[]'),'sha256'),'hex') FROM public.identity_passkeys p) AS passkeys,
                (SELECT encode(digest(COALESCE(jsonb_agg(to_jsonb(p) ORDER BY p.id)::text,'[]'),'sha256'),'hex') FROM public.auth_credentials p) AS providers,
                (SELECT encode(digest(COALESCE(jsonb_agg(jsonb_build_array(a.id,to_jsonb(a)->'person_id') ORDER BY a.id)::text,'[]'),'sha256'),'hex') FROM public.accounts a) AS account_bindings"))
        .await?.ok_or_else(|| DbErr::Custom("Expected identity counts".to_owned()))?;
    Ok(IdentityState {
        people: row.try_get("", "people")?,
        accounts: row.try_get("", "accounts")?,
        users: row.try_get("", "users")?,
        api_sessions: row.try_get("", "api_sessions")?,
        api_keys: row.try_get("", "api_keys")?,
        oauth_grants: row.try_get("", "oauth_grants")?,
        passwords: row.try_get("", "passwords")?,
        passkeys: row.try_get("", "passkeys")?,
        providers: row.try_get("", "providers")?,
        account_bindings: row.try_get("", "account_bindings")?,
    })
}

async fn accept(app: &Application, access: &str) -> Result<(u16, Value), DbErr> {
    let response = app
        .client
        .post(format!("{}/api/v1/invitations/accept", app.origin))
        .bearer_auth(access)
        .json(&json!({"token":"synthetic-account-invitation"}))
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

async fn acceptance_effects(db: &impl ConnectionTrait) -> Result<Vec<i64>, DbErr> {
    let row = db.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM public.household_memberships WHERE household_id=72002) AS memberships,
                (SELECT count(*) FROM public.household_invitations WHERE id=87001 AND accepted_at IS NOT NULL) AS accepted,
                (SELECT count(*) FROM public.versions WHERE household_id=72002) AS versions,
                (SELECT count(*) FROM public.security_audit_events WHERE household_id=72002
                 AND event_type IN ('household_access.membership_created','household_access.person_grant_changed')) AS audit,
                (SELECT count(*) FROM public.api_change_events WHERE household_id=72002) AS changes,
                (SELECT count(*) FROM public.person_access_grants WHERE household_id=72002) AS grants,
                (SELECT count(*) FROM public.carer_relationships WHERE household_id=72002) AS carers,
                (SELECT COALESCE(sum(permissions_version),0)::bigint FROM public.household_memberships WHERE household_id=72002) AS permission_versions,
                (SELECT count(*) FROM public.pg_loco_queue) AS jobs"))
        .await?.ok_or_else(|| DbErr::Custom("Expected invitation effects".to_owned()))?;
    [
        "memberships",
        "accepted",
        "versions",
        "audit",
        "changes",
        "grants",
        "carers",
        "permission_versions",
        "jobs",
    ]
    .into_iter()
    .map(|field| row.try_get("", field))
    .collect()
}

async fn seed_membership_invitation(app: &Application) -> Result<&'static str, DbErr> {
    let access = seed_acceptance(app).await;
    app.fixture
        .admin
        .execute_unprepared("DELETE FROM public.household_invitation_grants WHERE id=87002")
        .await?;
    Ok(access)
}

async fn claim_target_state(db: &impl ConnectionTrait) -> Result<Value, DbErr> {
    let row = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT jsonb_build_object(
            'person',to_jsonb(p),
            'participations',(SELECT COALESCE(jsonb_agg(to_jsonb(m) ORDER BY m.id),'[]'::jsonb)
                FROM public.household_memberships m WHERE m.person_id=p.id),
            'relationships',(SELECT COALESCE(jsonb_agg(to_jsonb(c) ORDER BY c.id),'[]'::jsonb)
                FROM public.carer_relationships c WHERE c.patient_id=p.id OR c.carer_id=p.id),
            'grants',(SELECT COALESCE(jsonb_agg(to_jsonb(g) ORDER BY g.id),'[]'::jsonb)
                FROM public.person_access_grants g WHERE g.person_id=p.id)) AS state
         FROM public.people p WHERE p.id=73104",
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("Expected existing claim target".to_owned()))?;
    row.try_get("", "state")
}

async fn original_account_state(db: &impl ConnectionTrait) -> Result<String, DbErr> {
    let row = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT encode(digest(to_jsonb(a)::text,'sha256'),'hex') AS state
         FROM public.accounts a WHERE a.id=71001",
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("Expected original Account".to_owned()))?;
    row.try_get("", "state")
}

#[tokio::test]
async fn existing_account_invitation_reuses_its_person_across_households() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let access = seed_membership_invitation(&app).await?;
        let before = identity_state(&app.fixture.admin).await?;
        let original_person = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT to_jsonb(p) AS state FROM public.people p WHERE p.id=73001"))
            .await?.ok_or_else(|| DbErr::Custom("Expected original Person".to_owned()))?
            .try_get::<Value>("", "state")?;
        let (status, first) = accept(&app, access).await?;
        require(status == 200, "Valid existing-account invitation must be accepted")?;
        require(first["data"]["person_id"] == "73001",
            "Invitation acceptance must reuse original Person 73001, not clone it")?;
        require(first["data"]["role"] == "member", "Invitation must create an ordinary member")?;
        require(identity_state(&app.fixture.admin).await? == before,
            "Joining must preserve Person/Account identity, credential counts, passwords, passkeys and provider links")?;
        let state = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT to_jsonb(p) FROM public.people p WHERE p.id=73001) AS person,
                    (SELECT count(*) FROM public.versions WHERE item_type='Person' AND item_id=73001) AS versions,
                    (SELECT count(*) FROM public.api_change_events WHERE household_id=72002 AND record_type='Person' AND record_id=73001 AND action='create') AS creates,
                    (SELECT count(*) FROM public.api_change_events WHERE household_id=72002 AND record_type='Person' AND record_id=73001 AND action='update') AS updates"))
            .await?.ok_or_else(|| DbErr::Custom("Expected participation visibility effect".to_owned()))?;
        require(state.try_get::<Value>("", "person")? == original_person
            && state.try_get::<i64>("", "versions")? == 0,
            "Joining must not mutate or fabricate creation history for the existing Person")?;
        require(state.try_get::<i64>("", "creates")? == 0
            && state.try_get::<i64>("", "updates")? == 1,
            "Joining must emit one supported update visibility notification, not a Person create")?;
        let mobile = app.token().await;
        let visible = app.client.get(format!("{}/api/v1/households/72002/people/73001", app.origin))
            .bearer_auth(&mobile).send().await.map_err(|_| DbErr::Custom("Expected joined household Person request".to_owned()))?;
        let visible_status = visible.status().as_u16();
        let visible: Value = visible.json().await.map_err(|_| DbErr::Custom("Expected joined Person response".to_owned()))?;
        require(visible_status == 200 && visible["data"]["id"] == 73001,
            "The new household must resolve the original Person through the maintained HTTP contract")?;
        let membership = first["data"]["membership_id"].as_str()
            .and_then(|value| value.parse::<i64>().ok())
            .ok_or_else(|| DbErr::Custom("Expected accepted membership ID".to_owned()))?;
        let committed_effects = acceptance_effects(&app.fixture.admin).await?;
        let (replay_status, replay) = accept(&app, access).await?;
        require(replay_status == 200 && first == replay,
            "Repeated acceptance must return the same membership and Person")?;
        require(identity_state(&app.fixture.admin).await? == before,
            "Invitation replay must not duplicate identity")?;
        require(acceptance_effects(&app.fixture.admin).await? == committed_effects,
            "Invitation replay must not duplicate membership, versions, audit, changes or jobs")?;
        let scope = HouseholdScope {
            actor: Actor { account_id: 71001 },
            household_id: 72002,
            request_id: "synthetic-shared-person-join".to_owned(),
        };
        let tenant = access::begin(&app.fixture.runtime, &scope, None).await
            .map_err(|_| DbErr::Custom("Accepted membership must provide B scope".to_owned()))?;
        require(tenant.membership().role == "member", "Accepted reader must not rely on manager authority")?;
        let visible = tenant.transaction().query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM public.people WHERE id=73001) AS visible,
                    EXISTS(SELECT 1 FROM public.people WHERE id=73002) AS foreign"))
            .await?.ok_or_else(|| DbErr::Custom("Expected subject visibility".to_owned()))?;
        let foreign: bool = visible.try_get("", "foreign")?;
        let visible: bool = visible.try_get("", "visible")?;
        require(visible, "B must see the original global Person")?;
        require(!foreign, "Joining B must not reveal A's other accountless subject")?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE public.household_memberships SET status='revoked', revoked_at=now(),
             permissions_version=permissions_version+1 WHERE id=$1", [membership.into()])).await?;
        let recheck = access::recheck(&tenant).await;
        let retained_foreign = tenant.transaction().query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM public.people WHERE id=73103) AS visible"))
            .await?.ok_or_else(|| DbErr::Custom("Expected post-revocation visibility".to_owned()))?;
        let retained_foreign: bool = retained_foreign.try_get("", "visible")?;
        tenant.rollback().await
            .map_err(|_| DbErr::Custom("Unable to close accepted scope".to_owned()))?;
        require(matches!(recheck, Err(med_tracker::models::errors::OperationError::Forbidden)),
            "An already-open B scope must reject actor membership revocation")?;
        require(!retained_foreign, "Revoked B scope must not retain visibility of another B Person")?;
        let (revoked_status, _) = accept(&app, access).await?;
        require(revoked_status == 422, "Acceptance replay must not restore revoked membership")?;
        require(identity_state(&app.fixture.admin).await? == before,
            "Revoked replay must not create replacement identity")?;
        require(matches!(access::begin(&app.fixture.runtime, &scope, None).await,
            Err(med_tracker::models::errors::OperationError::Forbidden)),
            "Revoked B membership must not authorise a fresh scope")?;
        let origin = access::begin(&app.fixture.runtime, &HouseholdScope {
            household_id: 72001, ..scope
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
        seed_membership_invitation(&app).await?;
        let tenant = access::begin(&app.fixture.runtime, &HouseholdScope {
            actor: Actor { account_id: 71001 }, household_id: 72001,
            request_id: "synthetic-clinical-participation-integrity".to_owned(),
        }, None).await.map_err(|_| DbErr::Custom("Expected restricted household scope".to_owned()))?;
        let attempted = tenant.transaction().begin().await?;
        let foreign = attempted.execute_unprepared(
            "INSERT INTO public.person_medications(household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at)
             VALUES(72001,73103,80001,1,'tablet',0,now(),now())"
        ).await;
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
        tenant.transaction().execute_unprepared(
            "INSERT INTO public.person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at)
             VALUES(81503,72001,73003,80001,1,'tablet',0,now(),now())"
        ).await?;
        tenant.commit().await.map_err(|_| DbErr::Custom("Expected valid participation reference commit".to_owned()))?;
        app.fixture.admin.execute_unprepared(
            "UPDATE public.household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1
             WHERE household_id=72001 AND person_id=73003"
        ).await?;
        let tenant = access::begin(&app.fixture.runtime, &HouseholdScope {
            actor: Actor { account_id: 71001 }, household_id: 72001,
            request_id: "synthetic-departed-clinical-reference".to_owned(),
        }, None).await.map_err(|_| DbErr::Custom("Expected retained origin household scope".to_owned()))?;
        let row = tenant.transaction().query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT person_id,household_id FROM public.person_medications WHERE id=81503"))
            .await?.ok_or_else(|| DbErr::Custom("Departure must retain existing clinical origin reference".to_owned()))?;
        require(row.try_get::<i64>("", "person_id")? == 73003 && row.try_get::<i64>("", "household_id")? == 72001,
            "Departure must not move or delete the recorded clinical reference")?;
        let attempted = tenant.transaction().begin().await?;
        let removal = attempted.execute_unprepared(
            "DELETE FROM public.household_memberships WHERE household_id=72001 AND person_id=73003"
        ).await;
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
        let access = seed_membership_invitation(&app).await?;
        let identity = identity_state(&app.fixture.admin).await?;
        let mutations = [
            "UPDATE public.household_invitations SET expires_at=now()-interval '1 second' WHERE id=87001",
            "UPDATE public.household_invitations SET expires_at=now()+interval '1 day',revoked_at=now() WHERE id=87001",
            "UPDATE public.household_invitations SET revoked_at=NULL WHERE id=87001; UPDATE public.household_memberships SET role='member',permissions_version=permissions_version+1 WHERE id=74002",
            "UPDATE public.household_memberships SET role='owner',status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74002",
        ];
        for mutation in mutations {
            app.fixture.admin.execute_unprepared(mutation).await?;
            let before = acceptance_effects(&app.fixture.admin).await?;
            let (status, body) = accept(&app, access).await?;
            require(status == 422 && body["error"]["code"] == "invitation_unavailable",
                "Expired/revoked invitations and revoked/demoted inviters must be rejected")?;
            require(acceptance_effects(&app.fixture.admin).await? == before,
                "Rejected invitation must not change membership, acceptance, audit or operational effects")?;
            require(identity_state(&app.fixture.admin).await? == identity,
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
        let access = seed_membership_invitation(&app).await?;
        let identity = identity_state(&app.fixture.admin).await?;
        let effects = acceptance_effects(&app.fixture.admin).await?;
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
        let (status, body) = accept(&app, access).await?;
        let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT is_called AS reached FROM public.synthetic_global_join_audit_reached"))
            .await?.ok_or_else(|| DbErr::Custom("Expected join audit marker".to_owned()))?;
        require(row.try_get("", "reached")?, "Forced acceptance audit failure must be reached")?;
        require(identity_state(&app.fixture.admin).await? == identity,
            "Failed acceptance must roll back Person/Account/credential changes")?;
        require(acceptance_effects(&app.fixture.admin).await? == effects,
            "Failed acceptance must roll back membership, accepted state, versions, audits, changes and jobs")?;
        require(status == 500 && body["error"]["code"] == "internal_error",
            "Audit failure must return the existing generic server failure")?;
        app.fixture.admin.execute_unprepared(
            "DROP TRIGGER synthetic_global_join_audit ON public.security_audit_events"
        ).await?;
        let (retry_status, retry) = accept(&app, access).await?;
        require(retry_status == 200 && retry["data"]["person_id"] == "73001",
            "Retry must join using the original Person")?;
        let committed = acceptance_effects(&app.fixture.admin).await?;
        let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS count FROM public.security_audit_events
             WHERE household_id=72002 AND event_type='household_access.membership_created'"))
            .await?.ok_or_else(|| DbErr::Custom("Expected membership acceptance audit".to_owned()))?;
        require(row.try_get::<i64>("", "count")? == 1,
            "Committed retry must produce exactly one membership audit")?;
        let (replay_status, replay) = accept(&app, access).await?;
        require(replay_status == 200 && replay == retry,
            "Committed retry must remain replayable")?;
        require(acceptance_effects(&app.fixture.admin).await? == committed
            && identity_state(&app.fixture.admin).await? == identity,
            "Replay after retry must not duplicate identity or effects")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn invitation_fields_cannot_claim_an_existing_accountless_person() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let access = seed_membership_invitation(&app).await?;
        app.fixture.admin.execute_unprepared(
            "UPDATE public.people SET name=(SELECT name FROM public.people WHERE id=73001),
             email=(SELECT email FROM public.accounts WHERE id=71001) WHERE id=73104"
        ).await?;
        let identity = identity_state(&app.fixture.admin).await?;
        let original_target = claim_target_state(&app.fixture.admin).await?;
        let response = app.client.post(format!("{}/api/v1/invitations/accept", app.origin))
            .bearer_auth(access)
            .json(&json!({"token":"synthetic-account-invitation","person_id":"73104",
                "account_id":"71002","dependent_ids":[73104],"relationship_type":"parent"}))
            .send().await.map_err(|_| DbErr::Custom("Claim injection transport failed".to_owned()))?;
        let status = response.status().as_u16();
        let body: Value = response.json().await
            .map_err(|_| DbErr::Custom("Claim injection response was not JSON".to_owned()))?;
        require(status == 200 && body["data"]["person_id"] == "73001",
            "Valid invitation may join only the authenticated Account's existing Person")?;
        require(claim_target_state(&app.fixture.admin).await? == original_target,
            "Matching identifiers or injected fields must preserve existing Person, participation, relationships and grants exactly")?;
        require(identity_state(&app.fixture.admin).await? == identity,
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
        seed_membership_invitation(&app).await?;
        app.fixture.admin.execute_unprepared(
            "UPDATE public.household_invitations SET email='synthetic-claim-signup@example.test' WHERE id=87001;
             UPDATE public.people SET name='Synthetic matching signup',email='synthetic-existing-contact@example.test' WHERE id=73104"
        ).await?;
        let target = claim_target_state(&app.fixture.admin).await?;
        let original_account = original_account_state(&app.fixture.admin).await?;
        let before = identity_state(&app.fixture.admin).await?;
        let response = app.client.post(format!("{}/api/auth/onboarding/signup", app.origin))
            .header("origin", app.context.config.server.full_url())
            .json(&json!({"name":"Synthetic matching signup","email":"synthetic-claim-signup@example.test",
                "date_of_birth":"1990-04-12","credential":"passkey",
                "invitation_token":"synthetic-account-invitation","person_id":73104,"account_id":71001}))
            .send().await.map_err(|_| DbErr::Custom("Signup claim transport failed".to_owned()))?;
        let status = response.status().as_u16();
        let body: Value = response.json().await
            .map_err(|_| DbErr::Custom(format!("Signup claim HTTP {status} response was not JSON")))?;
        require(status == 200 && body["status"] == true,
            &format!("Valid signup must retain its existing ignored-field contract (HTTP {status})"))?;
        require(claim_target_state(&app.fixture.admin).await? == target,
            "Signup must not adopt or alter the matching existing accountless Person or authority")?;
        require(original_account_state(&app.fixture.admin).await? == original_account,
            "Injected Account ID must not change the original Account")?;
        let after = identity_state(&app.fixture.admin).await?;
        require(after.accounts == before.accounts + 1 && after.people == before.people + 1
            && after.users == before.users + 1,
            "Legitimate signup must create exactly one distinct Account, Person and real User")?;
        require(after.api_sessions == before.api_sessions && after.api_keys == before.api_keys
            && after.oauth_grants == before.oauth_grants && after.passkeys == before.passkeys
            && after.providers == before.providers,
            "Passkey enrolment signup must not mint unrelated clinical credentials or provider links")?;
        let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT p.id<>73104 AS distinct_person FROM public.accounts a
             JOIN public.people p ON p.id=a.person_id
             WHERE a.email='synthetic-claim-signup@example.test' AND a.status=1"))
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
            seed_membership_invitation(&app).await?;
            app.fixture
                .admin
                .execute_unprepared(
                    "UPDATE public.people SET date_of_birth='1985-02-03' WHERE id=73001",
                )
                .await?;
            let token = app.token().await;
            let endpoint = format!("{}/api/v1/households/72001/profile", app.origin);
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
                profile["data"]["person_id"] == "73001" && profile["data"]["account_id"] == "71001",
                "Profile prerequisite must resolve the actual caller identity",
            )?;
            let target = claim_target_state(&app.fixture.admin).await?;
            let identity = identity_state(&app.fixture.admin).await?;
            let original_account = original_account_state(&app.fixture.admin).await?;
            let response = app.client.patch(&endpoint).bearer_auth(&token)
            .json(&json!({"profile":{"person_id":"73104","account_id":"71002","time_zone":"UTC"}}))
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
                claim_target_state(&app.fixture.admin).await? == target
                    && original_account_state(&app.fixture.admin).await? == original_account
                    && identity_state(&app.fixture.admin).await? == identity,
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
    let result = async {
        seed_membership_invitation(&app).await?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE public.people SET name='Synthetic signup control',email=$1 WHERE id=73104",
            [target_email.into()])).await?;
        let effects = acceptance_effects(&app.fixture.admin).await?;
        app.fixture.admin.execute_unprepared(
            "UPDATE public.household_invitations SET email='synthetic-signup-control@example.test' WHERE id=87001"
        ).await?;
        let target = claim_target_state(&app.fixture.admin).await?;
        let original_account = original_account_state(&app.fixture.admin).await?;
        let before = identity_state(&app.fixture.admin).await?;
        let response = app.client.post(format!("{}/api/auth/onboarding/signup", app.origin))
            .header("origin", app.context.config.server.full_url())
            .json(&json!({"name":"Synthetic signup control","email":"synthetic-signup-control@example.test",
                "date_of_birth":"1990-04-12","credential":"passkey",
                "invitation_token":"synthetic-account-invitation"}))
            .send().await.map_err(|_| DbErr::Custom("Signup control transport failed".to_owned()))?;
        let status = response.status().as_u16();
        require(claim_target_state(&app.fixture.admin).await? == target
            && original_account_state(&app.fixture.admin).await? == original_account,
            "Signup control must never attach or alter the existing target or Account")?;
        let after = identity_state(&app.fixture.admin).await?;
        if status == 200 {
            require(after.accounts == before.accounts + 1 && after.people == before.people + 1
                && after.users == before.users + 1,
                "Valid distinct-email signup must create one new Account/Person/real User")?;
            let body: Value = response.json().await
                .map_err(|_| DbErr::Custom("Signup control success was not JSON".to_owned()))?;
            require(body["status"] == true, "Signup control must return normal success")?;
            let provenance = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
                "SELECT count(v.id) AS versions,
                        COALESCE(bool_and(v.audit_context->>'actor_account_id'=a.id::text
                            AND v.audit_context->>'actor_user_id'=u.id::text),false) AS attributed,
                        (SELECT count(*) FROM public.api_change_events e WHERE e.household_id=72002
                         AND e.record_type='Person' AND e.record_id=a.person_id AND e.action='create') AS changes
                 FROM public.accounts a JOIN public.users u ON u.person_id=a.person_id
                 LEFT JOIN public.versions v ON v.item_type='Person' AND v.item_id=a.person_id AND v.event='create'
                 WHERE a.email='synthetic-signup-control@example.test' GROUP BY a.id,u.id"))
                .await?.ok_or_else(|| DbErr::Custom("Expected genuine signup provenance".to_owned()))?;
            require(provenance.try_get::<i64>("", "versions")? == 1
                && provenance.try_get::<bool>("", "attributed")?
                && provenance.try_get::<i64>("", "changes")? == 1,
                "Genuine invited signup must preserve one actual User-attributed Person creation version and create notification")?;
        } else {
            require(after == before, "Rejected signup must roll back all identity and credential state")?;
            require(acceptance_effects(&app.fixture.admin).await? == effects,
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
        seed_membership_invitation(&app).await?;
        app.fixture.admin.execute_unprepared(
            "UPDATE public.household_invitations SET email='synthetic-audit-signup@example.test' WHERE id=87001;
             CREATE SEQUENCE public.synthetic_signup_person_audit_reached;
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
        let before = identity_state(&app.fixture.admin).await?;
        let effects = acceptance_effects(&app.fixture.admin).await?;
        let response = app.client.post(format!("{}/api/auth/onboarding/signup", app.origin))
            .header("origin", app.context.config.server.full_url())
            .json(&json!({"name":"Synthetic audited signup","email":"synthetic-audit-signup@example.test",
                "date_of_birth":"1990-04-12","credential":"passkey",
                "invitation_token":"synthetic-account-invitation"}))
            .send().await.map_err(|_| DbErr::Custom("Signup audit rollback transport failed".to_owned()))?;
        let status = response.status().as_u16();
        let body = response.text().await.map_err(|_| DbErr::Custom("Signup audit rollback response failed".to_owned()))?;
        let reached = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT is_called AS reached FROM public.synthetic_signup_person_audit_reached"))
            .await?.ok_or_else(|| DbErr::Custom("Expected signup Person audit marker".to_owned()))?;
        require(reached.try_get::<bool>("", "reached")?, "Signup must attempt its genuine Person creation audit")?;
        require(identity_state(&app.fixture.admin).await? == before
            && acceptance_effects(&app.fixture.admin).await? == effects,
            "Person audit failure must atomically roll back Account, Person, actual User, participation, invitation, audit, changes and queued work")?;
        require(status == 500 && !body.contains("synthetic person audit unavailable"),
            "Signup audit failure must return a safe server error")?;
        Ok(())
    }.await;
    app.close().await;
    result
}
