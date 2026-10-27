use super::{
    Application, effects, identity, require,
    withdrawal::{declaration, no_store},
};
use med_tracker::models::_entities::{
    account_lockouts, accounts, audit_export_deliveries, audit_ledger_entries,
    household_memberships, oauth_applications, oauth_grants, versions,
};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter, Statement,
    sea_query::{Alias, Expr, ExprTrait, Func, Query},
};

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
    let query = Query::select()
        .expr_as(
            Func::cust("to_jsonb").arg(Expr::col(Alias::new("declaration"))),
            "state",
        )
        .from_as("guardian_declarations", "declaration")
        .and_where(Expr::col(("declaration", "id")).eq(id))
        .to_owned();
    let declaration = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await?
        .ok_or_else(|| DbErr::Custom("Guardian persisted state missing".into()))?
        .try_get::<Value>("", "state")?;
    let history = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("GuardianDeclaration"))
        .filter(versions::Column::ItemId.eq(id))
        .all(&app.fixture.admin)
        .await?;
    let ledger = audit_ledger_entries::Entity::find()
        .filter(audit_ledger_entries::Column::SourceTable.eq("versions"))
        .filter(audit_ledger_entries::Column::SourceId.is_in(history.iter().map(|row| row.id)))
        .all(&app.fixture.admin)
        .await?;
    let deliveries = audit_export_deliveries::Entity::find()
        .filter(
            audit_export_deliveries::Column::AuditLedgerEntryId
                .is_in(ledger.iter().map(|row| row.id)),
        )
        .count(&app.fixture.admin)
        .await?;
    let memberships = household_memberships::Entity::find()
        .filter(household_memberships::Column::PersonId.eq(app.fixture.adult.id))
        .filter(household_memberships::Column::Status.eq("active"))
        .filter(household_memberships::Column::RevokedAt.is_null())
        .count(&app.fixture.admin)
        .await?;
    Ok(
        json!({"declaration":declaration,"withdrawals":history.iter().filter(|row|row.event=="withdraw").count(),"ledger":ledger.len(),
    "deliveries":deliveries,"active_guardian_memberships":memberships}),
    )
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

pub(super) async fn native_token(app: &Application, account: i64) -> Result<String, DbErr> {
    let code = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();
    let mut grant = app.fixture.authorization.clone().into_active_model();
    grant.id = NotSet;
    grant.account_id = Set(account);
    grant.code = Set(Some(code.clone()));
    grant.expires_in = Set(now + chrono::Duration::minutes(5));
    grant.authenticated_at = Set(Some(now));
    grant.last_used_at = Set(Some(now));
    grant.insert(&app.fixture.admin).await?;
    exchange(
        app,
        &app.fixture.client_id,
        &code,
        &app.fixture.application.redirect_uri,
        None,
    )
    .await
}

