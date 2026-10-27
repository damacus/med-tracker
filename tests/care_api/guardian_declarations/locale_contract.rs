use super::super::{
    contract,
    platform::{hidden_field, response_cookies, sign_in},
};
use super::{Application, create, dependent, effects, require, withdrawal::no_store};
use sea_orm::{
    ConnectionTrait, DbErr,
    sea_query::{Expr, ExprTrait, Query},
};
use serde_json::{Value, json};

#[tokio::test]
async fn guardian_creation_form_and_validation_keep_selected_declaration_language()
-> Result<(), DbErr> {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let slug = &app.fixture.household.slug;
    let result=async {
        let cookie=sign_in(&app,app.fixture.account.id,&app.fixture.account.email).await;
        let public=app.context.config.server.full_url();
        let public_url=url::Url::parse(&public).map_err(|_|DbErr::Custom("Maintained browser origin must be valid".into()))?;
        let host=public_url.host_str().ok_or_else(||DbErr::Custom("Maintained browser host missing".into()))?;
        let authority=public_url.port().map_or_else(||host.to_owned(),|port|format!("{host}:{port}"));
        let before=effects(&app).await?;
        for (language,source) in [
            ("en",include_str!("../../../assets/reports/locales/en.yml")),
            ("es",include_str!("../../../assets/reports/locales/es.yml")),
            ("pt",include_str!("../../../assets/reports/locales/pt.yml")),
            ("ga",include_str!("../../../assets/reports/locales/ga.yml")),
            ("cy",include_str!("../../../assets/reports/locales/cy.yml")),
        ] {
            let labels:Value=serde_yaml_ng::from_str(source)
                .map_err(|_|DbErr::Custom("Maintained declaration translations must parse".into()))?;
            let response=app.client.get(format!("{}/households/{slug}/people/new",app.origin))
                .header("cookie",&cookie).header("accept-language",language).send().await
                .map_err(|_|DbErr::Custom("Selected-language creation form failed before response".into()))?;
            require(response.status().as_u16()==200,"Genuine guardian must reach the existing creation form")?;
            let cookies=response_cookies(&cookie,&response);
            let body=response.text().await.map_err(|_|DbErr::Custom("Selected-language creation form unavailable".into()))?;
            declaration_labels(&body,&labels[language]["guardian_declarations"],language,false)?;
            let csrf=hidden_field(&body,"authenticity_token");
            require(!csrf.is_empty(),"Selected-language form must retain maintained CSRF protection")?;
            let encoded=serde_urlencoded::to_string([
                ("name","Synthetic localized declared dependent"),("date_of_birth","2020-01-01"),
                ("person_type","minor"),("has_capacity","false"),("authenticity_token",csrf.as_str())
            ]).map_err(|_|DbErr::Custom("Localized validation form encoding failed".into()))?;
            let invalid=app.client.post(format!("{}/households/{slug}/people",app.origin))
                .header("cookie",&cookies).header("accept-language",language)
                .header("host",&authority).header("origin",public.trim_end_matches('/'))
                .header("content-type","application/x-www-form-urlencoded").body(encoded).send().await
                .map_err(|_|DbErr::Custom("Selected-language validation request failed before response".into()))?;
            require(invalid.status().as_u16()==422,"Missing affirmation must rerender its normal validation error")?;
            let error=invalid.text().await.map_err(|_|DbErr::Custom("Localized validation response unavailable".into()))?;
            declaration_labels(&error,&labels[language]["guardian_declarations"],language,false)?;
            require(error.contains("Synthetic localized declared dependent") && !hidden_field(&error,"authenticity_token").is_empty(),
                "Validation must preserve safe draft values and usable CSRF protection")?;
            require(effects(&app).await?==before,"Locale selection and rejected affirmation must have zero clinical/authority effects")?;
        }
        Ok(())
    }.await;
    app.close().await;
    result
}

fn declaration_labels(
    html: &str,
    labels: &Value,
    language: &str,
    checked: bool,
) -> Result<(), DbErr> {
    for key in ["statement", "statement_hint"] {
        let label = labels[key].as_str().ok_or_else(|| {
            DbErr::Custom("Expected maintained declaration translation key".into())
        })?;
        require(
            html.contains(label),
            &format!("Declaration {key} must use selected language {language}"),
        )?;
    }
    require(
        html.contains(&format!("lang=\"{language}\""))
            || (language == "en" && html.contains("lang=\"en-GB\"")),
        "The form document must retain the selected language",
    )?;
    let checkbox = html
        .split("<input")
        .filter_map(|part| part.split('>').next())
        .find(|input| input.contains("name=\"initial_guardian_declaration\""))
        .ok_or_else(|| DbErr::Custom("Declaration affirmation checkbox missing".into()))?;
    require(
        checkbox.contains("required")
            && checkbox.contains("checked") == checked
            && html.contains("for=\"initial_guardian_declaration\""),
        "Declaration checkbox must remain required, labelled and explicitly unchecked",
    )
}

fn documented_value(doc: &Value, schema: &Value, value: &Value, label: &str) -> Result<(), DbErr> {
    require(
        schema.is_object(),
        &format!("OpenAPI must document the existing {label} schema"),
    )?;
    std::panic::catch_unwind(|| contract::assert_value(doc, schema, value, label))
        .map_err(|_| DbErr::Custom(format!("Existing {label} wire shape must match OpenAPI")))
}

