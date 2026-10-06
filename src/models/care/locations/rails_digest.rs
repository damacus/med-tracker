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
    let mut params = body.as_object()?.clone();
    let household = tenant.scope().household_id;
    let location_path = format!("/api/v1/households/{household}/locations");
    let person_path = format!("/api/v1/households/{household}/people");
    let (envelope, collection) =
        if path == location_path || path.starts_with(&(location_path.clone() + "/")) {
            ("location", location_path)
        } else if path == person_path || path.starts_with(&(person_path.clone() + "/")) {
            ("person", person_path)
        } else {
            return None;
        };
    if params.keys().any(|key| key != envelope) {
        return None;
    }
    if let Some(attributes) = params.get_mut(envelope) {
        let attributes = attributes.as_object_mut()?;
        if attributes.keys().any(|key| {
            if envelope == "location" {
                key != "name" && key != "description"
            } else {
                !matches!(
                    key.as_str(),
                    "name" | "date_of_birth" | "email" | "person_type" | "has_capacity"
                )
            }
        }) {
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
