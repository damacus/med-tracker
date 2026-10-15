use super::*;
use base64::Engine;
use sha2::{Digest, Sha256};

type SecurityResult = Result<(), Box<dyn std::error::Error>>;

const PATH: &str = "/api/v1/auth/medication_history";

async fn audit_count(app: &Application) -> Result<i64, sea_orm::DbErr> {
    app.fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS n FROM public.account_clinical_access_events",
        ))
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("missing audit count".into()))?
        .try_get("", "n")
}

fn token_digest(token: &str) -> String {
    base64::engine::general_purpose::URL_SAFE.encode(Sha256::digest(token.as_bytes()))
}

fn no_store(response: &reqwest::Response) -> SecurityResult {
    if response
        .headers()
        .get("cache-control")
        .and_then(|value| value.to_str().ok())
        != Some("no-store")
    {
        return Err(format!("{} response was cacheable", response.status()).into());
    }
    Ok(())
}

#[tokio::test]
async fn self_history_excludes_personal_smart_and_household_bound_stored_credentials()
-> SecurityResult {
    let app = Application::new().await;
    let result=async {
        app.fixture.admin.execute_unprepared("UPDATE people SET date_of_birth='1985-02-03' WHERE id=73001").await?;
        let personal=super::profile::issue_profile_personal_key(&app,"Synthetic self history exclusion").await;
        let smart="synthetic-self-history-smart";
        let session="synthetic-self-history-bound-session";
        let app_token="synthetic-self-history-bound-app-token";
        app.fixture.admin.execute_unprepared("INSERT INTO oauth_applications(id,account_id,client_id,client_kind,name,redirect_uri,scopes,token_endpoint_auth_method,client_secret_hash,created_at,updated_at) VALUES(75971,71001,'self-history-smart','integration','Synthetic SMART client','https://example.test/callback','launch/patient patient/*.rs offline_access','client_secret_post',crypt('password',gen_salt('bf',4)),now(),now())").await?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO oauth_grants(id,account_id,oauth_application_id,client_kind,household_membership_id,person_id,permissions_version,redirect_uri,scopes,token_hash,expires_in,authenticated_at,last_used_at,created_at,updated_at) VALUES(76971,71001,75971,'integration',74001,73001,1,'https://example.test/callback','launch/patient patient/*.rs offline_access',$1,now()+interval '15 minutes',now(),now(),now(),now())",
            [token_digest(smart).into()])).await?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO api_sessions(id,account_id,household_membership_id,permissions_version,access_token_digest,refresh_token_digest,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(87971,71001,74001,1,$1,'synthetic-self-history-refresh',now()+interval '15 minutes',now()+interval '1 day',now(),now(),now())",
            [hex::encode(Sha256::digest(session.as_bytes())).into()])).await?;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO api_app_tokens(id,account_id,household_membership_id,name,token_digest,permissions_version,expires_at,last_used_at,created_at,updated_at) VALUES(87972,71001,74001,'Synthetic self history app',$1,1,now()+interval '1 month',now(),now(),now())",
            [hex::encode(Sha256::digest(app_token.as_bytes())).into()])).await?;
        for (kind,token) in [("personal key",personal.as_str()),("bound session",session),("bound app token",app_token)] {
            let response=app.client.get(format!("{}/api/v1/households/72001/people",app.origin))
                .bearer_auth(token).header("x-medtracker-key-permission","care:read").send().await?;
            if response.status().as_u16()!=200 {return Err(format!("local care prerequisite failed for {kind}: {}",response.status()).into());}
        }
        let mut statuses=Vec::new();
        for token in [personal.as_str(),smart,session,app_token] {
            let response=app.client.get(format!("{}{PATH}",app.origin))
                .bearer_auth(token).header("x-medtracker-key-permission","care:read").send().await?;
            no_store(&response)?;
            let status=response.status().as_u16();
            let body=response.text().await?;
            if body.contains("data\":") || body.contains("Synthetic tablets") {
                return Err("excluded credential received clinical data".into());
            }
            statuses.push(status);
        }
        let logout=app.client.delete(format!("{}/api/v1/auth/logout",app.origin))
            .bearer_auth(smart).send().await?;
        if logout.status().as_u16()!=204 {return Err(format!("SMART logout prerequisite failed: {}",logout.status()).into());}
        let smart_row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT g.revoked_at IS NOT NULL AS revoked,(SELECT count(*) FROM security_audit_events WHERE event_type='smart_oauth.revoked' AND actor_account_id=71001) AS audits FROM oauth_grants g WHERE g.id=76971"))
            .await?.ok_or("missing SMART grant")?;
        if !smart_row.try_get::<bool>("","revoked")? || smart_row.try_get::<i64>("","audits")?!=1 {
            return Err("SMART credential did not reach maintained logout authority".into());
        }
        Ok::<_,Box<dyn std::error::Error>>((statuses,audit_count(&app).await?))
    }.await;
    app.close().await;
    let (statuses, audits) = result?;
    assert_eq!(statuses, [401, 401, 403, 403]);
    assert_eq!(audits, 0);
    Ok(())
}

