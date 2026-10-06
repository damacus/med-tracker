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
