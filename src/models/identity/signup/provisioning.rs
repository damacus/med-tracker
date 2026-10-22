use super::*;

pub struct AccountProfile {
    pub email: String,
    pub name: String,
    pub date_of_birth: NaiveDate,
    pub invitation_token: Option<String>,
}

pub struct ProvisionedAccount {
    pub account_id: i64,
    pub person_id: i64,
    pub user_id: i64,
    pub household_id: i64,
    pub email: String,
}

pub(crate) fn profile_errors(
    name: &str,
    birth: Option<NaiveDate>,
    missing_birth: bool,
) -> BTreeMap<String, Vec<String>> {
    let mut errors = BTreeMap::<String, Vec<String>>::new();
    if name.trim().is_empty() {
        errors
            .entry("name".into())
            .or_default()
            .push("must be present".into());
    }
    let message = match birth {
        None if missing_birth => Some("must be present"),
        None => Some("must be a valid date"),
        Some(birth)
            if Utc::now()
                .date_naive()
                .checked_sub_months(Months::new(18 * 12))
                .is_none_or(|latest| birth > latest) =>
        {
            Some("Children must be added by a parent or carer.")
        }
        _ => None,
    };
    if let Some(message) = message {
        errors
            .entry("date_of_birth".into())
            .or_default()
            .push(message.into());
    }
    errors
}
