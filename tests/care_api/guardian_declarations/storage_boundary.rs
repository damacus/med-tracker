// SeaQuery cannot express PostgreSQL trigger/function DDL, SET LOCAL ROLE or the
// catalogue ACL/regclass inspection. Those narrow storage probes retain SQL; generated
// row mutations use builders and fault DDL is removed before the App closes.

#[path = "recovery_link_guard.rs"]
mod recovery_link_guard;

use super::super::platform::{operation_confirm, operation_start, sign_in};
use super::{Application, effects, require, withdrawal::declaration};
use med_tracker::models::_entities::{
    accounts, household_memberships, people, person_access_grants, versions,
};
use med_tracker::models::access::{self, Actor, HouseholdScope, PersonAccess};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set,
    sea_query::{Alias, Expr, ExprTrait, Func, Query},
};
use sea_orm::{
    ConnectionTrait, DatabaseTransaction, DbBackend, DbErr, Statement, TransactionTrait,
};
use serde_json::{Value, json};

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

fn scope(account_id: i64, household_id: i64) -> HouseholdScope {
    HouseholdScope {
        actor: Actor { account_id },
        household_id,
        request_id: "synthetic-guardian-storage-boundary".into(),
    }
}

async fn denied_sql(
    transaction: &DatabaseTransaction,
    statement: Statement,
    code: &str,
) -> Result<(), DbErr> {
    let savepoint = transaction.begin().await?;
    let result = savepoint.execute_raw(statement).await;
    savepoint.rollback().await?;
    require(
        result
            .as_ref()
            .err()
            .is_some_and(|error| database_code(error, code)),
        &format!("The attempted storage operation must be rejected with SQLSTATE {code}"),
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
        .map_err(|_| DbErr::Custom("Storage-boundary withdrawal failed before response".into()))
}

#[tokio::test]
async fn guardian_storage_catalogue_and_restricted_mutations_preserve_provenance()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let token = app.token().await;
        let (subject, id) = declaration(&app, &token).await?;
        let facts = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT c.relrowsecurity AND c.relforcerowsecurity AS forced,
              r.rolname='med_tracker_owner' AND NOT r.rolcanlogin AND NOT r.rolbypassrls AS owner,
              has_table_privilege('med_tracker_app',c.oid,'SELECT') AND has_table_privilege('med_tracker_app',c.oid,'INSERT') AS readable_insertable,
              NOT has_table_privilege('med_tracker_app',c.oid,'UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER') AS no_broad_writes,
              NOT EXISTS(SELECT 1 FROM aclexplode(c.relacl) WHERE grantee=0) AS no_public_table,
              (SELECT count(*) FROM pg_attribute a WHERE a.attrelid=c.oid AND a.attnum>0 AND NOT a.attisdropped
                AND has_column_privilege('med_tracker_app',c.oid,a.attnum,'UPDATE'))=2
                AND has_column_privilege('med_tracker_app',c.oid,'withdrawn_at','UPDATE')
                AND has_column_privilege('med_tracker_app',c.oid,'revision','UPDATE') AS exact_update_columns,
              (SELECT count(*) FROM pg_constraint f WHERE f.conrelid=c.oid AND f.contype='f' AND f.confdeltype='r')=7 AS retained_foreign_keys,
              (SELECT count(*) FROM pg_policy WHERE polrelid=c.oid)=7 AS policies,
              EXISTS(SELECT 1 FROM pg_trigger t JOIN pg_proc p ON p.oid=t.tgfoid JOIN pg_roles owner ON owner.oid=p.proowner
                WHERE t.tgrelid=c.oid AND NOT t.tgisinternal AND t.tgname='guardian_declarations_one_way'
                AND p.proname='guardian_declaration_one_way' AND owner.rolname='med_tracker_owner'
                AND NOT p.prosecdef AND p.proconfig=ARRAY['search_path=pg_catalog']::text[]
                AND NOT has_function_privilege('med_tracker_app',p.oid,'EXECUTE')
                AND NOT EXISTS(SELECT 1 FROM aclexplode(p.proacl) WHERE grantee=0 AND privilege_type='EXECUTE')) AS fixed_trigger
             FROM pg_class c JOIN pg_roles r ON r.oid=c.relowner WHERE c.oid='public.guardian_declarations'::regclass"))
            .await?.ok_or_else(||DbErr::Custom("Guardian declaration catalogue facts missing".into()))?;
        for field in ["forced","owner","readable_insertable","no_broad_writes","no_public_table","exact_update_columns","retained_foreign_keys","policies","fixed_trigger"] {
            require(facts.try_get::<bool>("",field)?, &format!("Guardian catalogue invariant failed: {field}"))?;
        }
        let transaction = app.context.db.begin().await?;
        transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app").await?;
        set_account(&transaction,app.fixture.account.id).await?;
        let row=transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT current_user='med_tracker_app' AS restricted,count(*) AS declarations FROM public.guardian_declarations WHERE id=$1",[id.into()]))
            .await?.ok_or_else(||DbErr::Custom("Restricted declaration read missing".into()))?;
        require(row.try_get::<bool>("","restricted")? && row.try_get::<i64>("","declarations")?==1,
            "Own declaration must actually be readable through the restricted runtime role")?;
        let query=Query::update().table("guardian_declarations").value("guardian_person_id",app.fixture.minor.id).and_where(Expr::col("id").eq(id)).to_owned();
        denied_sql(&transaction,transaction.get_database_backend().build(&query),"42501").await?;
        let query=Query::delete().from_table("guardian_declarations").and_where(Expr::col("id").eq(id)).to_owned();
        denied_sql(&transaction,transaction.get_database_backend().build(&query),"42501").await?;
        let query=Query::select().expr(Func::cust("med_tracker.guardian_declaration_one_way")).to_owned();
        denied_sql(&transaction,transaction.get_database_backend().build(&query),"42501").await?;
        transaction.rollback().await?;
        require(withdraw(&app,&token,id).await?.status().as_u16()==200,"Terminal-transition prerequisite must use genuine withdrawal")?;
        let transaction=app.context.db.begin().await?;
        transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app").await?;
        set_account(&transaction,app.fixture.account.id).await?;
        let query=Query::update().table("guardian_declarations")
            .value("withdrawn_at",Option::<chrono::DateTime<chrono::FixedOffset>>::None).value("revision",1)
            .and_where(Expr::col("id").eq(id)).to_owned();
        denied_sql(&transaction,transaction.get_database_backend().build(&query),"23514").await?;
        let query=Query::insert().into_table("guardian_declarations")
            .columns(["subject_person_id","guardian_person_id","origin_household_id","declared_by_account_id","subject_name_at_declaration"])
            .values_panic([Expr::val(subject),Expr::val(app.fixture.adult.id),Expr::val(app.fixture.household.id),Expr::val(app.fixture.account.id),Expr::val("Forbidden second initial declaration")]).to_owned();
        denied_sql(&transaction,transaction.get_database_backend().build(&query),"23505").await?;
        let query=Query::update().table("guardian_declarations").value("source","manager_appointment")
            .and_where(Expr::col("id").eq(id)).to_owned();
        denied_sql(&transaction,transaction.get_database_backend().build(&query),"42501").await?;
        transaction.rollback().await?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_foreign_household_participation_reveals_subject_but_not_declaration_authority()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let (subject,id)=declaration(&app,&token).await?;
let foreign_scenario=crate::scenario::Scenario::new().await;
household_memberships::Entity::update_many().col_expr(household_memberships::Column::Role,Expr::val("member"))
    .filter(household_memberships::Column::Id.eq(foreign_scenario.membership.id)).exec(&app.fixture.admin).await?;
let now=chrono::Utc::now().naive_utc();
let participation=household_memberships::ActiveModel {household_id:Set(foreign_scenario.household.id),
    person_id:Set(subject),role:Set("member".into()),status:Set("active".into()),joined_at:Set(now),
    created_at:Set(now),updated_at:Set(now),..Default::default()}.insert(&app.fixture.admin).await?;
        let tenant=access::begin(&app.context.db,&scope(foreign_scenario.account.id,foreign_scenario.household.id),app.context.config.settings.as_ref()).await
            .map_err(|_|DbErr::Custom("Foreign household actor must have a real active restricted scope".into()))?;
        access::require_person_access(&tenant,subject,PersonAccess::View).await
            .map_err(|_|DbErr::Custom("Shared subject must be visible by its genuine foreign participation".into()))?;
        let manageable=access::require_person_access(&tenant,subject,PersonAccess::Manage).await;
        require(matches!(manageable,Err(med_tracker::models::errors::OperationError::Forbidden)),
            "Shared participation must not grant another guardian's personal Manage authority")?;
let visible=people::Entity::find_by_id(subject).one(tenant.transaction()).await?.is_some();
let query=Query::select().expr_as(Expr::col("id").count(),"count").from("guardian_declarations")
    .and_where(Expr::col("id").eq(id)).to_owned();
let declarations=tenant.transaction().query_one_raw(tenant.transaction().get_database_backend().build(&query)).await?
    .ok_or_else(||DbErr::Custom("Foreign scope visibility facts missing".into()))?.try_get::<i64>("","count")?;
let grants=person_access_grants::Entity::find().filter(person_access_grants::Column::PersonId.eq(subject))
    .count(tenant.transaction()).await?;
require(visible && declarations==0 && grants==0,
    "A visible foreign-household subject must not reveal hidden declaration or acquire fallback grants")?;
        tenant.rollback().await.map_err(|_|DbErr::Custom("Foreign scope rollback failed".into()))?;
        let foreign=foreign_token(&app,foreign_scenario.account.id).await?;
        let capabilities=app.client.get(format!("{}/api/v1/households/{}/ui_capabilities",app.origin,foreign_scenario.household.id)).bearer_auth(&foreign).send().await
            .map_err(|_|DbErr::Custom("Foreign household capabilities failed before response".into()))?;
        require(capabilities.status().as_u16()==200,"Foreign ordinary member must reach maintained aggregate capabilities")?;
        let capabilities:Value=capabilities.json().await.map_err(|_|DbErr::Custom("Foreign capabilities must return JSON".into()))?;
        require(capabilities["data"]["people"]["manage_ids"].as_array().is_some_and(|ids|!ids.contains(&json!(subject))),
            "Foreign aggregate Manage must not infer authority from the hidden declaration")?;
household_memberships::Entity::update_many()
    .col_expr(household_memberships::Column::Status,Expr::val("revoked"))
    .col_expr(household_memberships::Column::RevokedAt,utc_now())
    .col_expr(household_memberships::Column::PermissionsVersion,Expr::col(household_memberships::Column::PermissionsVersion).add(1))
    .filter(household_memberships::Column::Id.eq(participation.id)).exec(&app.fixture.admin).await?;

        let tenant=access::begin(&app.context.db,&scope(foreign_scenario.account.id,foreign_scenario.household.id),app.context.config.settings.as_ref()).await
            .map_err(|_|DbErr::Custom("Unrevoked foreign actor scope must remain usable".into()))?;
        require(matches!(access::require_person_access(&tenant,subject,PersonAccess::View).await,
            Err(med_tracker::models::errors::OperationError::Forbidden)),"Subject departure must remove live foreign clinical visibility")?;
        tenant.rollback().await.map_err(|_|DbErr::Custom("Revoked subject scope rollback failed".into()))?;
        let own=app.client.get(format!("{}/api/v1/auth/guardian-declarations",app.origin)).bearer_auth(&token).send().await
            .map_err(|_|DbErr::Custom("Own declaration refresh failed".into()))?;
        require(own.status().as_u16()==200,"Foreign participation revocation must preserve global guardian read")?;
        let own:Value=own.json().await.map_err(|_|DbErr::Custom("Own declaration list must return JSON".into()))?;
        let expected_id=id.to_string();
        require(own["data"].as_array().is_some_and(|rows|rows.iter().any(|row|row["id"].as_str()==Some(expected_id.as_str()) && serde_json::Value::is_null(&row["withdrawn_at"]))),
            "Foreign departure must retain the original Recorded personal declaration")?;
        foreign_scenario.close().await;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_real_account_closure_retains_declaration_and_restricts_destructive_deletion()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let (subject,id)=declaration(&app,&token).await?;
        let before=provenance(&app,id).await?;
        let cookie=sign_in(&app,app.fixture.account.id,&app.fixture.account.email).await;
        let (status,body)=operation_start(&app,&cookie,json!({"action":"close_account"})).await;
        require(status.as_u16()==200,"Ordinary guardian must reach maintained account closure proof")?;
        let operation=body["operation_id"].as_str().ok_or_else(||DbErr::Custom("Account closure operation ID missing".into()))?;
        require(operation_confirm(&app,&cookie,operation).await.is_success(),"Actual password-confirmed ordinary account closure must succeed")?;
let after=provenance(&app,id).await?;
let account=accounts::Entity::find_by_id(app.fixture.account.id).one(&app.fixture.admin).await?
    .ok_or_else(||DbErr::Custom("Closed-account declaration must remain present".into()))?;
let active=household_memberships::Entity::find().filter(household_memberships::Column::PersonId.eq(app.fixture.adult.id))
    .filter(household_memberships::Column::Status.eq("active")).filter(household_memberships::Column::RevokedAt.is_null())
    .count(&app.fixture.admin).await?;
let versions=versions::Entity::find().filter(versions::Column::ItemType.eq("GuardianDeclaration"))
    .filter(versions::Column::ItemId.eq(id)).count(&app.fixture.admin).await?;
require(before==after && account.status==3 && active==0 && versions==1,
    "Soft closure must retain declaration/local provenance while ending active account participation")?;
        let denied=withdraw(&app,&token,id).await?;
        require(denied.status().as_u16()==401,"Closed account's formerly valid native credential must no longer exercise declaration authority")?;
        let transaction=app.fixture.admin.begin().await?;
let membership=Query::delete().from_table("household_memberships")
    .and_where(Expr::col("household_id").eq(app.fixture.household.id)).and_where(Expr::col("person_id").eq(subject)).to_owned();
let person=Query::delete().from_table("people").and_where(Expr::col("id").eq(subject)).to_owned();
let account=Query::delete().from_table("accounts").and_where(Expr::col("id").eq(app.fixture.account.id)).to_owned();
for statement in [transaction.get_database_backend().build(&membership),
    transaction.get_database_backend().build(&person),transaction.get_database_backend().build(&account)] {
    denied_sql(&transaction,statement,"23503").await?;
}
let query=Query::delete().from_table("guardian_declarations").and_where(Expr::col("id").eq(id)).to_owned();
denied_sql(&transaction,transaction.get_database_backend().build(&query),"23514").await?;
        transaction.rollback().await?;
        Ok(())
    }.await;
    app.close().await;
    result
}