#[tokio::test]
async fn mobile_token_without_effective_medtracker_scope_cannot_gain_self_history_or_audit()
-> SecurityResult {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        app.fixture.admin.execute_unprepared("UPDATE oauth_grants SET access_token_scopes='profile',access_token_scope_hash=token_hash WHERE id=76001 AND client_kind='mobile'").await?;
        let response=app.client.get(format!("{}{PATH}",app.origin)).bearer_auth(token).send().await?;
        no_store(&response)?;
        Ok::<_,Box<dyn std::error::Error>>((response.status().as_u16(),response.text().await?,audit_count(&app).await?))
    }.await;
    app.close().await;
    let (status, body, audits) = result?;
    assert_eq!(status, 403, "{body}");
    assert!(!body.contains("Synthetic tablets"));
    assert_eq!(audits, 0);
    Ok(())
}

#[tokio::test]
async fn source_audit_uses_server_trace_and_excludes_clinical_and_client_markers() -> SecurityResult
{
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        app.fixture.admin.execute_unprepared("UPDATE people SET clinical_name='PRIVATE_CLINICAL_HISTORY_MARKER' WHERE id=73001; UPDATE medications SET name='PRIVATE_MEDICINE_HISTORY_MARKER' WHERE id=80001; UPDATE person_medications SET administration_kind=1 WHERE id=81001").await?;
        let recorded=app.client.post(format!("{}/api/v1/households/72001/medication_takes",app.origin))
            .bearer_auth(&token).json(&json!({"medication_take":{"client_uuid":"96800000-0000-4000-8000-000000009791","source_type":"person_medication","source_id":"81001","taken_from_medication_id":80001,"dose_amount":"2","dose_unit":"tablet","taken_at":"2026-10-05T10:00:00Z"}})).send().await?;
        if recorded.status().as_u16()!=201 {return Err(format!("real clinical write prerequisite failed: {}",recorded.status()).into());}
        let spoof="97000000-0000-4000-8000-000000009799";
        let response=app.client.get(format!("{}{PATH}",app.origin)).bearer_auth(&token)
            .header("x-request-id",spoof)
            .header("x-medtracker-trace-id","97000000-0000-4000-8000-000000009798").send().await?;
        no_store(&response)?;
        let status=response.status().as_u16();
        let request_id=response.headers().get("x-medtracker-trace-id").ok_or("missing server response trace")?.to_str()?.to_owned();
        let body=response.text().await?;
        let audit=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT e.audit_context->>'request_id' AS request_id,to_jsonb(e)::text AS source,l.source_payload::text AS ledger FROM public.account_clinical_access_events e JOIN public.audit_ledger_entries l ON l.source_table='account_clinical_access_events' AND l.source_id=e.id WHERE e.event_type='self_medication_history.read' ORDER BY e.id DESC LIMIT 1"))
            .await?.ok_or("missing source audit")?;
        Ok::<_,Box<dyn std::error::Error>>((status,request_id,body,audit.try_get::<String>("","request_id")?,audit.try_get::<String>("","source")?,audit.try_get::<String>("","ledger")?))
    }.await;
    app.close().await;
    let (status, response_trace, body, audit_trace, source, ledger) = result?;
    assert_eq!(status, 200, "{body}");
    assert!(
        body.contains("PRIVATE_MEDICINE_HISTORY_MARKER"),
        "clinical fixture did not reach the read"
    );
    assert_ne!(response_trace, "97000000-0000-4000-8000-000000009799");
    assert_ne!(response_trace, "97000000-0000-4000-8000-000000009798");
    assert_eq!(uuid::Uuid::parse_str(&response_trace)?.get_version_num(), 4);
    assert_eq!(uuid::Uuid::parse_str(&audit_trace)?.get_version_num(), 4);
    assert_ne!(audit_trace, "97000000-0000-4000-8000-000000009799");
    assert_ne!(audit_trace, "97000000-0000-4000-8000-000000009798");
    for secret in [
        "PRIVATE_CLINICAL_HISTORY_MARKER",
        "PRIVATE_MEDICINE_HISTORY_MARKER",
        "97000000-0000-4000-8000-000000009799",
        "97000000-0000-4000-8000-000000009798",
    ] {
        assert!(!source.contains(secret));
        assert!(!ledger.contains(secret));
    }
    Ok(())
}

