use super::super::platform::{form_page, hidden_field, post_form, response_cookies, sign_in};
use super::{Application, create, dependent, effects, identity, require};
use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};
use serde_json::{Value, json};

pub(super) async fn declaration(app: &Application, token: &str) -> Result<(i64, i64), DbErr> {
    let mut person = dependent("Synthetic personal guardian record", "minor");
    person["initial_guardian_declaration"] = json!(true);
    let response = create(app, token, &person, "synthetic-own-guardian-record").await?;
    require(
        response.status().as_u16() == 201,
        "Withdrawal setup requires a genuinely created dependent",
    )?;
    let body: Value = response
        .json()
        .await
        .map_err(|_| DbErr::Custom("Created dependent response unavailable".to_owned()))?;
    let subject = body["data"]["id"]
        .as_i64()
        .or_else(|| {
            body["data"]["id"]
                .as_str()
                .and_then(|value| value.parse().ok())
        })
        .ok_or_else(|| DbErr::Custom("Created dependent ID unavailable".to_owned()))?;
    let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT id FROM public.guardian_declarations WHERE subject_person_id=$1 AND guardian_person_id=73001",
        [subject.into()])).await?.ok_or_else(||DbErr::Custom("Committed declaration unavailable".to_owned()))?;
    Ok((subject, row.try_get("", "id")?))
}

pub(super) fn no_store(response: &reqwest::Response) -> bool {
    response
        .headers()
        .get("cache-control")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|directive| directive.trim() == "no-store")
        })
}

#[tokio::test]
async fn guardian_withdrawal_survives_last_departure_without_current_foreign_profile_reads()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let token = app.token().await;
        let (subject,id) = declaration(&app,&token).await?;
        app.fixture.admin.execute_unprepared("UPDATE public.household_memberships SET status='revoked',revoked_at=timezone('UTC',clock_timestamp()),permissions_version=permissions_version+1 WHERE person_id=73001").await?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE public.people SET name='Synthetic future profile must remain hidden' WHERE id=$1",
            [subject.into()])).await?;
        let profile = app.client.get(format!("{}/api/v1/auth/profile",app.origin))
            .bearer_auth(&token).send().await.map_err(|_|DbErr::Custom("Zero-household profile prerequisite failed".to_owned()))?;
        require(profile.status().as_u16()==200,"The same native credential must remain valid with zero households")?;
        let endpoint = format!("{}/api/v1/auth/guardian-declarations",app.origin);
        let listed = app.client.get(format!("{endpoint}?limit=1")).bearer_auth(&token)
            .send().await.map_err(|_|DbErr::Custom("Own declaration list failed before response".to_owned()))?;
        require(listed.status().as_u16()==200,
            &format!("Own declaration list must remain200 after last departure, received{}",listed.status().as_u16()))?;
        require(no_store(&listed),"Own declaration list must be no-store")?;
        let body: Value = listed.json().await.map_err(|_|DbErr::Custom("Own declaration list must be JSON".to_owned()))?;
        let rows = body["data"].as_array().ok_or_else(||DbErr::Custom("Own declaration list must contain rows".to_owned()))?;
        require(rows.len()==1 && body["meta"]["next_before_id"].is_null(),"Own bounded list must return its one declaration")?;
        let row = rows.first().ok_or_else(||DbErr::Custom("Expected own declaration row".to_owned()))?;
        let expected_id=id.to_string();
        let expected_subject=subject.to_string();
        require(row["id"].as_str()==Some(expected_id.as_str()) && row["subject_person_id"].as_str()==Some(expected_subject.as_str())
            && row["subject_name_at_declaration"]=="Synthetic personal guardian record"
            && row["revision"]=="1" && row["withdrawn_at"].is_null(),
            "Own list must expose captured declaration facts and string IDs/revision")?;
        require(!body.to_string().contains("Synthetic future profile must remain hidden"),"Departure must not expose the subject's future profile")?;
        let original_identity = identity(&app).await?;
        let original_effects = effects(&app).await?;
        for (revision,status) in [("0",409_u16),("3",409_u16),("9223372036854775808",422_u16)] {
            let denied = app.client.patch(format!("{endpoint}/{id}")).bearer_auth(&token)
                .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":revision}}))
                .send().await.map_err(|_|DbErr::Custom("Revision validation request failed".to_owned()))?;
            require(denied.status().as_u16()==status && no_store(&denied),"Stale/future/overflow revisions must fail safely")?;
            require(effects(&app).await?==original_effects,"Rejected withdrawal must have no clinical or audit effects")?;
        }
        let withdraw = app.client.patch(format!("{endpoint}/{id}")).bearer_auth(&token)
            .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
            .send().await.map_err(|_|DbErr::Custom("Own withdrawal failed before response".to_owned()))?;
        require(withdraw.status().as_u16()==200 && no_store(&withdraw),"Guardian must be able to withdraw after last departure")?;
        let withdrawn: Value = withdraw.json().await.map_err(|_|DbErr::Custom("Withdrawal must return JSON".to_owned()))?;
        require(withdrawn["data"]["revision"]=="2" && withdrawn["data"]["withdrawn_at"].as_str().is_some(),"Withdrawal must return server-timed terminal revision2")?;
        let audit = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.versions WHERE item_type='GuardianDeclaration' AND item_id=$1 AND event='withdraw' AND household_id IS NULL AND whodunnit='77001' AND audit_context->>'actor_account_id'='71001') AS versions,(SELECT count(*) FROM public.audit_ledger_entries l JOIN public.versions v ON v.id=l.source_id WHERE l.source_table='versions' AND v.item_type='GuardianDeclaration' AND v.item_id=$1 AND v.event='withdraw' AND l.household_id IS NULL) AS ledger,(SELECT count(*) FROM public.audit_export_deliveries d JOIN public.audit_ledger_entries l ON l.id=d.audit_ledger_entry_id JOIN public.versions v ON v.id=l.source_id WHERE l.source_table='versions' AND v.item_type='GuardianDeclaration' AND v.item_id=$1 AND v.event='withdraw') AS deliveries",
            [id.into()])).await?.ok_or_else(||DbErr::Custom("Withdrawal audit facts unavailable".to_owned()))?;
        require(audit.try_get::<i64>("","versions")?==1 && audit.try_get::<i64>("","ledger")?==1
            && audit.try_get::<i64>("","deliveries")?==1,"Withdrawal must commit exactly one attributable global audit and delivery")?;
        let committed = effects(&app).await?;
        let replay = app.client.patch(format!("{endpoint}/{id}")).bearer_auth(&token)
            .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
            .send().await.map_err(|_|DbErr::Custom("Terminal withdrawal replay failed".to_owned()))?;
        require(replay.status().as_u16()==200 && no_store(&replay) && effects(&app).await?==committed,
            "Terminal withdrawal replay must preserve the one committed authority event")?;
        require(identity(&app).await?==original_identity,"Global withdrawal must not restore household membership or alter credentials")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_browser_creation_requires_unchecked_declaration_and_supports_own_withdrawal()
