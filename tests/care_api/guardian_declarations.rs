use super::Application;
use med_tracker::models::_entities::{household_memberships, person_access_grants, versions};
use med_tracker::models::care::portable_crypto;
use sea_orm::sea_query::{Expr, ExprTrait, Func, Query, SelectStatement};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, Set,
};
use sea_orm::{ConnectionTrait, DbErr};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[path = "guardian_declarations/withdrawal.rs"]
mod withdrawal;

#[path = "guardian_declarations/authority.rs"]
mod authority;

#[path = "guardian_declarations/security.rs"]
mod security;

#[path = "guardian_declarations/live_authority.rs"]
mod live_authority;

#[path = "guardian_declarations/storage_boundary.rs"]
mod storage_boundary;

#[path = "guardian_declarations/locale_contract.rs"]
mod locale_contract;

#[path = "guardian_declarations/recovery.rs"]
mod recovery;

#[path = "guardian_declarations/recovery_browser.rs"]
mod recovery_browser;

fn require(condition: bool, message: &str) -> Result<(), DbErr> {
    if condition {
        Ok(())
    } else {
        Err(DbErr::Custom(message.to_owned()))
    }
}

fn participants(app: &Application) -> SelectStatement {
    let memberships = Query::select()
        .column("person_id")
        .from("household_memberships")
        .and_where(Expr::col("household_id").eq(app.fixture.household.id))
        .to_owned();
    let retained = |column| {
        Query::select()
            .column(column)
            .from("guardian_declarations")
            .and_where(Expr::col("origin_household_id").eq(app.fixture.household.id))
            .to_owned()
    };
    Query::select()
        .column("id")
        .from("people")
        .and_where(
            Expr::col("id")
                .is_in([
                    app.fixture.adult.id,
                    app.fixture.minor.id,
                    app.fixture.dependent.id,
                ])
                .or(Expr::col("id").in_subquery(memberships))
                .or(Expr::col("id").in_subquery(retained("subject_person_id")))
                .or(Expr::col("id").in_subquery(retained("guardian_person_id"))),
        )
        .to_owned()
}

fn scenario_declarations(app: &Application) -> SelectStatement {
    Query::select()
        .column("id")
        .from("guardian_declarations")
        .and_where(
            Expr::col("declared_by_account_id")
                .eq(app.fixture.account.id)
                .or(Expr::col("subject_person_id").in_subquery(participants(app)))
                .or(Expr::col("guardian_person_id").in_subquery(participants(app))),
        )
        .to_owned()
}

fn scenario_version_ids(app: &Application) -> SelectStatement {
    Query::select()
        .column("id")
        .from("versions")
        .and_where(
            Expr::col("household_id")
                .eq(app.fixture.household.id)
                .or(Expr::col("item_type")
                    .eq("GuardianDeclaration")
                    .and(Expr::col("item_id").in_subquery(scenario_declarations(app)))),
        )
        .to_owned()
}

fn scenario_version_ledger_ids(app: &Application) -> SelectStatement {
    Query::select()
        .column("id")
        .from("audit_ledger_entries")
        .and_where(Expr::col("source_table").eq("versions"))
        .and_where(Expr::col("source_id").in_subquery(scenario_version_ids(app)))
        .to_owned()
}

fn accounts(app: &Application) -> SelectStatement {
    Query::select()
        .column("id")
        .from("accounts")
        .and_where(Expr::col("person_id").in_subquery(participants(app)))
        .to_owned()
}

async fn count(app: &Application, query: SelectStatement) -> Result<i64, DbErr> {
    app.fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await?
        .ok_or_else(|| DbErr::Custom("Expected scoped effect count".into()))?
        .try_get("", "count")
}

