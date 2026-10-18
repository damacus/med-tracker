use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    entities::{account, person, user},
    errors::OperationError,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
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

pub async fn for_person(
    tenant: &TenantTransaction,
    person_id: i64,
    uploaded: bool,
) -> Result<Option<String>, OperationError> {
    access::require_person_access(tenant, person_id, PersonAccess::View).await?;
    if uploaded {
        return Ok(None);
    }
    let person = access::person_queries::household_people(tenant.scope())
        .filter(person::Column::Id.eq(person_id))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    let Some(account) = account::Entity::find()
        .filter(account::Column::PersonId.eq(person.id))
        .one(tenant.transaction())
        .await?
        .filter(|account| enabled(&account.preferences))
    else {
        return Ok(None);
    };
    if let Some(email) = person
        .email
        .as_deref()
        .filter(|email| !email.trim().is_empty())
    {
        return Ok(image_url(email));
    }
    let user = user::Entity::find()
        .filter(user::Column::PersonId.eq(person.id))
        .one(tenant.transaction())
        .await?;
    let email = user
        .as_ref()
        .map(|user| user.email_address.as_str())
        .filter(|email| !email.trim().is_empty())
        .unwrap_or(&account.email);
    Ok(image_url(email))
}

pub async fn for_visible_people(
    tenant: &TenantTransaction,
    people: &[person::Model],
    uploaded: &HashSet<i64>,
) -> Result<HashMap<i64, String>, OperationError> {
    let eligible: Vec<_> = people
        .iter()
        .filter(|person| !uploaded.contains(&person.id))
        .collect();
    if eligible.is_empty() {
        return Ok(HashMap::new());
    }
    let accounts: HashMap<_, _> = account::Entity::find()
        .filter(account::Column::PersonId.is_in(eligible.iter().map(|person| person.id)))
        .all(tenant.transaction())
        .await?
        .into_iter()
        .filter(|account| enabled(&account.preferences))
        .map(|account| (account.person_id, account))
        .collect();
    let users: HashMap<_, _> = user::Entity::find()
        .filter(user::Column::PersonId.is_in(eligible.iter().map(|person| person.id)))
        .order_by_asc(user::Column::Id)
        .all(tenant.transaction())
        .await?
        .into_iter()
        .fold(HashMap::new(), |mut users, user| {
            users.entry(user.person_id).or_insert(user);
            users
        });
    Ok(eligible
        .into_iter()
        .filter_map(|person| {
            let account = accounts.get(&person.id)?;
            let email = person
                .email
                .as_deref()
                .filter(|email| !email.trim().is_empty())
                .or_else(|| {
                    users
                        .get(&person.id)
                        .map(|user| user.email_address.as_str())
                })
                .filter(|email| !email.trim().is_empty())
                .unwrap_or(&account.email);
            image_url(email).map(|url| (person.id, url))
        })
        .collect())
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