pub(super) async fn foreign_token(app: &Application, account: i64) -> Result<String, DbErr> {
    super::security::native_token(app, account).await
}

async fn clean_context(app: &Application, expected_pid: Option<i32>) -> Result<i32, DbErr> {
    let row=app.context.db.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT pg_backend_pid() AS pid,current_user=session_user
          AND pg_has_role(current_user,'med_tracker_app','MEMBER')
          AND EXISTS(SELECT 1 FROM pg_roles WHERE rolname=current_user AND NOT rolsuper AND NOT rolbypassrls) AS restricted,
          COALESCE(current_setting('med_tracker.current_account_id',true),'')='' AND
          COALESCE(current_setting('med_tracker.current_household_id',true),'')='' AND
          COALESCE(current_setting('med_tracker.current_membership_id',true),'')='' AND
          COALESCE(current_setting('med_tracker.current_invitation_token_digest',true),'')='' AS clean"))
        .await?.ok_or_else(||DbErr::Custom("Single-connection context facts missing".into()))?;
    let pid = row.try_get::<i32>("", "pid")?;
    require(
        row.try_get::<bool>("", "restricted")?
            && row.try_get::<bool>("", "clean")?
            && expected_pid.is_none_or(|expected| expected == pid),
        "All four GUCs must be empty on the same restricted App backend outside each transaction",
    )?;
    Ok(pid)
}

