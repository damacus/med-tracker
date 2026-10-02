use super::*;

pub(super) async fn reset_password_unavailable() -> Response {
    (
        StatusCode::NOT_IMPLEMENTED,
        html(medtracker_web::render_reset_unavailable()),
    )
        .into_response()
}

pub(super) async fn styles() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        medtracker_web::stylesheet(),
    )
        .into_response()
}

pub(super) async fn discovery(State(state): State<AppState>) -> Response {
    let base = state.oauth.base_url.as_str().trim_end_matches('/');
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({
            "issuer": base,
            "authorization_endpoint": format!("{base}/authorize"),
            "token_endpoint": format!("{base}/token"),
            "revocation_endpoint": format!("{base}/revoke"),
            "response_types_supported": ["code"],
            "grant_types_supported": ["authorization_code", "refresh_token"],
            "code_challenge_methods_supported": ["S256"],
            "token_endpoint_auth_methods_supported": ["none"]
        })),
    )
        .into_response()
}

pub(super) async fn capabilities(State(state): State<AppState>) -> Response {
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let rows = match oauth_application::Entity::find()
        .filter(oauth_application::Column::ClientKind.eq("mobile"))
        .filter(oauth_application::Column::TokenEndpointAuthMethod.eq("none"))
        .order_by_asc(oauth_application::Column::ClientId)
        .all(&db)
        .await
    {
        Ok(rows) => rows,
        Err(error) => return database_error(error).into_response(),
    };
    let clients: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "client_id": r.client_id, "name": r.name,
                "redirect_uris": r.redirect_uri.split_whitespace().collect::<Vec<_>>(),
                "scopes": r.scopes.split_whitespace().collect::<Vec<_>>()
            })
        })
        .collect();
    let _ = db.commit().await;
    let base = state.oauth.base_url.as_str().trim_end_matches('/');
    ([(header::CACHE_CONTROL, "no-store")], Json(json!({"data": {
        "format": "medtracker.api.capabilities.v1", "api_version": "v1",
        "authentication": {"methods": ["oauth_bearer", "api_app_token"], "hosted_mobile": "rodauth_authorization_code_pkce", "mobile_oauth": {
            "discovery_url": format!("{base}/.well-known/oauth-authorization-server"), "household_binding": "account",
            "inactivity_timeout_days": configured_lifetime_days("SESSION_INACTIVITY_TIMEOUT_DAYS", 30, 1).unwrap_or(0),
            "maximum_age_days": configured_lifetime_days("SESSION_MAX_AGE_DAYS", 0, 0).unwrap_or(0), "clients": clients
        }},
        "administration": {"household": true, "fresh_mfa_required": false, "app_tokens": true, "audit_logs": true, "invitations": true, "person_access_grants": true},
        "medication_pause_periods": {"supported": true, "reasons": ["out_of_supply", "temporarily_not_needed", "clinician_advice", "side_effects", "other"], "effective_time": "server_acceptance"},
        "dose_outcomes": {"source_types": ["schedule", "person_medication"], "max_read_days": 31, "actions": ["not_taken", "reopen", "take"], "replacement_requires_version": true},
        "stock_removals": {"actions": ["create", "index"], "submission_id_required": true, "max_page_size": 100},
        "location_management": {"actions": ["create", "update", "destroy"], "version_required": true, "person_memberships": ["create", "destroy"], "memberships_online_only": true},
        "medication_reviews": {"actions": ["index", "show", "update"], "version_required": true, "max_page_size": 100},
        "reports": {"formats": ["json", "pdf"], "health_history": true, "medication_reviews": true, "selected_person_required": true, "health_history_max_span_days": 366},
        "profile": {"actions": ["show", "update"], "online_only": true, "avatar": {"actions": ["show", "update", "destroy"], "max_bytes": 5_242_880, "content_types": ["image/png", "image/jpeg", "image/webp"]}},
        "invitations": {"actions": ["accept", "resend"], "online_only": true, "acceptance_session_required": true},
        "portable_formats": [],
        "backups": {"encrypted_migration_bundle": false, "unencrypted_zip": false, "health_data_json": false},
        "fhir": {"version": "R4", "resources": []},
        "sync": {"portable_ids": true, "numeric_ids": "backward_compatible", "mobile_snapshot": false, "dry_run_import": false, "idempotency_keys": true, "etag_conflicts": true, "change_feed": false, "batch_mutations": false, "tombstones": false, "operations": [], "online_only_resources": ["account", "profile", "avatar", "invitation", "household_membership", "person_access_grant", "location_membership", "report", "api_session", "api_app_token"]},
        "client_tools": {"cli": {"supported": false, "status": "available", "binary": "medtracker", "api_boundary": "/api/v1", "distribution": "github_release"}, "mcp_server": {"supported": false, "transport": "streamable_http", "endpoint": "/mcp", "stdio_binary": "medtracker-mcp", "tools": [], "resources": []}, "diagnostics": ["request_id", "retry_after"]}
    }}))).into_response()
}

pub(super) async fn households(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let (account_id, credential_household_id) = if headers.contains_key(header::AUTHORIZATION) {
        match crate::auth_sessions::household_read_scope(&db, &headers).await {
            Ok(scope) => scope,
            Err(error) => return error.into_response(),
        }
    } else {
        let session = match browser_session(&state, &db, &headers).await {
            Ok(Some(session)) => session,
            Ok(None) => return crate::ApiError::unauthorized().into_response(),
            Err(error) => return error,
        };
        (session.account_id, None)
    };
    if let Err(error) = tenant_setting(&db, "med_tracker.current_account_id", account_id).await {
        return database_error(error).into_response();
    }
    let mut membership_query = membership::Entity::find()
        .filter(membership::Column::AccountId.eq(account_id))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null());
    if let Some(household_id) = credential_household_id {
        membership_query =
            membership_query.filter(membership::Column::HouseholdId.eq(household_id));
    }
    let memberships = membership_query
        .order_by_asc(membership::Column::Id)
        .all(&db)
        .await;
    let memberships = match memberships {
        Ok(rows) => rows,
        Err(error) => return database_error(error).into_response(),
    };
    let household_ids: Vec<i64> = memberships.iter().map(|m| m.household_id).collect();
    let households = match household::Entity::find()
        .filter(household::Column::Id.is_in(household_ids))
        .filter(household::Column::Status.eq("active"))
        .filter(household::Column::LifecycleState.eq("active"))
        .all(&db)
        .await
    {
        Ok(rows) => rows,
        Err(error) => return database_error(error).into_response(),
    };
    let households: HashMap<i64, household::Model> =
        households.into_iter().map(|h| (h.id, h)).collect();
    let data: Vec<Value> = memberships.into_iter().filter_map(|m| {
        let h = households.get(&m.household_id)?;
        Some(json!({"id": h.id, "slug": h.slug, "name": h.name, "role": m.role, "membership_id": m.id}))
    }).collect();
    if let Err(error) = db.commit().await {
        return database_error(error).into_response();
    }
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({"account_id": account_id, "data": data})),
    )
        .into_response()
}
