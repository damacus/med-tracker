use super::*;

#[derive(serde::Serialize)]
struct RequestDigest<'a> {
    method: &'a str,
    path: &'a str,
    params: Value,
}

pub(super) fn digest(
    tenant: &TenantTransaction,
    method: &str,
    path: &str,
    body: &Value,
) -> Option<String> {
    let mut params = if method == "DELETE" && body.is_null() {
        serde_json::Map::new()
    } else {
        body.as_object()?.clone()
    };
    let household = tenant.scope().household_id;
    let locations_prefix = format!("/api/v1/households/{household}/locations/");
    if let Some(rest) = path.strip_prefix(&locations_prefix) {
        let mut parts = rest.split('/');
        let location_id = parts.next().unwrap_or_default();
        if parts.next() != Some("location_memberships") {
            return None;
        }
        let member_id = parts.next();
        if location_id.parse::<i64>().is_err()
            || member_id.is_some_and(|id| id.parse::<i64>().is_err())
            || parts.next().is_some()
        {
            return None;
        }
        if member_id.is_none() {
            if params.keys().any(|key| key != "location_membership") {
                return None;
            }
            let attributes = params.get("location_membership")?.as_object()?;
            if attributes.keys().any(|key| key != "person_id") {
                return None;
            }
        }
        params.insert("household_id".into(), json!(household.to_string()));
        params.insert("location_id".into(), json!(location_id));
        if let Some(id) = member_id {
            params.insert("id".into(), json!(id));
        }
        let request = RequestDigest {
            method,
            path,
            params: Value::Object(params),
        };
        return serde_json::to_vec(&request)
            .ok()
            .map(|bytes| hex::encode(Sha256::digest(bytes)));
    }
    let collections = [
        (
            "location",
            format!("/api/v1/households/{household}/locations"),
        ),
        ("person", format!("/api/v1/households/{household}/people")),
        (
            "household",
            format!("/api/v1/households/{household}/admin/settings"),
        ),
        (
            "household_membership",
            format!("/api/v1/households/{household}/admin/memberships"),
        ),
        (
            "person_access_grant",
            format!("/api/v1/households/{household}/admin/person_access_grants"),
        ),
    ];
    let (envelope, collection) = collections.into_iter().find(|(envelope, collection)| {
        path == collection
            || (*envelope != "household" && path.starts_with(&(collection.clone() + "/")))
    })?;
    if body.is_null()
        && (!matches!(envelope, "household_membership" | "person_access_grant")
            || !path.starts_with(&(collection.clone() + "/")))
    {
        return None;
    }
    if params.keys().any(|key| key != envelope) {
        return None;
    }
    if let Some(attributes) = params.get_mut(envelope) {
        let attributes = attributes.as_object_mut()?;
        let allowed: &[&str] = match envelope {
            "location" => &["name", "description"],
            "person" => &[
                "name",
                "date_of_birth",
                "email",
                "person_type",
                "has_capacity",
            ],
            "household" => &["name", "timezone", "subscription_plan"],
            "household_membership" => &["role", "status", "person_id"],
            "person_access_grant" => &[
                "household_membership_id",
                "person_id",
                "access_level",
                "relationship_type",
                "expires_at",
            ],
            _ => return None,
        };
        if attributes
            .keys()
            .any(|key| !allowed.contains(&key.as_str()))
        {
            return None;
        }
        if envelope == "person" && attributes.contains_key("email") {
            attributes.insert("email".into(), json!("[FILTERED]"));
        }
    }
    params.insert(
        "household_id".into(),
        json!(tenant.scope().household_id.to_string()),
    );
    let prefix = collection + "/";
    if let Some(id) = path.strip_prefix(&prefix) {
        params.insert("id".into(), json!(id));
    }
    let request = RequestDigest {
        method,
        path,
        params: Value::Object(params),
    };
    serde_json::to_vec(&request)
        .ok()
        .map(|bytes| hex::encode(Sha256::digest(bytes)))
}