#[tokio::test]
async fn guardian_single_app_connection_cleans_context_after_commit_and_reached_audit_rollback()
-> Result<(), DbErr> {
    for fail_audit in [false, true] {
        let app = Application::new_single_connection(None).await;
        let result:Result<(),DbErr>=async {
            let token=app.token().await;
            let (_,id)=declaration(&app,&token).await?;
            let pid=clean_context(&app,None).await?;
            for commit in [true,false] {
                let tenant=access::begin(&app.context.db,&scope(app.fixture.account.id,app.fixture.household.id),app.context.config.settings.as_ref()).await
                    .map_err(|_|DbErr::Custom("Legitimate single-connection household scope failed".into()))?;
let query=Query::select().expr_as(Func::cust("pg_backend_pid"),"pid")
    .expr_as(Expr::expr(Func::cust("med_tracker.current_account_id")).eq(app.fixture.account.id)
        .and(Expr::expr(Func::cust("med_tracker.current_household_id")).eq(app.fixture.household.id))
        .and(Expr::expr(Func::cust("med_tracker.current_membership_id")).eq(app.fixture.membership.id)),"scoped").to_owned();
let row=tenant.transaction().query_one_raw(tenant.transaction().get_database_backend().build(&query)).await?
    .ok_or_else(||DbErr::Custom("Single-connection household context facts missing".into()))?;
require(row.try_get::<i32>("","pid")?==pid && row.try_get::<bool>("","scoped")?,"Household transactions must use the actual same restricted App backend")?;

                tenant.transaction().execute_raw(tenant.transaction().get_database_backend().build(
                    &Query::select().expr(Func::cust("set_config").args([
                        Expr::val("med_tracker.current_invitation_token_digest"),
                        Expr::val("synthetic-local-invitation-marker"), Expr::val(true),
                    ])).to_owned(),
                )).await?;
                if commit {tenant.commit().await} else {tenant.rollback().await}
                    .map_err(|_|DbErr::Custom("Owned household context transaction did not finish".into()))?;
                clean_context(&app,Some(pid)).await?;
            }
            let account=app.fixture.account.id;
            let failure=if fail_audit {"RAISE EXCEPTION 'synthetic context audit unavailable' USING ERRCODE='23514';"} else {""};
            app.fixture.admin.execute_unprepared(&format!(
                "CREATE SEQUENCE public.synthetic_guardian_context_reached;
                 GRANT USAGE,SELECT,UPDATE ON SEQUENCE public.synthetic_guardian_context_reached TO med_tracker_app,med_tracker_owner;
                 CREATE FUNCTION public.synthetic_guardian_context_check() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
                   IF NEW.item_type='GuardianDeclaration' AND NEW.event='withdraw' THEN
                     IF current_setting('med_tracker.current_account_id',true) IS DISTINCT FROM '{account}'
                       OR COALESCE(current_setting('med_tracker.current_household_id',true),'')<>''
                       OR COALESCE(current_setting('med_tracker.current_membership_id',true),'')<>''
                       OR COALESCE(current_setting('med_tracker.current_invitation_token_digest',true),'')<>'' THEN
                         RAISE EXCEPTION 'synthetic context mismatch' USING ERRCODE='23514'; END IF;
                     PERFORM setval('public.synthetic_guardian_context_reached',pg_backend_pid(),true); {failure}
                   END IF; RETURN NEW; END $$;
                 CREATE TRIGGER synthetic_guardian_context_check BEFORE INSERT ON public.versions FOR EACH ROW EXECUTE FUNCTION public.synthetic_guardian_context_check()"
            )).await?;
            let before=effects(&app).await?;
            let response=withdraw(&app,&token,id).await?;
            require(response.status().as_u16()==if fail_audit {503}else{200},"Global withdrawal must reach the expected commit or injected audit rollback")?;
            let reached=app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(
                &Query::select().columns(["is_called", "last_value"]).from("synthetic_guardian_context_reached").to_owned())).await?
                .ok_or_else(||DbErr::Custom("In-transaction context witness missing".into()))?;
            require(reached.try_get::<bool>("","is_called")? && reached.try_get::<i64>("","last_value")?==i64::from(pid),
                "Both success and failure must reach clean global context on the same App backend")?;
            if fail_audit {
                require(effects(&app).await?==before,"Reached failed global audit must roll back clinical and version effects")?;
            }
            let row=app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(
                &Query::select().column("revision").expr_as(Expr::from(Query::select()
                    .expr(Expr::col(sea_orm::sea_query::Asterisk).count()).from("versions")
                    .and_where(Expr::col("item_type").eq("GuardianDeclaration"))
                    .and_where(Expr::col("item_id").eq(id)).and_where(Expr::col("event").eq("withdraw")).to_owned()), "withdrawals")
                    .from("guardian_declarations").and_where(Expr::col("id").eq(id)).to_owned()))
                .await?.ok_or_else(||DbErr::Custom("Post-withdrawal declaration state missing".into()))?;
            require(row.try_get::<i64>("","revision")?==if fail_audit {1}else{2}
                && row.try_get::<i64>("","withdrawals")?==if fail_audit {0}else{1},"Context rollback must preserve declaration and terminal audit atomicity")?;
            clean_context(&app,Some(pid)).await?;
let foreign_scenario=crate::scenario::Scenario::new().await;
let foreign=foreign_token(&app,foreign_scenario.account.id).await?;
foreign_scenario.close().await;
            let profile=app.client.get(format!("{}/api/v1/auth/profile",app.origin)).bearer_auth(&foreign).send().await
                .map_err(|_|DbErr::Custom("Pooled foreign profile request failed".into()))?;
            require(profile.status().as_u16()==200,"Next foreign principal must first prove valid account-bound authentication")?;
            let denied=withdraw(&app,&foreign,id).await?;
            require(denied.status().as_u16()==404,"Next foreign principal must not inherit previous guardian authority")?;
            clean_context(&app,Some(pid)).await?;
            Ok(())
        }.await;
        let cleanup = app
            .fixture
            .admin
            .execute_unprepared(
                "DROP TRIGGER IF EXISTS synthetic_guardian_context_check ON public.versions;
     DROP FUNCTION IF EXISTS public.synthetic_guardian_context_check();
     DROP SEQUENCE IF EXISTS public.synthetic_guardian_context_reached",
            )
            .await;
        app.close().await;
        cleanup?;

        result?;
    }
    Ok(())
}

