use super::*;
use crate::models::ids::{AccountId, HouseholdId, PersonId};
use sha2::Digest;

pub struct AccountProfile {
    pub email: String,
    pub name: String,
    pub date_of_birth: NaiveDate,
    pub invitation_token: Option<String>,
}

pub struct ProvisionedAccount {
    pub account_id: AccountId,
    pub person_id: PersonId,
    pub household_id: HouseholdId,
    pub email: String,
}

pub async fn provision_account_in(
    transaction: &DatabaseTransaction,
    profile: &AccountProfile,
    request_id: &str,
    settings: Option<&serde_json::Value>,
) -> Result<ProvisionedAccount, SignupError> {
    provision_with_password_in(transaction, profile, None, Some(request_id), settings).await
}

pub(super) async fn provision_with_password_in(
    transaction: &DatabaseTransaction,
    profile: &AccountProfile,
    password_hash: Option<&str>,
    request_id: Option<&str>,
    settings: Option<&serde_json::Value>,
) -> Result<ProvisionedAccount, SignupError> {
    let errors = profile_errors(&profile.name, Some(profile.date_of_birth), false);
    if !errors.is_empty() {
        return Err(SignupError::Invalid(errors));
    }
    let invitation_token = profile
        .invitation_token
        .as_deref()
        .filter(|token| !token.is_empty());
    let invitation = if let Some(token) = invitation_token {
        Some(
            invitations::signup_invitation(transaction, token)
                .await
                .map_err(operation_error)?,
        )
    } else {
        if !policy::open(transaction, settings).await? {
            return Err(SignupError::RegistrationClosed);
        }
        None
    };
    let email = match &invitation {
        Some(invitation) => invitation.email.clone(),
        None => invitations::validated_email(&profile.email).map_err(operation_error)?,
    };
    let preferences = initial_preferences(settings)?;
    let account = if let Some(token) = invitation_token {
        transaction
            .query_one_raw(sql(
                "SELECT * FROM med_tracker.bootstrap_invited_registration($1,$2,$3,$4,$5)",
                [
                    hex::encode(sha2::Sha256::digest(token.as_bytes())).into(),
                    profile.name.trim().to_owned().into(),
                    profile.date_of_birth.into(),
                    password_hash.map(str::to_owned).into(),
                    request_id.map(str::to_owned).into(),
                ],
            ))
            .await
    } else {
        let name = format!("{} Household", profile.name.trim());
        let mut suffix = [0u8; 4];
        getrandom::fill(&mut suffix).map_err(unavailable)?;
        let slug = format!("{}-{}", slug::slugify(&name), hex::encode(suffix));
        let timezone = super::super::time_zone::preferred(&serde_json::json!({}), settings)
            .map_err(unavailable)?
            .to_string();
        transaction
            .query_one_raw(sql(
                "SELECT * FROM med_tracker.bootstrap_registration($1,$2,$3,$4,$5,$6,$7,$8)",
                [
                    email.clone().into(),
                    profile.name.trim().to_owned().into(),
                    profile.date_of_birth.into(),
                    password_hash.map(str::to_owned).into(),
                    name.into(),
                    slug.into(),
                    timezone.into(),
                    request_id.map(str::to_owned).into(),
                ],
            ))
            .await
    }
    .map_err(provisioning_failure)?
    .ok_or(SignupError::Unavailable)?;
    let account_id =
        AccountId::from_database(account.try_get("", "account_id").map_err(unavailable)?);
    let person_id = PersonId::from_database(account.try_get("", "person_id").map_err(unavailable)?);
    transaction
        .execute_raw(sql(
            "SELECT set_config('med_tracker.current_account_id',$1,true)",
            [format!("{account_id}").into()],
        ))
        .await
        .map_err(unavailable)?;
    transaction
        .execute_raw(sql(
            "UPDATE accounts SET preferences=$2 WHERE id=$1",
            [account_id.get().into(), preferences.into()],
        ))
        .await
        .map_err(unavailable)?;
    let household_id = HouseholdId::from_database(match &invitation {
        Some(invitation) => invitation.household_id,
        None => account.try_get("", "household_id").map_err(unavailable)?,
    });
    Ok(ProvisionedAccount {
        account_id,
        person_id,
        household_id,
        email,
    })
}

fn provisioning_failure(error: sea_orm::DbErr) -> SignupError {
    if let sea_orm::DbErr::Query(sea_orm::RuntimeErr::SqlxError(ref source)) = error
        && source.as_database_error().is_some_and(|database| {
            database.code().as_deref() == Some("23505")
                && database.constraint() == Some("index_accounts_on_email")
        })
    {
        return invalid("email", "is already registered");
    }
    unavailable(error)
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
