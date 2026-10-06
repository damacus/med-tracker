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
    if params.keys().any(|key| key != "location") {
        return None;
    }
    if let Some(attributes) = params.get("location")
        && attributes
            .as_object()?
            .keys()
            .any(|key| key != "name" && key != "description")
    {
        return None;
    }
    params.insert(
        "household_id".into(),
        json!(tenant.scope().household_id.to_string()),
    );
    let prefix = format!(
        "/api/v1/households/{}/locations/",
        tenant.scope().household_id
    );
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
