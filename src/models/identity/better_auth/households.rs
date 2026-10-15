use better_auth_core::{AuthError, AuthRequest, AuthResult, store::SessionStore};
use sea_orm::ConnectionTrait;

use super::{
    IdentityService, request,
    store::{database_error, statement},
};
use crate::models::ids::HouseholdId;

pub struct NewHousehold {
    name: String,
    timezone: chrono_tz::Tz,
}

impl NewHousehold {
    pub fn parse(name: &str, timezone: &str) -> Option<Self> {
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        Some(Self {
            name: name.to_owned(),
            timezone: timezone.parse().ok()?,
        })
    }
}

pub struct CreatedHousehold {
    pub id: HouseholdId,
    pub slug: String,
}

pub async fn create(
    service: &IdentityService,
    auth: &AuthRequest,
    input: NewHousehold,
    request_id: String,
) -> AuthResult<CreatedHousehold> {
    request::trusted(service, auth, request_id, async {
        let actor = request::authenticated()?;
        let transaction = service.store.account_transaction().await?;
        let slug = format!("household-{}", uuid::Uuid::new_v4());
        let row = transaction
            .query_one_raw(statement(
                "SELECT * FROM med_tracker.create_household($1,$2,$3,$4)",
                [
                    input.name.into(),
                    slug.clone().into(),
                    input.timezone.to_string().into(),
                    actor.request_id.into(),
                ],
            ))
            .await
            .map_err(creation_error)?
            .ok_or_else(|| AuthError::internal("Household creation unavailable"))?;
        let id =
            HouseholdId::from_database(row.try_get("", "household_id").map_err(database_error)?);
        service
            .store
            .update_session_active_organization(&actor.session_token, Some(&id.to_string()))
            .await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(CreatedHousehold { id, slug })
    })
    .await
}

fn creation_error(error: sea_orm::DbErr) -> AuthError {
    if let sea_orm::DbErr::Query(sea_orm::RuntimeErr::SqlxError(ref source)) = error
        && source
            .as_database_error()
            .is_some_and(|database| database.code().as_deref() == Some("MT001"))
    {
        return AuthError::forbidden("Household creation requires an eligible adult account");
    }
    database_error(error)
}