fn resolved<'a>(doc: &'a Value, node: &'a Value) -> Result<&'a Value, DbErr> {
    if let Some(reference) = node["$ref"].as_str() {
        reference
            .strip_prefix('#')
            .and_then(|path| doc.pointer(path))
            .ok_or_else(|| {
                DbErr::Custom("Existing declaration contract reference must resolve".into())
            })
    } else {
        Ok(node)
    }
}

fn response_contract<'a>(
    doc: &'a Value,
    path: &str,
    method: &str,
    status: &str,
) -> Result<&'a Value, DbErr> {
    require(
        doc["paths"][path][method]["operationId"]
            .as_str()
            .is_some_and(|id| !id.is_empty()),
        "OpenAPI must catalogue the already implemented declaration operation",
    )?;
    resolved(doc, &doc["paths"][path][method]["responses"][status])
}

#[tokio::test]
async fn guardian_existing_create_list_and_withdraw_wire_match_openapi() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let mut input=dependent("Synthetic documented initial declaration","minor");
        input["initial_guardian_declaration"]=json!(true);
        let created=create(&app,&token,&input,"synthetic-guardian-locale-contract").await?;
        require(created.status().as_u16()==201,"Contract proof must first reach genuine declared dependent creation")?;
        let created:Value=created.json().await.map_err(|_|DbErr::Custom("Created dependent contract response must be JSON".into()))?;
        let doc=contract::contract();
        let create_path="/households/{household_id}/people";
        let request=&doc["paths"][create_path]["post"]["requestBody"]["content"]["application/json"]["schema"];
        documented_value(doc,request,&json!({"person":input}),"declared dependent creation request")?;
        let request=resolved(doc,request)?;
        let attributes=resolved(doc,&request["properties"]["person"])?;
        let affirmation=resolved(doc,&attributes["properties"]["initial_guardian_declaration"])?;
        require(attributes["required"].as_array().is_some_and(|fields|fields.contains(&json!("initial_guardian_declaration")))
            && affirmation["type"]=="boolean" && affirmation["enum"]==json!([true]),
            "Creation contract must require exact true attestation without accepting an arbitrary guardian ID")?;
        require(doc["components"]["schemas"]["PersonAttributes"]["properties"].get("initial_guardian_declaration").is_none(),
            "Ordinary Person updates must not advertise a new declaration or identity-claim command")?;
        let create_response=response_contract(doc,create_path,"post","201")?;
        documented_value(doc,&create_response["content"]["application/json"]["schema"],&created,"declared dependent creation response")?;
        let subject=created["data"]["id"].as_i64().ok_or_else(||DbErr::Custom("Created dependent ID missing".into()))?;
        let query = Query::select().column("id").from("guardian_declarations")
            .and_where(Expr::col("subject_person_id").eq(subject))
            .and_where(Expr::col("guardian_person_id").eq(app.fixture.adult.id)).to_owned();
        let id = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?.ok_or_else(||DbErr::Custom("Genuinely created declaration ID missing".into()))?.try_get::<i64>("","id")?;
        let listed=app.client.get(format!("{}/api/v1/auth/guardian-declarations?limit=1",app.origin)).bearer_auth(&token).send().await
            .map_err(|_|DbErr::Custom("Documented own declaration list failed before response".into()))?;
        require(listed.status().as_u16()==200 && no_store(&listed),"Existing own declaration list must remain200 and no-store")?;
        let listed:Value=listed.json().await.map_err(|_|DbErr::Custom("Own declaration list must be JSON".into()))?;
        let list_response=response_contract(doc,"/auth/guardian-declarations","get","200")?;
        require(list_response["headers"]["Cache-Control"].is_object(),"Own declaration response must document its no-store header")?;
        documented_value(doc,&list_response["content"]["application/json"]["schema"],&listed,"own declaration list response")?;
        let path="/auth/guardian-declarations/{id}";
        let withdrawal=json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}});
        documented_value(doc,&doc["paths"][path]["patch"]["requestBody"]["content"]["application/json"]["schema"],&withdrawal,"own declaration withdrawal request")?;
        let withdrawn=app.client.patch(format!("{}/api/v1/auth/guardian-declarations/{id}",app.origin)).bearer_auth(&token)
            .json(&withdrawal).send().await.map_err(|_|DbErr::Custom("Documented withdrawal failed before response".into()))?;
        require(withdrawn.status().as_u16()==200 && no_store(&withdrawn),"Existing own withdrawal must remain200 and no-store")?;
        let withdrawn:Value=withdrawn.json().await.map_err(|_|DbErr::Custom("Own withdrawal response must be JSON".into()))?;
        let withdraw_response=response_contract(doc,path,"patch","200")?;
        require(withdraw_response["headers"]["Cache-Control"].is_object(),"Withdrawal response must document its no-store header")?;
        documented_value(doc,&withdraw_response["content"]["application/json"]["schema"],&withdrawn,"own declaration withdrawal response")?;
        Ok(())
    }.await;
    app.close().await;
    result
}
