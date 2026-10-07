use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::Deserialize;

use crate::models::identity::signup::AccountProfile;

pub(super) type Errors = BTreeMap<String, Vec<String>>;

#[derive(Deserialize)]
pub(super) struct InvitationQuery {
    #[serde(default, alias = "token")]
    pub invitation_token: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct Create {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub date_of_birth: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub credential: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub invitation_token: Option<String>,
    #[serde(default)]
    pub authenticity_token: String,
}

impl Create {
    pub(super) fn empty(invitation_token: Option<String>) -> Self {
        Self { name: String::new(), date_of_birth: String::new(), email: String::new(), credential: "password".into(), password: String::new(), invitation_token, authenticity_token: String::new() }
    }

    pub(super) fn profile(&self) -> Result<AccountProfile, Errors> {
        let date_of_birth = NaiveDate::parse_from_str(&self.date_of_birth, "%Y-%m-%d").map_err(|_| {
            let message = if self.date_of_birth.trim().is_empty() { "Date of birth must be present" } else { "Date of birth must be a valid date" };
            BTreeMap::from([("date_of_birth".to_owned(), vec![message.to_owned()])])
        })?;
        Ok(AccountProfile { name: self.name.clone(), email: self.email.clone(), date_of_birth, invitation_token: self.invitation_token.clone() })
    }
}
