use super::super::platform::{operation_confirm, operation_start, sign_in};
use super::{Application, effects, require, withdrawal::declaration};
use med_tracker::models::access::{self, Actor, HouseholdScope, PersonAccess};
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

async fn denied_sql(transaction: &DatabaseTransaction, sql: &str, code: &str) -> Result<(), DbErr> {
    let savepoint = transaction.begin().await?;
    let result = savepoint.execute_unprepared(sql).await;
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
        let (_, id) = declaration(&app, &token).await?;
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
              (SELECT count(*) FROM pg_constraint f WHERE f.conrelid=c.oid AND f.contype='f' AND f.confdeltype='r')=6 AS retained_foreign_keys,
              (SELECT count(*) FROM pg_policy WHERE polrelid=c.oid)=4 AS policies,
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
        transaction.execute_unprepared("SELECT set_config('med_tracker.current_account_id','71001',true)").await?;
        let row=transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT current_user='med_tracker_app' AS restricted,count(*) AS declarations FROM public.guardian_declarations WHERE id=$1",[id.into()]))
            .await?.ok_or_else(||DbErr::Custom("Restricted declaration read missing".into()))?;
        require(row.try_get::<bool>("","restricted")? && row.try_get::<i64>("","declarations")?==1,
            "Own declaration must actually be readable through the restricted runtime role")?;
        denied_sql(&transaction,&format!("UPDATE public.guardian_declarations SET guardian_person_id=73002 WHERE id={id}"),"42501").await?;
        denied_sql(&transaction,&format!("DELETE FROM public.guardian_declarations WHERE id={id}"),"42501").await?;
        denied_sql(&transaction,"SELECT med_tracker.guardian_declaration_one_way()","42501").await?;
        transaction.rollback().await?;
        require(withdraw(&app,&token,id).await?.status().as_u16()==200,"Terminal-transition prerequisite must use genuine withdrawal")?;
        let transaction=app.context.db.begin().await?;
        transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app").await?;
        transaction.execute_unprepared("SELECT set_config('med_tracker.current_account_id','71001',true)").await?;
        denied_sql(&transaction,&format!("UPDATE public.guardian_declarations SET withdrawn_at=NULL,revision=1 WHERE id={id}"),"23514").await?;
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
        app.fixture.admin.execute_unprepared(
            "INSERT INTO public.people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73994,'Synthetic foreign household actor',0,true,now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES(71994,73994,'guardian-storage-foreign@example.test',2,now(),now());
             INSERT INTO public.users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES(77994,73994,'guardian-storage-foreign@example.test',crypt('password',gen_salt('bf',4)),true,now(),now());
             INSERT INTO public.households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72994,71994,'Synthetic foreign care household','guardian-storage-foreign','UTC',now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74994,72994,73994,'member','active',now(),now(),now())"
        ).await?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO public.household_memberships(household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(72994,$1,'member','active',now(),now(),now())",[subject.into()])).await?;
        let tenant=access::begin(&app.context.db,&scope(71994,72994),app.context.config.settings.as_ref()).await
            .map_err(|_|DbErr::Custom("Foreign household actor must have a real active restricted scope".into()))?;
        access::require_person_access(&tenant,subject,PersonAccess::View).await
            .map_err(|_|DbErr::Custom("Shared subject must be visible by its genuine foreign participation".into()))?;
        let manageable=access::require_person_access(&tenant,subject,PersonAccess::Manage).await;
        require(matches!(manageable,Err(med_tracker::models::errors::OperationError::Forbidden)),
            "Shared participation must not grant another guardian's personal Manage authority")?;
        let row=tenant.transaction().query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.people WHERE id=$1) AS people,(SELECT count(*) FROM public.guardian_declarations WHERE id=$2) AS declarations,(SELECT count(*) FROM public.person_access_grants WHERE person_id=$1) AS grants",
            [subject.into(),id.into()])).await?.ok_or_else(||DbErr::Custom("Foreign scope visibility facts missing".into()))?;
        require(row.try_get::<i64>("","people")?==1 && row.try_get::<i64>("","declarations")?==0 && row.try_get::<i64>("","grants")?==0,
            "A visible foreign-household subject must not reveal hidden declaration or acquire fallback grants")?;
        tenant.rollback().await.map_err(|_|DbErr::Custom("Foreign scope rollback failed".into()))?;
        let foreign=foreign_token(&app,71994).await?;
        let capabilities=app.client.get(format!("{}/api/v1/households/72994/ui_capabilities",app.origin)).bearer_auth(&foreign).send().await
            .map_err(|_|DbErr::Custom("Foreign household capabilities failed before response".into()))?;
        require(capabilities.status().as_u16()==200,"Foreign ordinary member must reach maintained aggregate capabilities")?;
        let capabilities:Value=capabilities.json().await.map_err(|_|DbErr::Custom("Foreign capabilities must return JSON".into()))?;
        require(capabilities["data"]["people"]["manage_ids"].as_array().is_some_and(|ids|!ids.contains(&json!(subject))),
            "Foreign aggregate Manage must not infer authority from the hidden declaration")?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE public.household_memberships SET status='revoked',revoked_at=timezone('UTC',clock_timestamp()),permissions_version=permissions_version+1 WHERE person_id=$1 AND household_id=72994",[subject.into()])).await?;
        let tenant=access::begin(&app.context.db,&scope(71994,72994),app.context.config.settings.as_ref()).await
            .map_err(|_|DbErr::Custom("Unrevoked foreign actor scope must remain usable".into()))?;
        require(matches!(access::require_person_access(&tenant,subject,PersonAccess::View).await,
            Err(med_tracker::models::errors::OperationError::Forbidden)),"Subject departure must remove live foreign clinical visibility")?;
        tenant.rollback().await.map_err(|_|DbErr::Custom("Revoked subject scope rollback failed".into()))?;
        let own=app.client.get(format!("{}/api/v1/auth/guardian-declarations",app.origin)).bearer_auth(&token).send().await
            .map_err(|_|DbErr::Custom("Own declaration refresh failed".into()))?;
        require(own.status().as_u16()==200,"Foreign participation revocation must preserve global guardian read")?;
        let own:Value=own.json().await.map_err(|_|DbErr::Custom("Own declaration list must return JSON".into()))?;
        let expected_id=id.to_string();
        require(own["data"].as_array().is_some_and(|rows|rows.iter().any(|row|row["id"].as_str()==Some(expected_id.as_str()) && row["withdrawn_at"].is_null())),
            "Foreign departure must retain the original Recorded personal declaration")?;
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
        let before=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT to_jsonb(d) AS declaration,(SELECT to_jsonb(c) FROM public.carer_relationships c WHERE c.guardian_declaration_id=d.id) AS link FROM public.guardian_declarations d WHERE d.id=$1",[id.into()])).await?
            .ok_or_else(||DbErr::Custom("Closure provenance prerequisite missing".into()))?;
        let cookie=sign_in(&app,71001,"persistence@example.test").await;
        let (status,body)=operation_start(&app,&cookie,json!({"action":"close_account"})).await;
        require(status.as_u16()==200,"Ordinary guardian must reach maintained account closure proof")?;
        let operation=body["operation_id"].as_str().ok_or_else(||DbErr::Custom("Account closure operation ID missing".into()))?;
        require(operation_confirm(&app,&cookie,operation).await.is_success(),"Actual password-confirmed ordinary account closure must succeed")?;
        let after=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT to_jsonb(d) AS declaration,(SELECT to_jsonb(c) FROM public.carer_relationships c WHERE c.guardian_declaration_id=d.id) AS link,(SELECT status FROM public.accounts WHERE id=71001) AS account_status,(SELECT count(*) FROM public.household_memberships WHERE person_id=73001 AND status='active' AND revoked_at IS NULL) AS active_memberships,(SELECT count(*) FROM public.versions WHERE item_type='GuardianDeclaration' AND item_id=d.id) AS versions FROM public.guardian_declarations d WHERE d.id=$1",[id.into()])).await?
            .ok_or_else(||DbErr::Custom("Closed-account declaration must remain present".into()))?;
        require(before.try_get::<Value>("","declaration")?==after.try_get::<Value>("","declaration")?
            && before.try_get::<Value>("","link")?==after.try_get::<Value>("","link")?
            && after.try_get::<i32>("","account_status")?==3 && after.try_get::<i64>("","active_memberships")?==0
            && after.try_get::<i64>("","versions")?==1,
            "Soft closure must retain declaration/local provenance while ending active account participation")?;
        let denied=withdraw(&app,&token,id).await?;
        require(denied.status().as_u16()==401,"Closed account's formerly valid native credential must no longer exercise declaration authority")?;
        let transaction=app.fixture.admin.begin().await?;
        for sql in [format!("DELETE FROM public.household_memberships WHERE household_id=72001 AND person_id={subject}"),
            format!("DELETE FROM public.people WHERE id={subject}"),"DELETE FROM public.accounts WHERE id=71001".into()] {
            denied_sql(&transaction,&sql,"23503").await?;
        }
        denied_sql(&transaction,&format!("DELETE FROM public.guardian_declarations WHERE id={id}"),"23514").await?;
        transaction.rollback().await?;
        Ok(())
    }.await;
    app.close().await;
    result
}

