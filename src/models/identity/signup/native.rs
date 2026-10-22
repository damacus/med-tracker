use super::*;

pub(crate) async fn prepare_native_profile(
    transaction: &impl ConnectionTrait,
    profile: &mut AccountProfile,
    settings: Option<&serde_json::Value>,
) -> Result<i64, SignupError> {
    let errors = profile_errors(&profile.name, Some(profile.date_of_birth), false);
    if !errors.is_empty() {
        return Err(SignupError::Invalid(errors));
    }
    profile.email = if let Some(token) = profile
        .invitation_token
        .as_deref()
        .filter(|t| !t.is_empty())
    {
        invitations::signup_invitation(transaction, token)
            .await
            .map_err(operation_error)?
            .email
    } else {
        if !policy::open(transaction, settings).await? {
            return Err(SignupError::RegistrationClosed);
        }
        invitations::validated_email(&profile.email).map_err(operation_error)?
    };
    let row = transaction
        .query_one_raw(sql(
            "SELECT nextval(pg_get_serial_sequence('public.people','id')) AS id",
            [],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(SignupError::Unavailable)?;
    row.try_get("", "id").map_err(unavailable)
}

pub(crate) async fn complete_native_profile(
    transaction: &impl ConnectionTrait,
    account_id: i64,
    profile: &AccountProfile,
    request_id: Option<&str>,
    settings: Option<&serde_json::Value>,
) -> Result<ProvisionedAccount, SignupError> {
    transaction
        .execute_raw(sql(
            "SELECT set_config('med_tracker.current_account_id',$1,true)",
            [account_id.to_string().into()],
        ))
        .await
        .map_err(unavailable)?;
    let row = transaction
        .query_one_raw(sql(
            "SELECT person_id FROM accounts WHERE id=$1 AND email=$2 AND status=1",
            [account_id.into(), profile.email.clone().into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(SignupError::Unavailable)?;
    let person_id: i64 = row.try_get("", "person_id").map_err(unavailable)?;
    let invitation = if let Some(token) = profile
        .invitation_token
        .as_deref()
        .filter(|t| !t.is_empty())
    {
        Some(
            invitations::signup_invitation(transaction, token)
                .await
                .map_err(operation_error)?,
        )
    } else {
        None
    };
    let household_id = match invitation {
        Some(invitation) => invitation.household_id,
        None => bootstrap::household(transaction, account_id, &profile.name, settings).await?,
    };
    transaction.execute_raw(sql(
        "INSERT INTO people(id,name,email,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES($1,$2,$3,$4,0,true,now(),now())",
        [person_id.into(), profile.name.trim().to_owned().into(), profile.email.clone().into(), profile.date_of_birth.into()],
    )).await.map_err(unavailable)?;
    let user = transaction.query_one_raw(sql(
        "INSERT INTO users(person_id,email_address,active,created_at,updated_at) VALUES($1,$2,true,now(),now()) RETURNING id",
        [person_id.into(), profile.email.clone().into()],
    )).await.map_err(unavailable)?.ok_or(SignupError::Unavailable)?;
    let user_id: i64 = user.try_get("", "id").map_err(unavailable)?;
    let actor = SignupInvitationContext {
        account_id,
        person_id,
        user_id,
        email: profile.email.clone(),
    };
    if let Some(token) = profile
        .invitation_token
        .as_deref()
        .filter(|t| !t.is_empty())
    {
        invitations::accept_signup(transaction, &actor, token, request_id.unwrap_or(""))
            .await
            .map_err(operation_error)?;
    } else {
        bootstrap::owner(transaction, &actor, household_id, &profile.name, request_id).await?;
    }
    transaction
        .execute_raw(sql(
            "INSERT INTO identity_onboarding(account_id) VALUES($1)",
            [account_id.into()],
        ))
        .await
        .map_err(unavailable)?;
    Ok(ProvisionedAccount {
        account_id,
        person_id,
        user_id,
        household_id,
        email: profile.email.clone(),
    })
}