-> Result<(), DbErr> {
    let mut app = Application::new().await;
    let result = async {
        app.client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .build().map_err(|_|DbErr::Custom("Owned browser client unavailable".to_owned()))?;
        let cookie = sign_in(&app,71001,"persistence@example.test").await;
        let path = "/households/persistence-fixture/people/new";
        let (cookies,body) = form_page(&app,&cookie,path).await;
        let checkbox = body.split("<input").find(|part|part.split('>').next().is_some_and(|input|input.contains("name=\"initial_guardian_declaration\"")))
            .and_then(|part|part.split('>').next()).ok_or_else(||DbErr::Custom("New dependent form must offer an explicit initial guardian declaration checkbox".to_owned()))?;
        require(checkbox.contains("required") && !checkbox.contains("checked"),"Initial guardian declaration must start unchecked and be required")?;
        require(body.contains("for=\"initial_guardian_declaration\""),"Declaration checkbox must have an accessible label")?;
        let csrf = hidden_field(&body,"authenticity_token");
        require(!csrf.is_empty(),"Existing creation form must provide real CSRF protection")?;
        let before = effects(&app).await?;
        let form = [("name","Synthetic browser declared dependent"),("date_of_birth","2020-01-01"),
            ("person_type","minor"),("has_capacity","false"),("authenticity_token",csrf.as_str())];
        let missing = post_form(&app,&cookies,"/households/persistence-fixture/people",&form).await;
        let status = missing.status().as_u16();
        let cookies = response_cookies(&cookies,&missing);
        let error_body = missing.text().await.map_err(|_|DbErr::Custom("Declaration error page unavailable".to_owned()))?;
        require(status==422 && effects(&app).await?==before,"Missing browser declaration must rerender422 without creation effects")?;
        let csrf = hidden_field(&error_body,"authenticity_token");
        let valid = post_form(&app,&cookies,"/households/persistence-fixture/people",&[
            ("name","Synthetic browser declared dependent"),("date_of_birth","2020-01-01"),
            ("person_type","minor"),("has_capacity","false"),("initial_guardian_declaration","true"),
            ("authenticity_token",csrf.as_str()),
        ]).await;
        require(valid.status().as_u16()==303,"Explicit browser declaration must create and redirect normally")?;
        let (cookies,page) = form_page(&app,&cookies,"/profile/guardian-declarations").await;
        require(page.contains("Synthetic browser declared dependent"),"Own declaration page must show the captured dependent")?;
        let id = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT id FROM public.guardian_declarations WHERE subject_name_at_declaration='Synthetic browser declared dependent' AND guardian_person_id=73001"))
            .await?.ok_or_else(||DbErr::Custom("Browser declaration ID unavailable".to_owned()))?.try_get::<i64>("","id")?;
        let route = format!("/profile/guardian-declarations/{id}/withdraw");
        let csrf = hidden_field(&page,"authenticity_token");
        let before = effects(&app).await?;
        let denied = post_form(&app,&cookies,&route,&[("revision","1"),("withdrawn","true")]).await;
        require(denied.status().as_u16()==403 && effects(&app).await?==before,"Browser withdrawal must enforce CSRF without authority effects")?;
        let withdrawn = post_form(&app,&cookies,&route,&[("authenticity_token",csrf.as_str()),("revision","1"),("withdrawn","true")]).await;
        require(withdrawn.status().as_u16()==303,"Guardian's valid browser withdrawal must redirect after commit")?;
        let (_,refreshed) = form_page(&app,&cookies,"/profile/guardian-declarations").await;
        require(refreshed.contains("Withdrawn") && !refreshed.contains(&format!("action=\"{route}\"")),"Refresh must show final withdrawal without offering restoration")?;
        Ok(())
    }.await;
    app.close().await;
    result
}