async fn foreign_token(app: &Application, account: i64) -> Result<String, DbErr> {
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO public.oauth_grants(id,account_id,oauth_application_id,client_kind,code,code_challenge,code_challenge_method,redirect_uri,scopes,expires_in,authenticated_at,last_used_at,created_at,updated_at) VALUES(76994,$1,75001,'mobile','synthetic-storage-foreign-code','E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM','S256','io.damacus.medtracker:/oauth2redirect','medtracker offline_access',now()+interval '5 minutes',now(),now(),now(),now())",[account.into()])).await?;
    let fields = serde_urlencoded::to_string([
        ("grant_type", "authorization_code"),
        ("client_id", "native"),
        ("code", "synthetic-storage-foreign-code"),
        ("redirect_uri", "io.damacus.medtracker:/oauth2redirect"),
        (
            "code_verifier",
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
        ),
    ])
    .map_err(|_| DbErr::Custom("Foreign native exchange encoding failed".into()))?;
    let response = app
        .client
        .post(format!("{}/token", app.origin))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(fields)
        .send()
        .await
        .map_err(|_| DbErr::Custom("Foreign native exchange failed before response".into()))?;
    require(
        response.status().as_u16() == 200,
        "Foreign principal must use a genuinely issued native credential",
    )?;
    let response: Value = response
        .json()
        .await
        .map_err(|_| DbErr::Custom("Foreign exchange must return JSON".into()))?;
    response["access_token"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| DbErr::Custom("Foreign access token missing".into()))
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
        let app = Application::new_single_connection().await;
        let result:Result<(),DbErr>=async {
            let token=app.token().await;
            let (_,id)=declaration(&app,&token).await?;
            let pid=clean_context(&app,None).await?;
            for commit in [true,false] {
                let tenant=access::begin(&app.context.db,&scope(71001,72001),app.context.config.settings.as_ref()).await
                    .map_err(|_|DbErr::Custom("Legitimate single-connection household scope failed".into()))?;
                let row=tenant.transaction().query_one_raw(Statement::from_string(DbBackend::Postgres,
                    "SELECT pg_backend_pid() AS pid,med_tracker.current_account_id()=71001 AND med_tracker.current_household_id()=72001 AND med_tracker.current_membership_id()=74001 AS scoped"))
                    .await?.ok_or_else(||DbErr::Custom("Single-connection household context facts missing".into()))?;
                require(row.try_get::<i32>("","pid")?==pid && row.try_get::<bool>("","scoped")?,"Household transactions must use the actual same restricted App backend")?;
                tenant.transaction().execute_unprepared("SELECT set_config('med_tracker.current_invitation_token_digest','synthetic-local-invitation-marker',true)").await?;
                if commit {tenant.commit().await} else {tenant.rollback().await}
                    .map_err(|_|DbErr::Custom("Owned household context transaction did not finish".into()))?;
                clean_context(&app,Some(pid)).await?;
            }
            let failure=if fail_audit {"RAISE EXCEPTION 'synthetic context audit unavailable' USING ERRCODE='23514';"} else {""};
            app.fixture.admin.execute_unprepared(&format!(
                "CREATE SEQUENCE public.synthetic_guardian_context_reached;
                 GRANT USAGE,SELECT,UPDATE ON SEQUENCE public.synthetic_guardian_context_reached TO med_tracker_app,med_tracker_owner;
                 CREATE FUNCTION public.synthetic_guardian_context_check() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
                   IF NEW.item_type='GuardianDeclaration' AND NEW.event='withdraw' THEN
                     IF current_setting('med_tracker.current_account_id',true) IS DISTINCT FROM '71001'
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
            require(response.status().as_u16()==if fail_audit {500}else{200},"Global withdrawal must reach the expected commit or injected audit rollback")?;
            let reached=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
                "SELECT is_called,last_value FROM public.synthetic_guardian_context_reached")).await?
                .ok_or_else(||DbErr::Custom("In-transaction context witness missing".into()))?;
            require(reached.try_get::<bool>("","is_called")? && reached.try_get::<i64>("","last_value")?==i64::from(pid),
                "Both success and failure must reach clean global context on the same App backend")?;
            if fail_audit {
                require(effects(&app).await?==before,"Reached failed global audit must roll back clinical and version effects")?;
            }
            let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                "SELECT revision,(SELECT count(*) FROM public.versions v WHERE v.item_type='GuardianDeclaration' AND v.item_id=d.id AND v.event='withdraw') AS withdrawals FROM public.guardian_declarations d WHERE d.id=$1",[id.into()]))
                .await?.ok_or_else(||DbErr::Custom("Post-withdrawal declaration state missing".into()))?;
            require(row.try_get::<i64>("","revision")?==if fail_audit {1}else{2}
                && row.try_get::<i64>("","withdrawals")?==if fail_audit {0}else{1},"Context rollback must preserve declaration and terminal audit atomicity")?;
            clean_context(&app,Some(pid)).await?;
            app.fixture.admin.execute_unprepared(
                "INSERT INTO public.people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73994,'Synthetic pooled foreign owner',0,true,now(),now());
                 INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES(71994,73994,'pooled-guardian-foreign@example.test',2,now(),now())"
            ).await?;
            let foreign=foreign_token(&app,71994).await?;
            let profile=app.client.get(format!("{}/api/v1/auth/profile",app.origin)).bearer_auth(&foreign).send().await
                .map_err(|_|DbErr::Custom("Pooled foreign profile request failed".into()))?;
            require(profile.status().as_u16()==200,"Next foreign principal must first prove valid account-bound authentication")?;
            let denied=withdraw(&app,&foreign,id).await?;
            require(denied.status().as_u16()==404,"Next foreign principal must not inherit previous guardian authority")?;
            clean_context(&app,Some(pid)).await?;
            Ok(())
        }.await;
        app.close().await;
        result?;
    }
    Ok(())
}
