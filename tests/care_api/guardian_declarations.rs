use super::Application;
use med_tracker::models::care::portable_crypto;
use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};
use serde_json::{Value, json};

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

async fn effects(app: &Application) -> Result<Value, DbErr> {
    app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT jsonb_build_object(
          'people',(SELECT count(*) FROM public.people),
          'memberships',(SELECT count(*) FROM public.household_memberships),
          'grants',(SELECT count(*) FROM public.person_access_grants),
          'carers',(SELECT count(*) FROM public.carer_relationships),
          'locations',(SELECT count(*) FROM public.locations),
          'location_links',(SELECT count(*) FROM public.location_memberships),
          'versions',(SELECT count(*) FROM public.versions),
          'changes',(SELECT count(*) FROM public.api_change_events),
          'jobs',(SELECT count(*) FROM public.pg_loco_queue),
          'notifications',(SELECT count(*) FROM public.notification_events),
          'deliveries',(SELECT count(*) FROM public.notification_deliveries),
          'permissions',(SELECT jsonb_agg(jsonb_build_array(id,permissions_version) ORDER BY id) FROM public.household_memberships)
        ) AS state"))
        .await?
        .ok_or_else(|| DbErr::Custom("Expected current clinical effect counts".to_owned()))?
        .try_get("", "state")
}

