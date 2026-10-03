use super::configuration::secret;
use super::helpers::{
    database_error, field, form_fields, html, oauth_error, redirect, transaction,
};
use super::sessions::{
    browser_cookie_age, browser_session, pending, renewed_session_cookie, Pending, PENDING_COOKIE,
    SESSION_COOKIE,
};
use crate::entities::{oauth_application, oauth_grant};
use crate::{tenant_setting, AppState};
use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use url::form_urlencoded;

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub(super) struct AuthorizationRequest {
    response_type: String,
    response_mode: String,
    client_id: String,
    redirect_uri: String,
    scope: String,
    state: String,
    code_challenge: String,
    code_challenge_method: String,
}

impl AuthorizationRequest {
    pub(super) fn path(&self) -> String {
        let mut query = form_urlencoded::Serializer::new(String::new());
        query
            .append_pair("response_type", &self.response_type)
            .append_pair("response_mode", &self.response_mode)
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("scope", &self.scope)
            .append_pair("state", &self.state)
            .append_pair("code_challenge", &self.code_challenge)
            .append_pair("code_challenge_method", &self.code_challenge_method);
        format!("/authorize?{}", query.finish())
    }
}

#[derive(Clone)]
pub(super) struct Client {
    pub(super) id: i64,
    name: String,
    client_id: String,
    redirect_uri: String,
    scopes: String,
}

pub(super) async fn client(
    db: &DatabaseTransaction,
    client_id: &str,
) -> Result<Option<Client>, Response> {
    let record = oauth_application::Entity::find()
        .filter(oauth_application::Column::ClientId.eq(client_id))
        .filter(oauth_application::Column::ClientKind.eq("mobile"))
        .filter(oauth_application::Column::TokenEndpointAuthMethod.eq("none"))
        .filter(oauth_application::Column::ClientSecret.is_null())
        .filter(oauth_application::Column::ClientSecretHash.is_null())
        .one(db)
        .await
        .map_err(|e| database_error(e).into_response())?;
    Ok(record.map(|r| Client {
        id: r.id,
        name: r.name,
        client_id: r.client_id,
        redirect_uri: r.redirect_uri,
        scopes: r.scopes,
    }))
}

fn valid_request(request: &AuthorizationRequest, client: &Client) -> bool {
    request.response_type == "code"
        && request.response_mode == "query"
        && request.client_id == client.client_id
        && client
            .redirect_uri
            .split_whitespace()
            .any(|uri| uri == request.redirect_uri)
        && !request.state.is_empty()
        && request.state.len() <= 256
        && super::pkce::valid_challenge(&request.code_challenge_method, &request.code_challenge)
        && !request.scope.is_empty()
        && request.scope.split_whitespace().all(|scope| {
            client
                .scopes
                .split_whitespace()
                .any(|allowed| allowed == scope)
        })
        && request
            .scope
            .split_whitespace()
            .any(|scope| scope == "medtracker")
        && request.scope.split_whitespace().count()
            == request
                .scope
                .split_whitespace()
                .collect::<std::collections::HashSet<_>>()
                .len()
}

pub(super) async fn authorize(
    State(state): State<AppState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    let Some(query) = uri.query() else {
        return oauth_error("invalid_request");
    };
    if query.len() > 2048 {
        return oauth_error("invalid_request");
    }
    let pairs = form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect::<Vec<_>>();
    let mut query = HashMap::new();
    for (key, value) in pairs {
        if query.insert(key, value).is_some() {
            return oauth_error("invalid_request");
        }
    }
    if query.len() != 8 {
        return oauth_error("invalid_request");
    }
    let request = match serde_json::from_value::<AuthorizationRequest>(json!(query)) {
        Ok(request) => request,
        Err(_) => return oauth_error("invalid_request"),
    };
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let registered = match client(&db, &request.client_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return oauth_error("unauthorized_client"),
        Err(error) => return error,
    };
    if !valid_request(&request, &registered) {
        return oauth_error("invalid_request");
    }
    let session = match browser_session(&state, &db, &headers).await {
        Ok(value) => value,
        Err(error) => return error,
    };
    if let Err(error) = db.commit().await {
        return database_error(error).into_response();
    }
    let pending = match pending(&state, &headers) {
        Some(value) if value.request == request => value,
        _ => Pending {
            request,
            csrf: secret(),
            issued_at: Utc::now().timestamp(),
        },
    };
    let Some(cookie) = state.oauth.sign(&pending) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut response = if let Some(session) = session {
        let fields = vec![
            (
                "response_type".to_owned(),
                pending.request.response_type.clone(),
            ),
            (
                "response_mode".to_owned(),
                pending.request.response_mode.clone(),
            ),
            ("client_id".to_owned(), pending.request.client_id.clone()),
            (
                "redirect_uri".to_owned(),
                pending.request.redirect_uri.clone(),
            ),
            ("state".to_owned(), pending.request.state.clone()),
            (
                "code_challenge".to_owned(),
                pending.request.code_challenge.clone(),
            ),
            (
                "code_challenge_method".to_owned(),
                pending.request.code_challenge_method.clone(),
            ),
        ];
        let mut response = html(medtracker_web::render_consent(
            &session.csrf,
            &registered.name,
            &pending
                .request
                .scope
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
            &fields,
        ));
        if let Some(cookie) = renewed_session_cookie(&state, &session) {
            response.headers_mut().append(header::SET_COOKIE, cookie);
        }
        response
    } else {
        redirect("/login")
    };
    response.headers_mut().append(
        header::SET_COOKIE,
        state.oauth.cookie(PENDING_COOKIE, &cookie, 600),
    );
    response
}