async fn effects(app: &Application) -> Result<Value, DbErr> {
    let mut state = serde_json::Map::new();
    for (name, table) in [
        ("memberships", "household_memberships"),
        ("grants", "person_access_grants"),
        ("carers", "carer_relationships"),
        ("locations", "locations"),
        ("location_links", "location_memberships"),
        ("changes", "api_change_events"),
        ("notifications", "notification_events"),
        ("deliveries", "notification_deliveries"),
    ] {
        let query = Query::select()
            .expr_as(Expr::col("id").count(), "count")
            .from(table)
            .and_where(Expr::col("household_id").eq(app.fixture.household.id))
            .to_owned();
        state.insert(name.into(), json!(count(app, query).await?));
    }
    // Version provenance excludes permitted HTTP-attempt security audit records.
    for (name, table, column, ids) in [
        ("versions", "versions", "id", scenario_version_ids(app)),
        (
            "version_ledger",
            "audit_ledger_entries",
            "id",
            scenario_version_ledger_ids(app),
        ),
        (
            "audit_deliveries",
            "audit_export_deliveries",
            "audit_ledger_entry_id",
            scenario_version_ledger_ids(app),
        ),
    ] {
        let query = Query::select()
            .expr_as(Expr::col("id").count(), "count")
            .from(table)
            .and_where(Expr::col(column).in_subquery(ids))
            .to_owned();
        state.insert(name.into(), json!(count(app, query).await?));
    }
    let people = Query::select()
        .expr_as(Expr::col("id").count(), "count")
        .from("people")
        .and_where(Expr::col("id").in_subquery(participants(app)))
        .to_owned();
    state.insert("people".into(), json!(count(app, people).await?));
    let jobs = Query::select()
        .expr_as(Expr::col("id").count(), "count")
        .from("pg_loco_queue")
        .and_where(
            Func::cust("jsonb_extract_path_text")
                .args([Expr::col("task_data"), Expr::val("to")])
                .eq(&app.fixture.account.email),
        )
        .to_owned();
    state.insert("jobs".into(), json!(count(app, jobs).await?));
    let memberships = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(app.fixture.household.id))
        .all(&app.fixture.admin)
        .await?;
    let mut permissions = memberships
        .iter()
        .map(|row| (row.id, row.permissions_version))
        .collect::<Vec<_>>();
    permissions.sort_unstable();
    state.insert("permissions".into(), json!(permissions));
    Ok(Value::Object(state))
}

async fn identity(app: &Application) -> Result<Value, DbErr> {
    let mut state = serde_json::Map::new();
    for (name, table, column, principals) in [
        ("accounts", "accounts", "id", accounts(app)),
        ("sessions", "api_sessions", "account_id", accounts(app)),
        ("keys", "identity_api_keys", "reference_id", accounts(app)),
        ("grants", "oauth_grants", "account_id", accounts(app)),
    ] {
        let query = Query::select()
            .expr_as(Expr::col("id").count(), "count")
            .from(table)
            .and_where(Expr::col(column).in_subquery(principals))
            .to_owned();
        state.insert(name.into(), json!(count(app, query).await?));
    }
    for (name, table, column) in [
        ("passwords", "auth_credentials", "user_id"),
        ("passkeys", "identity_passkeys", "account_id"),
        ("providers", "auth_credentials", "user_id"),
    ] {
        let query = Query::select()
            .expr_as(Func::cust("to_jsonb").arg(Expr::col("r")), "record")
            .from_as(table, "r")
            .and_where(Expr::col(("r", column)).in_subquery(accounts(app)))
            .order_by(("r", "id"), sea_orm::Order::Asc)
            .to_owned();
        let records = app
            .fixture
            .admin
            .query_all_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?
            .into_iter()
            .map(|row| row.try_get::<Value>("", "record"))
            .collect::<Result<Vec<_>, _>>()?;
        let encoded = serde_json::to_vec(&records)
            .map_err(|_| DbErr::Custom("Scoped identity snapshot failed".into()))?;
        state.insert(name.into(), json!(hex::encode(Sha256::digest(encoded))));
    }
    let query = Query::select()
        .columns(["id", "person_id"])
        .from("accounts")
        .and_where(Expr::col("id").in_subquery(accounts(app)))
        .order_by("id", sea_orm::Order::Asc)
        .to_owned();
    let bindings = app
        .fixture
        .admin
        .query_all_raw(app.fixture.admin.get_database_backend().build(&query))
        .await?
        .into_iter()
        .map(|row| {
            Ok((
                row.try_get::<i64>("", "id")?,
                row.try_get::<i64>("", "person_id")?,
            ))
        })
        .collect::<Result<Vec<_>, DbErr>>()?;
    state.insert("bindings".into(), json!(bindings));
    Ok(Value::Object(state))
}

