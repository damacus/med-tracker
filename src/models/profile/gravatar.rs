use crate::models::{
    access::{self, TenantTransaction},
    entities::person,
    errors::OperationError,
};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

pub(crate) fn enabled(preferences: &Value) -> bool {
    preferences["gravatar_enabled"]
        .as_bool()
        .unwrap_or_else(|| {
            preferences["gravatar_enabled"]
                .as_str()
                .is_some_and(|value| matches!(value, "true" | "1"))
        })
}

pub fn for_account(
    account: &crate::models::entities::account::Model,
    uploaded: bool,
) -> Option<String> {
    if uploaded || !enabled(&account.preferences) {
        None
    } else {
        image_url(&account.email)
    }
}

pub async fn for_person(
    tenant: &TenantTransaction,
    person_id: i64,
    uploaded: bool,
) -> Result<Option<String>, OperationError> {
    access::require_participation(tenant, person_id).await?;
    if uploaded {
        return Ok(None);
    }
    let row = tenant
        .transaction()
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT med_tracker.person_avatar_digest($1) AS digest",
            [person_id.into()],
        ))
        .await?
        .ok_or(OperationError::Unavailable)?;
    Ok(row
        .try_get::<Option<String>>("", "digest")?
        .map(|digest| image_url_from_digest(&digest)))
}

pub async fn for_visible_people(
    tenant: &TenantTransaction,
    people: &[person::Model],
    uploaded: &HashSet<i64>,
) -> Result<HashMap<i64, String>, OperationError> {
    access::recheck(tenant).await?;
    let ids: Vec<i64> = people
        .iter()
        .filter(|person| !uploaded.contains(&person.id))
        .map(|person| person.id)
        .collect();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = tenant.transaction().query_all_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT id,med_tracker.person_avatar_digest(id) AS digest FROM people WHERE id=ANY($1::bigint[])",
        [ids.into()],
    )).await?;
    let mut urls = HashMap::new();
    for row in rows {
        if let Some(digest) = row.try_get::<Option<String>>("", "digest")? {
            urls.insert(row.try_get("", "id")?, image_url_from_digest(&digest));
        }
    }
    Ok(urls)
}

fn image_url_from_digest(digest: &str) -> String {
    format!("https://www.gravatar.com/avatar/{digest}?s=128&d=404&r=g")
}

fn image_url(email: &str) -> Option<String> {
    let email = email.trim().to_lowercase();
    if email.is_empty() {
        return None;
    }
    let digest = hex::encode(Sha256::digest(email.as_bytes()));
    let mut url = url::Url::parse(&format!("https://www.gravatar.com/avatar/{digest}"))
        .expect("Gravatar URL with a hex digest is valid");
    url.query_pairs_mut()
        .append_pair("s", "128")
        .append_pair("d", "404")
        .append_pair("r", "g");
    Some(url.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalises_email_using_the_gravatar_reference_vector() {
        assert_eq!(
            image_url(" MyEmailAddress@example.com ").unwrap(),
            "https://www.gravatar.com/avatar/84059b07d4be67b806386c0aad8070a23f18836bbaae342275dc0a83414c32ee?s=128&d=404&r=g"
        );
        assert_eq!(image_url("  "), None);
    }

    #[test]
    fn only_explicit_current_or_retained_opt_in_enables_gravatar() {
        for value in [
            serde_json::json!(true),
            serde_json::json!("true"),
            serde_json::json!("1"),
        ] {
            assert!(enabled(&serde_json::json!({"gravatar_enabled":value})));
        }
        for value in [
            Value::Null,
            serde_json::json!(false),
            serde_json::json!("false"),
            serde_json::json!("0"),
        ] {
            assert!(!enabled(&serde_json::json!({"gravatar_enabled":value})));
        }
    }
}
