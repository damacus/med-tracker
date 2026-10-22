use super::*;

#[tokio::test]
async fn legacy_manage_grant_cannot_edit_other_care_or_expose_manage_controls() -> Result<(), DbErr>
{
    use med_tracker::models::entities::{
        grant, membership, schedule as schedule_record, security_audit_event,
    };
    use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};

    let app = Application::new().await;
    let result = async {
        let now = chrono::Utc::now().naive_utc();
        grant::Entity::insert(grant::ActiveModel {
            household_id: Set(72001),
            household_membership_id: Set(74001),
            person_id: Set(73002),
            access_level: Set("manage".into()),
            relationship_type: Set("parent".into()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }).exec(&app.fixture.admin).await?;
        let token = app.token().await;
        let created = schedule(&app, &token, 73001).await?;
        require(created.status().as_u16() == 201, "Self-managed schedule prerequisite must succeed")?;
        schedule_record::Entity::update_many()
            .col_expr(schedule_record::Column::PersonId, sea_orm::sea_query::Expr::value(73002))
            .exec(&app.fixture.admin).await?;
        let original_grants = grant::Entity::find().all(&app.fixture.admin).await?;
        let original_plans = schedule_record::Entity::find().all(&app.fixture.admin).await?;
        let original_stock = app.fixture.effect().await;
        for role in ["owner", "administrator", "member"] {
            membership::Entity::update_many()
                .col_expr(membership::Column::Role, sea_orm::sea_query::Expr::value(role))
                .filter(membership::Column::Id.eq(74001))
                .exec(&app.fixture.admin).await?;
            let manages = capabilities(&app, &token).await?;
            require(manages["data"]["people"]["create"] == (role != "member"),
                "Person creation follows household management, never legacy grants")?;
            require(!manages["data"]["people"]["manage_ids"].as_array().is_some_and(|ids| ids.contains(&json!(73002))),
                "Legacy grant must not expose aggregate Manage for another person")?;
            let read = app.client.get(format!("{}/api/v1/households/72001/schedules", app.origin))
                .bearer_auth(&token).send().await.map_err(|_| DbErr::Custom("Plan read failed".into()))?;
            require(read.status().as_u16() == 200, "Household participants retain plan reads")?;
            let body:Value = read.json().await.map_err(|_| DbErr::Custom("Plan response was not JSON".into()))?;
            require(body["data"].as_array().is_some_and(|plans| !plans.is_empty() && plans.iter().all(|plan| plan["can_manage"] == false && plan["can_record"] == true)),
                "Legacy grant must not expose plan editing while household recording remains available")?;
            let before = effects(&app).await?;
            let denied = schedule(&app, &token, 73002).await?;
            require(denied.status().as_u16() == 403, "Legacy-only grant must not authorise another person's treatment")?;
            let request_id = denied.headers().get("x-request-id").and_then(|value| value.to_str().ok())
                .ok_or_else(|| DbErr::Custom("Denied treatment request ID missing".into()))?;
            let audit = security_audit_event::Entity::find()
                .filter(security_audit_event::Column::RequestId.eq(request_id))
                .filter(security_audit_event::Column::EventType.eq("api.request"))
                .one(&app.fixture.admin).await?
                .ok_or_else(|| DbErr::Custom("Denied treatment audit missing".into()))?;
            require(audit.actor_account_id == Some(71001) && audit.actor_membership_id == Some(74001)
                && audit.metadata["controller"] == "api/v1/schedules" && audit.metadata["status"] == 403,
                "Denied treatment must retain attributable failure audit")?;
            require(effects(&app).await? == before, "Denied treatment must preserve clinical records, versions, sync and outbox")?;
            require(app.fixture.effect().await == original_stock,
                "Denied treatment must preserve administration history and stock")?;
            if role == "member" {
                let mut input = dependent("Synthetic legacy grant denied child", "minor");
                input["initial_guardian_declaration"] = json!(true);
                let denied = create(&app, &token, &input, "synthetic-legacy-create-denied").await?;
                require(denied.status().as_u16() == 403 && effects(&app).await? == before,
                    "Legacy-granted member must not create a dependent or change clinical state")?;
            }
        }
        require(grant::Entity::find().all(&app.fixture.admin).await? == original_grants,
            "Removing authority must preserve historical grants")?;
        require(schedule_record::Entity::find().all(&app.fixture.admin).await? == original_plans,
            "Denied treatment must preserve existing plans")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

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
        let plan:Value=response.json().await.map_err(|_|DbErr::Custom("Guardian plan response was not JSON".into()))?;
        require(plan["data"]["can_manage"]==true,"Declaration-only guardian must see the same Manage permission as the write boundary")?;
        let assigned=app.client.post(format!("{}/api/v1/households/72001/person_medications",app.origin))
            .bearer_auth(&token).json(&json!({"person_medication":{"person_id":person_id.to_string(),"medication_id":"80001","administration_kind":"routine"}}))
            .send().await.map_err(|_|DbErr::Custom("Guardian assignment request failed".into()))?;
        require(assigned.status().as_u16()==201,"Declaration-only guardian must assign medication")?;
        let assignment:Value=assigned.json().await.map_err(|_|DbErr::Custom("Guardian assignment response was not JSON".into()))?;
        require(assignment["data"]["can_manage"]==true,"Assignment Manage projection must include declaration-only guardians")?;
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
        for (collection, id) in [("schedules", &plan["data"]["id"]), ("person_medications", &assignment["data"]["id"])] {
            let id=id.as_i64().ok_or_else(||DbErr::Custom("Created treatment ID missing".into()))?;
            let read=app.client.get(format!("{}/api/v1/households/72001/{collection}/{id}",app.origin))
                .bearer_auth(&token).send().await.map_err(|_|DbErr::Custom("Withdrawn guardian treatment read failed".into()))?;
            require(read.status().as_u16()==200,"Withdrawal retains participating household care reads")?;
            let body:Value=read.json().await.map_err(|_|DbErr::Custom("Withdrawn guardian treatment response was not JSON".into()))?;
            require(body["data"]["can_manage"]==false && body["data"]["can_record"]==true,
                "Withdrawal removes existing treatment edit controls while retaining household recording")?;
        }
        require(effects(&app).await?==before,"Denied plan write must preserve clinical and authority state")?;
        Ok(())
    }.await;
    app.close().await;
    result
}
