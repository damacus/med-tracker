use super::{
    Application, effects, identity, require,
    withdrawal::{declaration, no_store},
};
use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};
use serde_json::{Value, json};

async fn withdrawal(app: &Application, token: &str, id: i64) -> Result<reqwest::Response, DbErr> {
    app.client
        .patch(format!(
            "{}/api/v1/auth/guardian-declarations/{id}",
            app.origin
        ))
        .bearer_auth(token)
        .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
        .send()
        .await
        .map_err(|_| DbErr::Custom("Guardian withdrawal request failed before response".into()))
}

async fn state(app: &Application, id: i64) -> Result<Value, DbErr> {
    app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT jsonb_build_object('declaration',(SELECT to_jsonb(d) FROM public.guardian_declarations d WHERE d.id=$1),'withdrawals',(SELECT count(*) FROM public.versions WHERE item_type='GuardianDeclaration' AND item_id=$1 AND event='withdraw'),'ledger',(SELECT count(*) FROM public.audit_ledger_entries),'deliveries',(SELECT count(*) FROM public.audit_export_deliveries),'active_guardian_memberships',(SELECT count(*) FROM public.household_memberships WHERE person_id=73001 AND status='active' AND revoked_at IS NULL)) AS state",
        [id.into()])).await?.ok_or_else(||DbErr::Custom("Guardian persisted state missing".into()))?.try_get("","state")
}

async fn exchange(
    app: &Application,
    client: &str,
    code: &str,
    redirect: &str,
    secret: Option<&str>,
) -> Result<String, DbErr> {
    let mut fields = vec![
        ("grant_type", "authorization_code"),
        ("client_id", client),
        ("code", code),
        ("redirect_uri", redirect),
        (
            "code_verifier",
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
        ),
    ];
    if let Some(secret) = secret {
        fields.push(("client_secret", secret));
    }
    let encoded = serde_urlencoded::to_string(fields)
        .map_err(|_| DbErr::Custom("Credential exchange form encoding failed".into()))?;
    let response = app
        .client
        .post(format!("{}/token", app.origin))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(encoded)
        .send()
        .await
        .map_err(|_| {
            DbErr::Custom("Maintained credential exchange failed before response".into())
        })?;
    require(
        response.status().as_u16() == 200,
        "Maintained credential exchange must succeed before exclusion",
    )?;
    let body: Value = response
        .json()
        .await
        .map_err(|_| DbErr::Custom("Credential exchange returned invalid JSON".into()))?;
    body["access_token"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| DbErr::Custom("Credential exchange omitted access token".into()))
}