#[tokio::test]
async fn source_audit_failure_returns_no_clinical_response_and_rolls_back() -> SecurityResult {
    let app = Application::new().await;
    let result=async {
        let token=app.token().await;
        app.fixture.admin.execute_unprepared("UPDATE medications SET name='PRIVATE_AUDIT_FAILURE_CLINICAL_MARKER' WHERE id=80001; UPDATE person_medications SET administration_kind=1 WHERE id=81001").await?;
        let recorded=app.client.post(format!("{}/api/v1/households/72001/medication_takes",app.origin))
            .bearer_auth(&token).json(&json!({"medication_take":{"client_uuid":"96800000-0000-4000-8000-000000009792","source_type":"person_medication","source_id":"81001","taken_from_medication_id":80001,"dose_amount":"2","dose_unit":"tablet","taken_at":"2026-10-05T10:00:00Z"}})).send().await?;
        if recorded.status().as_u16()!=201 {return Err(format!("real clinical write prerequisite failed: {}",recorded.status()).into());}
        app.fixture.admin.execute_unprepared("CREATE SEQUENCE public.synthetic_self_history_audit_reached; GRANT USAGE,SELECT ON SEQUENCE public.synthetic_self_history_audit_reached TO med_tracker_owner; CREATE FUNCTION public.synthetic_self_history_source_failure() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN PERFORM nextval('public.synthetic_self_history_audit_reached'); RAISE EXCEPTION 'PRIVATE_AUDIT_FAILURE_MARKER' USING ERRCODE='23514'; END $$; CREATE TRIGGER synthetic_self_history_source_failure BEFORE INSERT ON public.account_clinical_access_events FOR EACH ROW EXECUTE FUNCTION public.synthetic_self_history_source_failure()").await?;
        let response=app.client.get(format!("{}{PATH}",app.origin)).bearer_auth(token).send().await?;
        no_store(&response)?;
        let status=response.status().as_u16();
        let body=response.text().await?;
        let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT is_called FROM public.synthetic_self_history_audit_reached) AS reached,(SELECT count(*) FROM public.account_clinical_access_events) AS source,(SELECT count(*) FROM public.audit_ledger_entries WHERE source_table='account_clinical_access_events') AS ledger,(SELECT count(*) FROM public.audit_export_deliveries d JOIN public.audit_ledger_entries l ON l.id=d.audit_ledger_entry_id WHERE l.source_table='account_clinical_access_events') AS delivery"))
            .await?.ok_or("missing rollback counts")?;
        Ok::<_,Box<dyn std::error::Error>>((status,body,row.try_get::<bool>("","reached")?,row.try_get::<i64>("","source")?,row.try_get::<i64>("","ledger")?,row.try_get::<i64>("","delivery")?))
    }.await;
    app.close().await;
    let (status, body, reached, source, ledger, delivery) = result?;
    assert_eq!(status, 500, "{body}");
    assert!(reached, "the source INSERT rejection must be reached");
    assert!(!body.contains("data\":"));
    assert!(!body.contains("PRIVATE_AUDIT_FAILURE_MARKER"));
    assert!(!body.contains("PRIVATE_AUDIT_FAILURE_CLINICAL_MARKER"));
    assert_eq!((source, ledger, delivery), (0, 0, 0));
    Ok(())
}