#[tokio::test]
async fn guardian_global_withdrawal_excludes_valid_household_and_integration_credentials()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let native=app.token().await;
        let (_,id)=declaration(&app,&native).await?;
        super::super::account_sessions::stored_credentials(&app).await;
        let current = super::super::account_sessions::credential_token(&app,"current");
        let app_token = super::super::account_sessions::credential_token(&app,"app");
        let key=super::super::profile::issue_profile_personal_key(&app,"Guardian exclusion").await;
        for (kind,token,expected) in [("household session",current.as_str(),403_u16),
            ("household app token",app_token.as_str(),403_u16),("personal key",key.as_str(),401_u16)] {
            let positive=app.client.get(format!("{}/api/v1/households/{}/people",app.origin,app.fixture.household.id))
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
        let client_id = format!("guardian-integration-{}",uuid::Uuid::new_v4());
        let code = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().naive_utc();
        let application = oauth_applications::ActiveModel {
            account_id:Set(Some(app.fixture.account.id)), client_id:Set(client_id.clone()),
            client_kind:Set("integration".into()), name:Set("Synthetic guardian integration".into()),
            redirect_uri:Set("https://example.test/guardian-callback".into()),
            scopes:Set("launch/patient patient/*.rs offline_access".into()),
            token_endpoint_auth_method:Set("client_secret_post".into()),
            client_secret_hash:Set(Some(bcrypt::hash("password",4).map_err(|error|DbErr::Custom(error.to_string()))?)),
            created_at:Set(now),updated_at:Set(now),..Default::default()
        }.insert(&app.fixture.admin).await?;
        oauth_grants::ActiveModel {
            account_id:Set(app.fixture.account.id),oauth_application_id:Set(application.id),
            client_kind:Set("integration".into()),household_membership_id:Set(Some(app.fixture.membership.id)),
            person_id:Set(Some(app.fixture.adult.id)),permissions_version:Set(Some(app.fixture.membership.permissions_version)),
            code:Set(Some(code.clone())),code_challenge:Set(Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".into())),
            code_challenge_method:Set(Some("S256".into())),redirect_uri:Set(Some(application.redirect_uri.clone())),
            scopes:Set(application.scopes.clone()),expires_in:Set(now+chrono::Duration::minutes(5)),
            authenticated_at:Set(Some(now)),last_used_at:Set(Some(now)),created_at:Set(now),updated_at:Set(now),..Default::default()
        }.insert(&app.fixture.admin).await?;
        let integration=exchange(&app,&client_id,&code,&application.redirect_uri,Some("password")).await?;
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
    let result = async {
        let token = app.token().await;
        let (_, id) = declaration(&app, &token).await?;
        let foreign_account = app.fixture.foreign_household().await;
        household_memberships::Entity::delete_by_id(foreign_account.membership.id)
            .exec(&app.fixture.admin)
            .await?;
        let code = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().naive_utc();
        let mut grant = app.fixture.authorization.clone().into_active_model();
        grant.id = NotSet;
        grant.account_id = Set(foreign_account.account.id);
        grant.code = Set(Some(code.clone()));
        grant.expires_in = Set(now + chrono::Duration::minutes(5));
        grant.authenticated_at = Set(Some(now));
        grant.last_used_at = Set(Some(now));
        grant.insert(&app.fixture.admin).await?;
        let foreign = exchange(
            &app,
            &app.fixture.client_id,
            &code,
            &app.fixture.application.redirect_uri,
            None,
        )
        .await?;
        foreign_account.close().await;
        let positive = app
            .client
            .get(format!("{}/api/v1/auth/profile", app.origin))
            .bearer_auth(&foreign)
            .send()
            .await
            .map_err(|_| {
                DbErr::Custom("Foreign native profile prerequisite failed before response".into())
            })?;
        require(
            positive.status().as_u16() == 200,
            "Foreign zero-household native Account must be genuinely authenticated",
        )?;
        let before = state(&app, id).await?;
        let denied = withdrawal(&app, &foreign, id).await?;
        require(
            denied.status().as_u16() == 404 && no_store(&denied),
            "A valid foreign Account must not discover or withdraw another guardian's declaration",
        )?;
        require(
            state(&app, id).await? == before,
            "Foreign denial must preserve declaration and audit",
        )?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_terminal_replay_rechecks_revoked_locked_and_closed_native_authority()
-> Result<(), DbErr> {
    for mutation in 0..3 {
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
            let now = chrono::Utc::now().naive_utc();
            match mutation {
                0 => {
                    oauth_grants::Entity::update_many()
                        .col_expr(oauth_grants::Column::RevokedAt, Expr::val(now))
                        .filter(oauth_grants::Column::Id.eq(app.fixture.authorization.id))
                        .exec(&app.fixture.admin)
                        .await?;
                }
                1 => {
                    account_lockouts::ActiveModel {
                        account_id: Set(app.fixture.account.id),
                        key: Set(uuid::Uuid::new_v4().to_string()),
                        deadline: Set(now + chrono::Duration::hours(1)),
                        ..Default::default()
                    }
                    .insert(&app.fixture.admin)
                    .await?;
                }
                _ => {
                    accounts::Entity::update_many()
                        .col_expr(accounts::Column::Status, Expr::val(3))
                        .filter(accounts::Column::Id.eq(app.fixture.account.id))
                        .exec(&app.fixture.admin)
                        .await?;
                }
            }

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
        let cleanup = app.fixture.admin.execute_unprepared(
            "DROP TRIGGER IF EXISTS synthetic_guardian_withdrawal_failure ON public.versions;
             DROP TRIGGER IF EXISTS synthetic_guardian_withdrawal_failure ON public.audit_ledger_entries;
             DROP FUNCTION IF EXISTS public.synthetic_guardian_withdrawal_failure();
             DROP SEQUENCE IF EXISTS public.synthetic_guardian_withdrawal_reached"
        ).await;
        app.close().await;
        cleanup?;

        result?;
    }
    Ok(())
}