#[tokio::test]
async fn guardian_global_withdrawal_excludes_valid_household_and_integration_credentials()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let native=app.token().await;
        let (_,id)=declaration(&app,&native).await?;
        super::super::account_sessions::stored_credentials(&app).await;
        let key=super::super::profile::issue_profile_personal_key(&app,"Guardian exclusion").await;
        for (kind,token,expected) in [("household session","synthetic-current-session",403_u16),
            ("household app token","synthetic-app-token",403_u16),("personal key",key.as_str(),401_u16)] {
            let positive=app.client.get(format!("{}/api/v1/households/72001/people",app.origin))
                .bearer_auth(token).send().await.map_err(|_|DbErr::Custom("Local credential prerequisite failed before response".into()))?;
            require(positive.status().as_u16()==200,&format!("The {kind} must first prove valid local care access"))?;
            let before=state(&app,id).await?;
            let clinical=effects(&app).await?;
            let identities=identity(&app).await?;
            let denied=withdrawal(&app,token,id).await?;
            require(denied.status().as_u16()==expected && no_store(&denied),&format!("The valid {kind} must not acquire global withdrawal authority"))?;
            require(state(&app,id).await?==before && effects(&app).await?==clinical && identity(&app).await?==identities,
                "Credential exclusion must preserve declaration, clinical state, identities and audit")?;
        }
        app.fixture.admin.execute_unprepared(
            "INSERT INTO public.oauth_applications(id,account_id,client_id,client_kind,name,redirect_uri,scopes,token_endpoint_auth_method,client_secret_hash,created_at,updated_at) VALUES(75991,71001,'smart-guardian-exclusion','integration','Synthetic guardian integration','https://example.test/guardian-callback','launch/patient patient/*.rs offline_access','client_secret_post',crypt('password',gen_salt('bf',4)),now(),now());
             INSERT INTO public.oauth_grants(id,account_id,oauth_application_id,client_kind,household_membership_id,person_id,permissions_version,code,code_challenge,code_challenge_method,redirect_uri,scopes,expires_in,authenticated_at,last_used_at,created_at,updated_at) VALUES(76991,71001,75991,'integration',74001,73001,1,'synthetic-guardian-integration-code','E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM','S256','https://example.test/guardian-callback','launch/patient patient/*.rs offline_access',now()+interval '5 minutes',now(),now(),now(),now())"
        ).await?;
        let integration=exchange(&app,"smart-guardian-exclusion","synthetic-guardian-integration-code","https://example.test/guardian-callback",Some("password")).await?;
        let before=state(&app,id).await?;
        let denied=withdrawal(&app,&integration,id).await?;
        require(denied.status().as_u16()==401 && no_store(&denied),"A genuinely exchanged integration credential must not withdraw global declarations")?;
        require(state(&app,id).await?==before,"Integration denial must preserve declaration and audit state")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_withdrawal_rejects_foreign_owner_after_live_native_profile_prerequisite()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let (_,id)=declaration(&app,&token).await?;
        app.fixture.admin.execute_unprepared(
            "INSERT INTO public.people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73991,'Synthetic other declaration owner',0,true,now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at) VALUES(71991,73991,'other-guardian@example.test',2,now(),now());
             INSERT INTO public.oauth_grants(id,account_id,oauth_application_id,client_kind,code,code_challenge,code_challenge_method,redirect_uri,scopes,expires_in,authenticated_at,last_used_at,created_at,updated_at) VALUES(76992,71991,75001,'mobile','synthetic-other-guardian-code','E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM','S256','io.damacus.medtracker:/oauth2redirect','medtracker offline_access',now()+interval '5 minutes',now(),now(),now(),now())"
        ).await?;
        let foreign=exchange(&app,"native","synthetic-other-guardian-code","io.damacus.medtracker:/oauth2redirect",None).await?;
        let positive=app.client.get(format!("{}/api/v1/auth/profile",app.origin)).bearer_auth(&foreign).send().await
            .map_err(|_|DbErr::Custom("Foreign native profile prerequisite failed before response".into()))?;
        require(positive.status().as_u16()==200,"Foreign zero-household native Account must be genuinely authenticated")?;
        let before=state(&app,id).await?;
        let denied=withdrawal(&app,&foreign,id).await?;
        require(denied.status().as_u16()==404 && no_store(&denied),"A valid foreign Account must not discover or withdraw another guardian's declaration")?;
        require(state(&app,id).await?==before,"Foreign denial must preserve declaration and audit")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_terminal_replay_rechecks_revoked_locked_and_closed_native_authority()
-> Result<(), DbErr> {
    for mutation in [
        "UPDATE public.oauth_grants SET revoked_at=now() WHERE id=76001",
        "INSERT INTO public.account_lockouts(account_id,key,deadline) VALUES(71001,'synthetic-guardian-lock',now()+interval '1 hour')",
        "UPDATE public.accounts SET status=3 WHERE id=71001",
    ] {
        let app = Application::new().await;
        let result: Result<(), DbErr> = async {
            let token = app.token().await;
            let (_, id) = declaration(&app, &token).await?;
            let first = withdrawal(&app, &token, id).await?;
            require(
                first.status().as_u16() == 200,
                "Live native guardian must first commit its own withdrawal",
            )?;
            let before = state(&app, id).await?;
            let replay = withdrawal(&app, &token, id).await?;
            require(
                replay.status().as_u16() == 200 && state(&app, id).await? == before,
                "Live terminal replay must add no second version or ledger event",
            )?;
            app.fixture.admin.execute_unprepared(mutation).await?;
            let before = state(&app, id).await?;
            let denied = withdrawal(&app, &token, id).await?;
            require(
                denied.status().as_u16() == 401 && no_store(&denied),
                "Terminal replay must recheck exact credential and Account liveness",
            )?;
            require(
                state(&app, id).await? == before,
                "Revoked or locked terminal replay must have no state or audit effect",
            )?;
            Ok(())
        }
        .await;
        app.close().await;
        result?;
    }
    Ok(())
}