async fn declared_effects(app: &Application, person: i64) -> Result<(), DbErr> {
    let query = Query::select()
        .expr_as(Expr::col("id").count(), "count")
        .from("guardian_declarations")
        .and_where(Expr::col("subject_person_id").eq(person))
        .and_where(Expr::col("guardian_person_id").eq(app.fixture.adult.id))
        .and_where(Expr::col("declared_by_account_id").eq(app.fixture.account.id))
        .and_where(Expr::col("statement_version").eq(1))
        .and_where(Expr::col("withdrawn_at").is_null())
        .to_owned();
    let declarations = count(app, query).await?;
    let participations = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(household_memberships::Column::PersonId.eq(person))
        .filter(household_memberships::Column::Status.eq("active"))
        .filter(household_memberships::Column::RevokedAt.is_null())
        .count(&app.fixture.admin)
        .await?;
    let grants = person_access_grants::Entity::find()
        .filter(person_access_grants::Column::PersonId.eq(person))
        .count(&app.fixture.admin)
        .await?;
    let person_versions = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("Person"))
        .filter(versions::Column::ItemId.eq(person))
        .filter(versions::Column::Event.eq("create"))
        .count(&app.fixture.admin)
        .await?;
    let declarations_for_person = Query::select()
        .column("id")
        .from("guardian_declarations")
        .and_where(Expr::col("subject_person_id").eq(person))
        .to_owned();
    let declaration_versions = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("GuardianDeclaration"))
        .filter(Expr::col(versions::Column::ItemId).in_subquery(declarations_for_person))
        .filter(versions::Column::Event.eq("create"))
        .count(&app.fixture.admin)
        .await?;
    require(
        declarations == 1
            && participations == 1
            && grants == 0
            && person_versions == 1
            && declaration_versions == 1,
        "Creation must commit one explicit declaration/participation/provenance and no separate Manage grant",
    )
}

fn dependent(name: &str, kind: &str) -> Value {
    json!({"name":name,"date_of_birth":if kind=="minor" {"2020-01-01"} else {"1980-01-01"},
        "person_type":kind,"has_capacity":false})
}

