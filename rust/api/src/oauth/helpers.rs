use super::*;

pub(super) fn redirect(location: &str) -> Response {
    (
        StatusCode::FOUND,
        [(header::LOCATION, location)],
        [(header::CACHE_CONTROL, "no-store")],
    )
        .into_response()
}

pub(super) fn oauth_error(code: &'static str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({"error": code})),
    )
        .into_response()
}

pub(super) fn database_error(_: sea_orm::DbErr) -> crate::ApiError {
    eprintln!("OAuth database operation failed");
    crate::ApiError::internal()
}

pub(super) fn html(body: String) -> Response {
    (
        [
            (header::CACHE_CONTROL, "no-store"),
            (
                header::CONTENT_SECURITY_POLICY,
                "default-src 'none'; script-src 'self'; style-src 'self'; font-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'",
            ),
        ],
        Html(body),
    )
        .into_response()
}

pub(super) fn sql(query: &str, values: impl IntoIterator<Item = sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, query, values)
}

pub(super) async fn transaction(state: &AppState) -> Result<DatabaseTransaction, Response> {
    let db = state
        .db
        .begin()
        .await
        .map_err(|e| database_error(e).into_response())?;
    restricted_role(&db)
        .await
        .map_err(|e| database_error(e).into_response())?;
    Ok(db)
}

pub(super) fn form_fields(headers: &HeaderMap, body: &[u8]) -> Option<Vec<(String, String)>> {
    if body.len() > 8192
        || !headers
            .get(header::CONTENT_TYPE)?
            .to_str()
            .ok()?
            .starts_with("application/x-www-form-urlencoded")
    {
        return None;
    }
    Some(form_urlencoded::parse(body).into_owned().collect())
}

pub(super) fn field<'a>(fields: &'a [(String, String)], name: &str) -> Option<&'a str> {
    let mut values = fields
        .iter()
        .filter(|(key, _)| key == name)
        .map(|(_, value)| value.as_str());
    let value = values.next()?;
    values.next().is_none().then_some(value)
}

#[cfg(test)]
mod tests {
    #[test]
    fn authentication_pages_allow_only_same_origin_fonts() {
        let response = super::html(String::new());
        let policy = response.headers()[super::header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap();
        let font_source = policy
            .split(';')
            .map(str::trim)
            .find(|directive| directive.starts_with("font-src"));
        assert_eq!(font_source, Some("font-src 'self'"));
    }
}