pub(super) async fn consent(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let Some(pending) = pending(&state, &headers) else {
        return StatusCode::FORBIDDEN.into_response();
    };
    let Some(fields) = form_fields(&headers, &body) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let db = match transaction(&state).await {
        Ok(db) => db,
        Err(error) => return error,
    };
    let session = match browser_session(&state, &db, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return StatusCode::FORBIDDEN.into_response(),
        Err(error) => return error,
    };
    if field(&fields, "authenticity_token") != Some(session.csrf.as_str()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    for (key, expected) in [
        ("response_type", pending.request.response_type.as_str()),
        ("response_mode", pending.request.response_mode.as_str()),
        ("client_id", pending.request.client_id.as_str()),
        ("redirect_uri", pending.request.redirect_uri.as_str()),
        ("state", pending.request.state.as_str()),
        ("code_challenge", pending.request.code_challenge.as_str()),
        (
            "code_challenge_method",
            pending.request.code_challenge_method.as_str(),
        ),
    ] {
        if field(&fields, key) != Some(expected) {
            return oauth_error("invalid_request");
        }
    }
    let requested = pending.request.scope.split_whitespace().collect::<Vec<_>>();
    let selected = fields
        .iter()
        .filter(|(key, _)| key == "scope[]")
        .map(|(_, value)| value.as_str())
        .collect::<Vec<_>>();
    if selected.is_empty()
        || !selected.contains(&"medtracker")
        || selected.len()
            != selected
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
        || !selected.iter().all(|scope| requested.contains(scope))
    {
        return oauth_error("invalid_scope");
    }
    let approved_scopes = selected.join(" ");
    let registered = match client(&db, &pending.request.client_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return oauth_error("unauthorized_client"),
        Err(error) => return error,
    };
    if !valid_request(&pending.request, &registered) {
        return oauth_error("invalid_request");
    }
    if let Err(error) =
        tenant_setting(&db, "med_tracker.current_account_id", session.account_id).await
    {
        return database_error(error).into_response();
    }
    let code = secret();
    let now = Utc::now().naive_utc();
    let authenticated_at = match chrono::DateTime::<Utc>::from_timestamp(session.issued_at, 0) {
        Some(value) => value.naive_utc(),
        None => return StatusCode::FORBIDDEN.into_response(),
    };
    let grant = oauth_grant::ActiveModel {
        account_id: Set(session.account_id),
        oauth_application_id: Set(registered.id),
        client_kind: Set("mobile".to_owned()),
        scopes: Set(approved_scopes),
        code: Set(Some(code.clone())),
        code_challenge: Set(Some(pending.request.code_challenge.clone())),
        code_challenge_method: Set(Some("S256".to_owned())),
        redirect_uri: Set(Some(pending.request.redirect_uri.clone())),
        expires_in: Set(now + Duration::minutes(5)),
        authenticated_at: Set(Some(authenticated_at)),
        last_used_at: Set(Some(now)),
        device_name: Set(Some(registered.name)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let insert = grant.insert(&db).await;
    if let Err(error) = insert {
        return database_error(error).into_response();
    }
    if let Err(error) = db.commit().await {
        return database_error(error).into_response();
    }
    let separator = if pending.request.redirect_uri.contains('?') {
        '&'
    } else {
        '?'
    };
    let mut query = form_urlencoded::Serializer::new(String::new());
    query
        .append_pair("code", &code)
        .append_pair("state", &pending.request.state);
    let callback = format!(
        "{}{separator}{}",
        pending.request.redirect_uri,
        query.finish()
    );
    let mut response = redirect(&callback);
    if let Some(cookie) = state.oauth.sign(&session) {
        response.headers_mut().append(
            header::SET_COOKIE,
            state
                .oauth
                .cookie(SESSION_COOKIE, &cookie, browser_cookie_age()),
        );
    }
    response.headers_mut().append(
        header::SET_COOKIE,
        state.oauth.cookie(PENDING_COOKIE, "", 0),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::{valid_request, AuthorizationRequest, Client};

    #[test]
    fn s256_challenges_must_encode_exactly_one_sha256_digest() {
        let client = Client {
            id: 1,
            name: "Test".into(),
            client_id: "mobile".into(),
            redirect_uri: "app://callback".into(),
            scopes: "medtracker".into(),
        };
        let mut request = AuthorizationRequest {
            response_type: "code".into(),
            response_mode: "query".into(),
            client_id: client.client_id.clone(),
            redirect_uri: client.redirect_uri.clone(),
            scope: "medtracker".into(),
            state: "state".into(),
            code_challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".into(),
            code_challenge_method: "S256".into(),
        };
        assert!(valid_request(&request, &client));
        for malformed in [
            "A".repeat(44),
            "A".repeat(128),
            format!("{}B", "A".repeat(42)),
        ] {
            request.code_challenge = malformed;
            assert!(!valid_request(&request, &client));
        }
    }
}
