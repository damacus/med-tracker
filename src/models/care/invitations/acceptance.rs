use super::*;
use crate::models::{
    entities::membership, identity::api_session::ApiSessionPrincipal,
    identity::browser::BrowserPrincipal,
};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, Statement,
    TransactionTrait,
};

pub async fn accept(
    db: &DatabaseConnection,
    principal: &ApiSessionPrincipal,
    token: &str,
    request_id: &str,
    settings: Option<&serde_json::Value>,
) -> Result<Value, OperationError> {
    accept_validated(db, Principal::Api(principal), token, request_id, settings).await
}

pub async fn accept_browser(
    db: &DatabaseConnection,
    principal: &BrowserPrincipal,
    token: &str,
    request_id: &str,
    settings: Option<&serde_json::Value>,
) -> Result<Value, OperationError> {
    accept_validated(
        db,
        Principal::Browser(principal),
        token,
        request_id,
        settings,
    )
    .await
}

pub(crate) struct AcceptanceActor {
    pub person_id: crate::models::ids::PersonId,
}

enum Principal<'a> {
    Api(&'a ApiSessionPrincipal),
    Browser(&'a BrowserPrincipal),
}

impl Principal<'_> {
    async fn revalidate(
        &self,
        transaction: &DatabaseTransaction,
        settings: Option<&serde_json::Value>,
    ) -> Result<AcceptanceActor, OperationError> {
        match self {
            Self::Api(principal) => {
                let actor = principal
                    .revalidate(transaction, settings)
                    .await
                    .map_err(authentication)?;
                Ok(AcceptanceActor {
                    person_id: crate::models::ids::PersonId::from_database(actor.person_id),
                })
            }
            Self::Browser(principal) => {
                let actor = principal
                    .revalidate_invitation(transaction)
                    .await
                    .map_err(authentication)?;
                Ok(AcceptanceActor {
                    person_id: crate::models::ids::PersonId::from_database(actor.person_id),
                })
            }
        }
    }
}

async fn accept_validated(
    db: &DatabaseConnection,
    principal: Principal<'_>,
    token: &str,
    request_id: &str,
    settings: Option<&serde_json::Value>,
) -> Result<Value, OperationError> {
    if token.trim().is_empty() {
        return Err(unavailable());
    }
    let transaction = db.begin().await?;
    transaction
        .execute_unprepared("SET LOCAL ROLE med_tracker_app")
        .await?;
    let actor = principal.revalidate(&transaction, settings).await?;
    let row = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.accept_household_invitation($1,$2)",
            [tokens::digest(token).into(), request_id.into()],
        ))
        .await
        .map_err(|error| {
            if let sea_orm::DbErr::Query(sea_orm::RuntimeErr::SqlxError(ref source)) = error
                && source
                    .as_database_error()
                    .is_some_and(|database| database.code().as_deref() == Some("MT001"))
            {
                unavailable()
            } else {
                OperationError::from(error)
            }
        })?
        .ok_or_else(unavailable)?;
    let household_id =
        crate::models::ids::HouseholdId::from_database(row.try_get("", "household_id")?);
    let person_id = crate::models::ids::PersonId::from_database(row.try_get("", "person_id")?);
    if person_id != actor.person_id {
        return Err(unavailable());
    }
    let membership_id: i64 = row.try_get("", "membership_id")?;
    let member = membership::Entity::find_by_id(membership_id)
        .one(&transaction)
        .await?
        .ok_or_else(unavailable)?;
    transaction.commit().await?;
    Ok(
        json!({"data":{"household_id":household_id.to_string(),"membership_id":membership_id.to_string(),"person_id":person_id.to_string(),"role":member.role}}),
    )
}

pub(super) fn unavailable() -> OperationError {
    invalid("base", "Invitation unavailable")
}

fn authentication(error: crate::models::identity::resource::AuthenticationError) -> OperationError {
    use crate::models::identity::resource::AuthenticationError;
    match error {
        AuthenticationError::Unauthenticated => OperationError::Unauthenticated,
        AuthenticationError::Forbidden | AuthenticationError::InsufficientScope { .. } => {
            OperationError::Forbidden
        }
        AuthenticationError::Unavailable => OperationError::Unavailable,
    }
}