async fn identity(app: &Application) -> Result<Value, DbErr> {
    app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT jsonb_build_object(
          'accounts',(SELECT count(*) FROM public.accounts),
          'users',(SELECT count(*) FROM public.users),
          'sessions',(SELECT count(*) FROM public.api_sessions),
          'keys',(SELECT count(*) FROM public.identity_api_keys),
          'grants',(SELECT count(*) FROM public.oauth_grants),
          'passwords',(SELECT encode(digest(COALESCE(string_agg(id::text||':'||COALESCE(password,''),',' ORDER BY id),''),'sha256'),'hex') FROM public.auth_credentials),
          'passkeys',(SELECT encode(digest(COALESCE(jsonb_agg(to_jsonb(p) ORDER BY p.id)::text,'[]'),'sha256'),'hex') FROM public.identity_passkeys p),
          'providers',(SELECT encode(digest(COALESCE(jsonb_agg(to_jsonb(p) ORDER BY p.id)::text,'[]'),'sha256'),'hex') FROM public.auth_credentials p),
          'bindings',(SELECT jsonb_agg(jsonb_build_array(id,person_id) ORDER BY id) FROM public.accounts)
        ) AS state"))
        .await?
        .ok_or_else(|| DbErr::Custom("Expected current credential state".to_owned()))?
        .try_get("", "state")
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
        .post(format!("{}/api/v1/households/72001/people", app.origin))
        .bearer_auth(token)
        .header("idempotency-key", key)
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
            json!({"name":"Synthetic forged guardian","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true,"guardian_person_id":73002}),
            json!({"id":73002,"name":"Synthetic existing subject","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}),
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
            let read=app.client.get(format!("{}/api/v1/households/72001/people/{id}",app.origin))
                .bearer_auth(&token).send().await.map_err(|_|DbErr::Custom("Dependent refresh failed".to_owned()))?;
            require(read.status().as_u16()==200,"Declared dependent must be readable after committed creation")?;
            let state=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                "SELECT (SELECT count(*) FROM public.guardian_declarations WHERE subject_person_id=$1 AND guardian_person_id=73001 AND declared_by_account_id=71001 AND statement_version=1 AND withdrawn_at IS NULL) AS declarations,(SELECT count(*) FROM public.household_memberships WHERE household_id=72001 AND person_id=$1 AND status='active' AND revoked_at IS NULL) AS participations,(SELECT count(*) FROM public.person_access_grants WHERE person_id=$1) AS grants,(SELECT count(*) FROM public.versions WHERE item_type='Person' AND item_id=$1 AND event='create') AS person_versions,(SELECT count(*) FROM public.versions v JOIN public.guardian_declarations d ON d.id=v.item_id WHERE v.item_type='GuardianDeclaration' AND d.subject_person_id=$1 AND v.event='create') AS declaration_versions",
                [id.into()])).await?.ok_or_else(||DbErr::Custom("Expected committed declaration effects".to_owned()))?;
            require(state.try_get::<i64>("","declarations")?==1
                && state.try_get::<i64>("","participations")?==1
                && state.try_get::<i64>("","grants")?==0
                && state.try_get::<i64>("","person_versions")?==1
                && state.try_get::<i64>("","declaration_versions")?==1,
                "Creation must commit one explicit declaration/participation/provenance and no separate Manage grant")?;
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
        app.fixture.admin.execute_unprepared("UPDATE public.household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001").await?;
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
        let payload=json!({"format":"medtracker.portable.v1","scope":"household","exported_at":"2026-10-11T10:00:00Z","source_instance_id":"synthetic:guardian-test","records":{"people":[
            {"portable_id":"synthetic-imported-dependent-1","name":"Synthetic restored minor","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false},
            {"portable_id":"synthetic-imported-dependent-2","name":"Synthetic restored dependent adult","date_of_birth":"1980-01-01","person_type":"dependent_adult","has_capacity":false}
        ]}});
        let passphrase="synthetic guardian import password";
        let bundle=portable_crypto::encrypt(&payload,passphrase)
            .map_err(|_|DbErr::Custom("Unable to encrypt maintained portable test input".to_owned()))?;
        let before=effects(&app).await?;
        let original_identity=identity(&app).await?;
        for declarations in [
            json!([]),
            json!([{"person_portable_id":"synthetic-imported-dependent-1","accepted":true}]),
            json!([{"person_portable_id":"synthetic-imported-dependent-1","accepted":true},{"person_portable_id":"synthetic-imported-dependent-1","accepted":true}]),
            json!([{"person_portable_id":"synthetic-imported-dependent-1","accepted":true},{"person_portable_id":"synthetic-imported-dependent-2","accepted":false}]),
        ] {
            let response=app.client.post(format!("{}/api/v1/households/72001/portable_imports",app.origin))
                .bearer_auth(&token).header("x-medtracker-portable-passphrase",passphrase)
                .json(&json!({"bundle":bundle,"initial_guardian_declarations":declarations})).send().await
                .map_err(|_|DbErr::Custom("Import declaration request failed before response".to_owned()))?;
            require(response.status().as_u16()==422 && effects(&app).await?==before,
                "Each new imported dependent requires one current true declaration with zero partial effects")?;
        }
        let declarations=json!([
            {"person_portable_id":"synthetic-imported-dependent-1","accepted":true},
            {"person_portable_id":"synthetic-imported-dependent-2","accepted":true}
        ]);
        let applied=app.client.post(format!("{}/api/v1/households/72001/portable_imports",app.origin))
            .bearer_auth(&token).header("x-medtracker-portable-passphrase",passphrase)
            .json(&json!({"bundle":bundle,"initial_guardian_declarations":declarations})).send().await
            .map_err(|_|DbErr::Custom("Valid import declaration request failed before response".to_owned()))?;
        require(applied.status().as_u16()==201,
            &format!("Two explicit current import declarations must apply201, received{}",applied.status().as_u16()))?;
        let state=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS declarations,count(*) FILTER(WHERE d.guardian_person_id=73001 AND d.declared_by_account_id=71001 AND d.declared_at>'2026-10-11T10:00:00Z'::timestamptz AND d.withdrawn_at IS NULL) AS current_actor,(SELECT count(*) FROM public.person_access_grants g JOIN public.people p ON p.id=g.person_id WHERE p.portable_id IN('synthetic-imported-dependent-1','synthetic-imported-dependent-2')) AS grants FROM public.guardian_declarations d JOIN public.people p ON p.id=d.subject_person_id WHERE p.portable_id IN('synthetic-imported-dependent-1','synthetic-imported-dependent-2')"))
            .await?.ok_or_else(||DbErr::Custom("Expected imported declaration facts".to_owned()))?;
        require(state.try_get::<i64>("","declarations")?==2 && state.try_get::<i64>("","current_actor")?==2
            && state.try_get::<i64>("","grants")?==0 && identity(&app).await?==original_identity,
            "Import must record fresh principal-derived declarations without independent grants or credentials")?;
        let after=effects(&app).await?;
        for field in ["notifications","deliveries","jobs"] {
            require(after.get(field)==before.get(field),"Restore must not create operational notification, delivery or mail jobs")?;
        }
        let repeated_declarations=app.client.post(format!("{}/api/v1/households/72001/portable_imports",app.origin))
            .bearer_auth(&token).header("x-medtracker-portable-passphrase",passphrase)
            .json(&json!({"bundle":bundle,"initial_guardian_declarations":declarations})).send().await
            .map_err(|_|DbErr::Custom("Fresh repeated declaration request failed before response".to_owned()))?;
        require(repeated_declarations.status().as_u16()==422 && effects(&app).await?==after,
            "A fresh import request must reject declarations for now-existing People without effects")?;
        let replay=app.client.post(format!("{}/api/v1/households/72001/portable_imports",app.origin))
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
        let count=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS count FROM public.guardian_declarations WHERE guardian_person_id=73001"))
            .await?.ok_or_else(||DbErr::Custom("Expected retained declaration count".to_owned()))?;
        require(count.try_get::<i64>("","count")?==2,"Import replay must not duplicate initial declarations")?;
        Ok(())
    }.await;
    app.close().await;
    result
}