async fn create(
    app: &Application,
    token: &str,
    person: &Value,
    key: &str,
) -> Result<reqwest::Response, DbErr> {
    app.client
        .post(format!(
            "{}/api/v1/households/{}/people",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(token)
        .header(
            "idempotency-key",
            format!("{}-{key}", app.fixture.client_uuid),
        )
        .json(&json!({"person":person}))
        .send()
        .await
        .map_err(|_| DbErr::Custom("Dependent creation request failed before response".to_owned()))
}

#[tokio::test]
async fn guardian_declarations_manual_create_requires_explicit_attestation() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let token = app.token().await;
        let before = effects(&app).await?;
        let original_identity = identity(&app).await?;
        for (label, attestation) in [
            ("missing", None),
            ("false", Some(json!(false))),
            ("null", Some(Value::Null)),
            ("string", Some(json!("true"))),
            ("number", Some(json!(1))),
            ("object", Some(json!({"accepted":true}))),
        ] {
            let mut person = dependent("Synthetic explicit declaration child", "minor");
            if let Some(value) = attestation {
                person.as_object_mut().ok_or_else(|| DbErr::Custom("Expected synthetic Person input".to_owned()))?
                    .insert("initial_guardian_declaration".to_owned(), value);
            }
            let response = create(&app, &token, &person, &format!("synthetic-declaration-{label}")).await?;
            require(response.status().as_u16()==422,
                &format!("{label} declaration must return422, received{}",response.status().as_u16()))?;
            require(effects(&app).await?==before && identity(&app).await?==original_identity,
                "Invalid declaration must preserve Person, authority, credentials, versions and queue state")?;
        }
        for (case, person) in [
            json!({"name":"Synthetic capable adult","date_of_birth":"1980-01-01","person_type":"adult","has_capacity":true,"initial_guardian_declaration":true}),
            json!({"name":"Synthetic disguised dependent","date_of_birth":"1980-01-01","person_type":"adult","has_capacity":false,"initial_guardian_declaration":true}),
            json!({"name":"Synthetic forged guardian","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true,"guardian_person_id":app.fixture.minor.id}),
            json!({"id":app.fixture.minor.id,"name":"Synthetic existing subject","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}),
        ].into_iter().enumerate() {
            let response = create(&app,&token,&person,&format!("synthetic-invalid-declaration-identity-{case}")).await?;
            require(response.status().as_u16()==422,
                "Capable adults, disguised dependent kinds and supplied authority identities must be rejected")?;
            require(effects(&app).await?==before && identity(&app).await?==original_identity,
                "Rejected identity selection must have no clinical or authority effects")?;
        }
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_declarations_manual_create_is_atomic_and_replay_safe() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let token = app.token().await;
        let original_identity = identity(&app).await?;
        for kind in ["minor","dependent_adult"] {
            let mut person = dependent(&format!("Synthetic declared {kind}"),kind);
            person.as_object_mut().ok_or_else(||DbErr::Custom("Expected synthetic declaration input".to_owned()))?
                .insert("initial_guardian_declaration".to_owned(),json!(true));
            let key = format!("synthetic-declared-{kind}");
            let response=create(&app,&token,&person,&key).await?;
            require(response.status().as_u16()==201,
                &format!("Explicit new {kind} declaration must create201, received{}",response.status().as_u16()))?;
            let body:Value=response.json().await.map_err(|_|DbErr::Custom("Expected created dependent response".to_owned()))?;
            let id=body.get("data").and_then(|data|data.get("id"))
                .and_then(|value|value.as_i64().or_else(||value.as_str().and_then(|text|text.parse().ok())))
                .ok_or_else(||DbErr::Custom("Expected created dependent ID".to_owned()))?;
            let read=app.client.get(format!("{}/api/v1/households/{}/people/{id}",app.origin, app.fixture.household.id))
                .bearer_auth(&token).send().await.map_err(|_|DbErr::Custom("Dependent refresh failed".to_owned()))?;
            require(read.status().as_u16()==200,"Declared dependent must be readable after committed creation")?;
            declared_effects(&app,id).await?;
            let committed=effects(&app).await?;
            let replay=create(&app,&token,&person,&key).await?;
            require(replay.status().as_u16()==201 && replay.headers().get("idempotency-replayed").is_some(),
                "Same-key dependent creation must return its committed replay")?;
            require(effects(&app).await?==committed,"Replay must not duplicate declaration or clinical effects")?;
            let mut changed=person.clone();
            changed.as_object_mut().ok_or_else(||DbErr::Custom("Expected changed synthetic declaration".to_owned()))?
                .insert("name".to_owned(),json!("Synthetic changed declaration request"));
            let conflict=create(&app,&token,&changed,&key).await?;
            require(conflict.status().as_u16()==409 && effects(&app).await?==committed,
                "Changed-body replay must conflict without clinical effects")?;
            require(identity(&app).await?==original_identity,
                "Dependent creation must not create or modify Accounts, Users, passwords, passkeys or provider links")?;
        }
        let mut membership = app.fixture.membership.clone().into_active_model();
        membership.status=Set("revoked".into());
        membership.revoked_at=Set(Some(chrono::Utc::now().naive_utc()));
        membership.permissions_version=Set(app.fixture.membership.permissions_version+1);
        membership.update(&app.fixture.admin).await?;
        let mut person=dependent("Synthetic declared minor","minor");
        person.as_object_mut().ok_or_else(||DbErr::Custom("Expected replay input".to_owned()))?
            .insert("initial_guardian_declaration".to_owned(),json!(true));
        let before=effects(&app).await?;
        let denied=create(&app,&token,&person,"synthetic-declared-minor").await?;
        require(denied.status().as_u16()==403 && effects(&app).await?==before,
            "Live membership revocation must deny before saved creation replay")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_declarations_import_new_dependents_require_current_attestations()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        let imported_minor = uuid::Uuid::new_v4().to_string();
        let imported_dependent = uuid::Uuid::new_v4().to_string();
        let payload=json!({"format":"medtracker.portable.v1","scope":"household","exported_at":"2026-10-11T10:00:00Z","source_instance_id":"synthetic:guardian-test","records":{"people":[
            {"portable_id":imported_minor,"name":"Synthetic restored minor","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false},
            {"portable_id":imported_dependent,"name":"Synthetic restored dependent adult","date_of_birth":"1980-01-01","person_type":"dependent_adult","has_capacity":false}
        ]}});
        let passphrase="synthetic guardian import password";
        let bundle=portable_crypto::encrypt(&payload,passphrase)
            .map_err(|_|DbErr::Custom("Unable to encrypt maintained portable test input".to_owned()))?;
        let before=effects(&app).await?;
        let original_identity=identity(&app).await?;
        for declarations in [
            json!([]),
            json!([{"person_portable_id":imported_minor,"accepted":true}]),
            json!([{"person_portable_id":imported_minor,"accepted":true},{"person_portable_id":imported_minor,"accepted":true}]),
            json!([{"person_portable_id":imported_minor,"accepted":true},{"person_portable_id":imported_dependent,"accepted":false}]),
        ] {
            let response=app.client.post(format!("{}/api/v1/households/{}/portable_imports",app.origin, app.fixture.household.id))
                .bearer_auth(&token).header("x-medtracker-portable-passphrase",passphrase)
                .json(&json!({"bundle":bundle,"initial_guardian_declarations":declarations})).send().await
                .map_err(|_|DbErr::Custom("Import declaration request failed before response".to_owned()))?;
            require(response.status().as_u16()==422 && effects(&app).await?==before,
                "Each new imported dependent requires one current true declaration with zero partial effects")?;
        }
        let declarations=json!([
            {"person_portable_id":imported_minor,"accepted":true},
            {"person_portable_id":imported_dependent,"accepted":true}
        ]);
        let applied=app.client.post(format!("{}/api/v1/households/{}/portable_imports",app.origin, app.fixture.household.id))
            .bearer_auth(&token).header("x-medtracker-portable-passphrase",passphrase)
            .json(&json!({"bundle":bundle,"initial_guardian_declarations":declarations})).send().await
            .map_err(|_|DbErr::Custom("Valid import declaration request failed before response".to_owned()))?;
        require(applied.status().as_u16()==201,
            &format!("Two explicit current import declarations must apply201, received{}",applied.status().as_u16()))?;
        let subjects = Query::select().column("id").from("people")
            .and_where(Expr::col("portable_id").is_in([imported_minor.clone(),imported_dependent.clone()]))
            .and_where(Expr::col("id").in_subquery(participants(&app))).to_owned();
        let all_declarations = Query::select().expr_as(Expr::col("id").count(),"count").from("guardian_declarations")
            .and_where(Expr::col("subject_person_id").in_subquery(subjects.clone())).to_owned();
        let mut current_declarations = all_declarations.clone();
        current_declarations.and_where(Expr::col("guardian_person_id").eq(app.fixture.adult.id))
            .and_where(Expr::col("declared_by_account_id").eq(app.fixture.account.id))
            .and_where(Expr::col("declared_at").gt("2026-10-11T10:00:00Z".parse::<chrono::DateTime<chrono::Utc>>().unwrap()))
            .and_where(Expr::col("withdrawn_at").is_null());
        let grants = person_access_grants::Entity::find()
            .filter(Expr::col(person_access_grants::Column::PersonId).in_subquery(subjects)).count(&app.fixture.admin).await?;
        require(count(&app,all_declarations).await?==2 && count(&app,current_declarations).await?==2
            && grants==0 && identity(&app).await?==original_identity,
            "Import must record fresh principal-derived declarations without independent grants or credentials")?;
        let after=effects(&app).await?;
        for field in ["notifications","deliveries","jobs"] {
            require(after.get(field)==before.get(field),"Restore must not create operational notification, delivery or mail jobs")?;
        }
        let repeated_declarations=app.client.post(format!("{}/api/v1/households/{}/portable_imports",app.origin, app.fixture.household.id))
            .bearer_auth(&token).header("x-medtracker-portable-passphrase",passphrase)
            .json(&json!({"bundle":bundle,"initial_guardian_declarations":declarations})).send().await
            .map_err(|_|DbErr::Custom("Fresh repeated declaration request failed before response".to_owned()))?;
        require(repeated_declarations.status().as_u16()==422 && effects(&app).await?==after,
            "A fresh import request must reject declarations for now-existing People without effects")?;
        let replay=app.client.post(format!("{}/api/v1/households/{}/portable_imports",app.origin, app.fixture.household.id))
            .bearer_auth(&token).header("x-medtracker-portable-passphrase",passphrase)
            .json(&json!({"bundle":bundle})).send().await
            .map_err(|_|DbErr::Custom("Existing authorised import replay failed before response".to_owned()))?;
        let replay_status = replay.status().as_u16();
        let replay_body: Value = replay.json().await
            .map_err(|_|DbErr::Custom("Existing import response was not JSON".to_owned()))?;
        let replay_code = replay_body["error"]["code"].as_str()
            .filter(|code|code.len()<=64 && code.bytes().all(|byte|byte.is_ascii_alphanumeric() || byte==b'_'))
            .unwrap_or("absent");
        require(replay_status==201,&format!("Existing authorised records must replay without a new declaration: status={replay_status}, error_code={replay_code}"))?;
        require(effects(&app).await?==after,"Authorised import repeat must preserve clinical records, versions, changes and operational state")?;
        let query = Query::select().expr_as(Expr::col("id").count(),"count").from("guardian_declarations")
            .and_where(Expr::col("guardian_person_id").eq(app.fixture.adult.id)).to_owned();
        require(count(&app,query).await?==2,"Import replay must not duplicate initial declarations")?;
        Ok(())
    }.await;
    app.close().await;
    result
}
