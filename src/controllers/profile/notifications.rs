use super::*;
use crate::models::{access::TenantTransaction, notification_preferences};

const FLAGS: [&str; 5] = [
    "enabled",
    "dose_due_enabled",
    "missed_dose_enabled",
    "low_stock_enabled",
    "private_text_enabled",
];
const TIMES: [&str; 4] = [
    "morning_time",
    "afternoon_time",
    "evening_time",
    "night_time",
];

pub(super) async fn context(
    tenant: &TenantTransaction,
    account_id: i64,
    labels: &Value,
    draft: Option<&HashMap<String, String>>,
) -> std::result::Result<Value, OperationError> {
    let mut values = match notification_preferences::read(tenant, account_id).await {
        Ok((row, owner)) => {
            notification_preferences::representation(&row, &owner).0["data"].clone()
        }
        Err(OperationError::NotFound) => {
            json!({"enabled":true,"dose_due_enabled":true,"missed_dose_enabled":true,"low_stock_enabled":true,"private_text_enabled":false,"morning_time":"08:00","afternoon_time":"14:00","evening_time":"18:00","night_time":"22:00"})
        }
        Err(error) => return Err(error),
    };
    for field in TIMES {
        values[field] = json!(
            values[field]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(5)
                .collect::<String>()
        );
    }
    if let Some(draft) = draft.filter(|draft| {
        draft
            .get("setting")
            .is_some_and(|value| value == "notifications")
    }) {
        for field in FLAGS {
            values[field] = json!(draft.get(field).is_some_and(|value| value == "true"));
        }
        for field in TIMES {
            values[field] = json!(draft.get(field).cloned().unwrap_or_default());
        }
    }
    let mut managed = serde_json::to_value(notification_preferences::managed::read(tenant).await?)
        .map_err(|_| OperationError::Unavailable)?;
    if let Some(people) = managed.as_array_mut() {
        for person in people {
            person["label"] = json!(
                labels["profiles"]["notifications"]["managed_people"]["toggle_label"]
                    .as_str()
                    .unwrap_or("")
                    .replace("%{name}", person["name"].as_str().unwrap_or(""))
            );
            if let Some(draft) = draft.filter(|draft| {
                draft
                    .get("setting")
                    .is_some_and(|value| value == "notifications")
            }) {
                person["enabled"] = json!(
                    draft
                        .get(&format!("managed_{}", person["id"]))
                        .is_some_and(|value| value == "true")
                );
            }
        }
    }
    values["managed_people"] = managed;
    values["category_count"] = json!(
        FLAGS[1..4]
            .iter()
            .filter(|field| values[**field].as_bool().unwrap_or(false))
            .count()
            .to_string()
    );
    Ok(values)
}

fn selected_people(
    draft: &HashMap<String, String>,
) -> std::result::Result<Vec<i64>, OperationError> {
    draft
        .iter()
        .filter(|(key, _)| key.starts_with("managed_"))
        .map(|(key, value)| {
            if value != "true" {
                return Err(profile::validation("managed_person_ids", "is invalid"));
            }
            key.trim_start_matches("managed_")
                .parse::<i64>()
                .ok()
                .filter(|id| *id > 0)
                .ok_or_else(|| profile::validation("managed_person_ids", "is invalid"))
        })
        .collect()
}

fn parse(
    draft: &HashMap<String, String>,
) -> std::result::Result<notification_preferences::Changes, OperationError> {
    if draft.keys().any(|key| {
        !FLAGS.contains(&key.as_str())
            && !TIMES.contains(&key.as_str())
            && !["authenticity_token", "setting"].contains(&key.as_str())
            && !key.starts_with("managed_")
    }) {
        return Err(profile::validation("notification_preference", "is invalid"));
    }
    let mut fields = serde_json::Map::new();
    for field in FLAGS {
        if draft.get(field).is_some_and(|value| value != "true") {
            return Err(profile::validation(field, "is invalid"));
        }
        fields.insert(field.into(), json!(draft.contains_key(field)));
    }
    for field in TIMES {
        let value = draft
            .get(field)
            .ok_or_else(|| profile::validation(field, "is required"))?;
        fields.insert(
            field.into(),
            if value.is_empty() {
                Value::Null
            } else {
                json!(value)
            },
        );
    }
    notification_preferences::parse(&json!({"notification_preference":fields}))
}

pub(super) async fn save(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    (headers, request): (HeaderMap, Option<Extension<LocoRequestId>>),
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(mut draft): Form<HashMap<String, String>>,
) -> Response {
    if token
        .verify(
            draft
                .get("authenticity_token")
                .map(String::as_str)
                .unwrap_or(""),
        )
        .is_err()
    {
        return operation_error(OperationError::Forbidden);
    }
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let result = async {
        notification_preferences::managed::update(
            &tenant,
            principal.account_id(),
            parse(&draft)?,
            &selected_people(&draft)?,
        )
        .await
    }
    .await;
    match result {
        Ok(_) => {
            if tenant.commit().await.is_err() {
                return unavailable();
            }
            (
                StatusCode::SEE_OTHER,
                [
                    (
                        header::LOCATION,
                        format!("/households/{slug}/profile#notifications"),
                    ),
                    (header::CACHE_CONTROL, "no-store".into()),
                ],
            )
                .into_response()
        }
        Err(OperationError::Validation { .. }) => {
            if tenant.rollback().await.is_err() {
                return unavailable();
            }
            draft.insert("setting".into(), "notifications".into());
            Page {
                ctx: &ctx,
                slug: &slug,
                session: &session,
                request_id,
                headers: &headers,
                token: &token,
                view: &view,
            }
            .render(Some(draft))
            .await
        }
        Err(error) => operation_error(error),
    }
}
