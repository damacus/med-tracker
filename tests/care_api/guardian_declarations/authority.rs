use super::*;

async fn capabilities(app: &Application, token: &str) -> Result<Value, DbErr> {
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/72001/ui_capabilities",
            app.origin
        ))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| DbErr::Custom("Guardian capability request failed before response".into()))?;
    require(
        response.status().as_u16() == 200,
        "Guardian capability prerequisite must succeed",
    )?;
    response
        .json()
        .await
        .map_err(|_| DbErr::Custom("Guardian capability response was not JSON".into()))
}

async fn schedule(
    app: &Application,
    token: &str,
    person_id: i64,
) -> Result<reqwest::Response, DbErr> {
    app.client.post(format!("{}/api/v1/households/72001/schedules",app.origin)).bearer_auth(token)
        .json(&json!({"schedule":{"person_id":person_id.to_string(),"medication_id":"80001","dose_amount":"2","dose_unit":"tablet","start_date":"2026-01-01","end_date":"2027-01-01","schedule_type":"prn","schedule_config":{},"dose_cycle":"daily"}}))
        .send().await.map_err(|_|DbErr::Custom("Guardian plan request failed before response".into()))
}

#[tokio::test]
async fn declared_guardian_manage_is_live_for_direct_and_aggregate_access() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let mut input=dependent("Synthetic live guardian authority","minor");
        input["initial_guardian_declaration"]=json!(true);
        let response=create(&app,&token,&input,"synthetic-live-guardian-authority").await?;
        require(response.status().as_u16()==201,"Guardian authority prerequisite must create a declared dependent")?;
        let body:Value=response.json().await.map_err(|_|DbErr::Custom("Dependent creation response was not JSON".into()))?;
        let person_id=body["data"]["id"].as_i64().ok_or_else(||DbErr::Custom("Created dependent ID missing".into()))?;
        let manages=capabilities(&app,&token).await?;
        require(manages["data"]["people"]["manage_ids"].as_array().is_some_and(|ids|ids.contains(&json!(person_id))),"A current initial guardian must appear in aggregate Manage without a legacy grant")?;
        let response=schedule(&app,&token,person_id).await?;
        require(response.status().as_u16()==201,&format!("Current guardian must create a dependent plan, received{}",response.status().as_u16()))?;
        let unrelated=schedule(&app,&token,73002).await?;
        require(unrelated.status().as_u16()==403,"Household manager must not gain unrelated subject plan authority")?;
        let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT id,(SELECT count(*) FROM public.person_access_grants WHERE person_id=$1) AS grants FROM public.guardian_declarations WHERE subject_person_id=$1",
            [person_id.into()])).await?.ok_or_else(||DbErr::Custom("Created declaration missing".into()))?;
        require(row.try_get::<i64>("","grants")?==0,"Declared guardian authority must not mint legacy grants")?;
        let declaration_id:i64=row.try_get("","id")?;
        let withdrawn=app.client.patch(format!("{}/api/v1/auth/guardian-declarations/{declaration_id}",app.origin))
            .bearer_auth(&token).json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}})).send().await
            .map_err(|_|DbErr::Custom("Guardian withdrawal failed before response".into()))?;
        require(withdrawn.status().as_u16()==200,"Personal guardian withdrawal must succeed")?;
        let before=effects(&app).await?;
        let response=schedule(&app,&token,person_id).await?;
        require(response.status().as_u16()==403,"Withdrawn declaration must no longer authorise dependent plan writes")?;
        let manages=capabilities(&app,&token).await?;
        require(!manages["data"]["people"]["manage_ids"].as_array().is_some_and(|ids|ids.contains(&json!(person_id))),"Withdrawn declaration must disappear from aggregate Manage")?;
        require(effects(&app).await?==before,"Denied plan write must preserve clinical and authority state")?;
        Ok(())
    }.await;
    app.close().await;
    result
}