#[tokio::test]
async fn guardian_withdrawal_reached_version_and_ledger_failures_rollback_and_retry()
-> Result<(), DbErr> {
    for ledger in [false, true] {
        let app = Application::new().await;
        let result: Result<(), DbErr> = async {
            let token=app.token().await;
            let (_,id)=declaration(&app,&token).await?;
            let (table,predicate)=if ledger {("audit_ledger_entries","NEW.source_table='versions' AND NEW.source_payload->>'item_type'='GuardianDeclaration' AND NEW.source_payload->>'event'='withdraw'")} else {("versions","NEW.item_type='GuardianDeclaration' AND NEW.event='withdraw'")};
            app.fixture.admin.execute_unprepared(&format!(
                "CREATE SEQUENCE public.synthetic_guardian_withdrawal_reached;
                 GRANT USAGE,SELECT ON SEQUENCE public.synthetic_guardian_withdrawal_reached TO med_tracker_app,med_tracker_owner;
                 CREATE FUNCTION public.synthetic_guardian_withdrawal_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF {predicate} THEN PERFORM nextval('public.synthetic_guardian_withdrawal_reached'); RAISE EXCEPTION 'synthetic guardian audit failure' USING ERRCODE='23514'; END IF; RETURN NEW; END $$;
                 CREATE TRIGGER synthetic_guardian_withdrawal_failure BEFORE INSERT ON public.{table} FOR EACH ROW EXECUTE FUNCTION public.synthetic_guardian_withdrawal_failure()"
            )).await?;
            let before=state(&app,id).await?;
            let clinical=effects(&app).await?;
            let failed=withdrawal(&app,&token,id).await?;
            require(failed.status().as_u16()==500 && no_store(&failed),"Reached audit failure must safely deny withdrawal")?;
            let body:Value=failed.json().await.map_err(|_|DbErr::Custom("Audit failure response must be JSON".into()))?;
            require(body["error"]["code"]=="internal_error" && !body.to_string().contains("synthetic guardian audit failure"),"Audit failure must not disclose raw database detail")?;
            let reached=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
                "SELECT is_called AS reached FROM public.synthetic_guardian_withdrawal_reached")).await?
                .ok_or_else(||DbErr::Custom("Audit reached marker missing".into()))?.try_get::<bool>("","reached")?;
            require(reached,"Audit rollback must prove the injected version/ledger failure was reached")?;
            require(state(&app,id).await?==before && effects(&app).await?==clinical,"Audit failure must roll back declaration, version, ledger, delivery and clinical state")?;
            app.fixture.admin.execute_unprepared(&format!("DROP TRIGGER synthetic_guardian_withdrawal_failure ON public.{table}")).await?;
            let retried=withdrawal(&app,&token,id).await?;
            require(retried.status().as_u16()==200,"Retry after recovered audit availability must commit withdrawal")?;
            let committed=state(&app,id).await?;
            require(committed["withdrawals"]==1 && committed["declaration"]["revision"]==2,"Recovered withdrawal must have one terminal event")?;
            let replay=withdrawal(&app,&token,id).await?;
            require(replay.status().as_u16()==200 && state(&app,id).await?==committed,"Recovered terminal replay must preserve exact audit count")?;
            Ok(())
        }.await;
        app.close().await;
        result?;
    }
    Ok(())
}