fn utc_now() -> sea_orm::sea_query::SimpleExpr {
    Expr::expr(
        Func::cust("timezone").args([Expr::val("UTC"), Expr::expr(Func::cust("clock_timestamp"))]),
    )
}
async fn set_account(transaction: &DatabaseTransaction, account: i64) -> Result<(), DbErr> {
    let query = Query::select()
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.current_account_id"),
            Expr::val(account.to_string()),
            Expr::val(true),
        ]))
        .to_owned();
    transaction
        .execute_raw(transaction.get_database_backend().build(&query))
        .await?;
    Ok(())
}
async fn provenance(app: &Application, id: i64) -> Result<Value, DbErr> {
    let query = Query::select()
        .expr_as(
            Func::cust("to_jsonb").arg(Expr::col(Alias::new("d"))),
            "declaration",
        )
        .expr_as(
            Func::cust("to_jsonb").arg(Expr::col(Alias::new("c"))),
            "link",
        )
        .from_as("guardian_declarations", "d")
        .join_as(
            sea_orm::sea_query::JoinType::LeftJoin,
            "carer_relationships",
            "c",
            Expr::col(("c", "guardian_declaration_id")).equals(("d", "id")),
        )
        .and_where(Expr::col(("d", "id")).eq(id))
        .to_owned();
    let row = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await?
        .ok_or_else(|| DbErr::Custom("Closure provenance prerequisite missing".into()))?;
    Ok(
        json!({"declaration":row.try_get::<Value>("","declaration")?,"link":row.try_get::<Value>("","link")?}),
    )
}
