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

pub async fn provision_account_in(
    transaction: &DatabaseTransaction,
    profile: &AccountProfile,
    request_id: &str,
) -> Result<ProvisionedAccount, SignupError> {
    provision_with_password_in(transaction, profile, None, Some(request_id)).await
}

pub(super) async fn provision_with_password_in(
    transaction: &DatabaseTransaction,
    profile: &AccountProfile,
    password_hash: Option<&str>,
    request_id: Option<&str>,
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
        if !policy::open(transaction).await? {
            return Err(SignupError::RegistrationClosed);
        }
        None
    };
    let email = match &invitation {
        Some(invitation) => invitation.email.clone(),
        None => invitations::validated_email(&profile.email).map_err(operation_error)?,
    };
    let preferences = initial_preferences()?;
    let account = transaction.query_one_raw(sql(
        "INSERT INTO accounts(email,password_hash,status,preferences,created_at,updated_at) VALUES($1,$2,1,$3,now(),now()) ON CONFLICT(email) WHERE status = ANY(ARRAY[1,2]) DO NOTHING RETURNING id",
        [email.clone().into(),password_hash.map(str::to_owned).into(),preferences.into()])).await.map_err(unavailable)?
        .ok_or_else(|| invalid("email", "is already registered"))?;
    let account_id: i64 = account.try_get("", "id").map_err(unavailable)?;
    transaction
        .execute_raw(sql(
            "SELECT set_config('med_tracker.current_account_id',$1,true)",
            [format!("{account_id}").into()],
        ))
        .await
        .map_err(unavailable)?;
    let household_id = match &invitation {
        Some(invitation) => invitation.household_id,
        None => bootstrap::household(transaction, account_id, &profile.name).await?,
    };
    let person = transaction.query_one_raw(sql(
        "INSERT INTO people(account_id,household_id,name,email,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES($1,$2,$3,$4,$5,0,true,now(),now()) RETURNING id",
        [account_id.into(),household_id.into(),profile.name.trim().to_owned().into(),email.clone().into(),profile.date_of_birth.into()])).await.map_err(unavailable)?.ok_or(SignupError::Unavailable)?;
    let person_id: i64 = person.try_get("", "id").map_err(unavailable)?;
    let user = transaction.query_one_raw(sql(
        "INSERT INTO users(person_id,email_address,password_digest,active,created_at,updated_at) VALUES($1,$2,$3,true,now(),now()) RETURNING id",
        [person_id.into(),email.clone().into(),password_hash.map(str::to_owned).into()])).await.map_err(unavailable)?.ok_or(SignupError::Unavailable)?;
    let user_id: i64 = user.try_get("", "id").map_err(unavailable)?;
    let context = SignupInvitationContext {
        account_id,
        person_id,
        user_id,
        email: email.clone(),
    };
    if let Some(token) = invitation_token {
        invitations::accept_signup(transaction, &context, token, request_id.unwrap_or(""))
            .await
            .map_err(operation_error)?;
    } else {
        bootstrap::owner(
            transaction,
            &context,
            household_id,
            &profile.name,
            request_id,
        )
        .await?;
    }
    Ok(ProvisionedAccount {
        account_id,
        person_id,
        user_id,
        household_id,
        email,
    })
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
