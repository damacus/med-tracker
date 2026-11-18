use crate::models::{
    entities::{account, platform_admin, version},
    errors::OperationError,
};
use loco_rs::{
    app::AppContext,
    task::{Task, TaskInfo, Vars},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
    sea_query::{Expr, Func, OnConflict, Query},
};
use serde_json::json;

pub struct BootstrapTask;

fn invalid(message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"error": message}),
    }
}

pub async fn appoint(
    database: &DatabaseConnection,
    email: &str,
    operator: &str,
    reason: &str,
) -> Result<bool, OperationError> {
    if email.trim().is_empty() || operator.trim().is_empty() || reason.trim().is_empty() {
        return Err(invalid("Email, operator and reason are required."));
    }
    let transaction = database.begin().await?;
    let query = Query::select()
        .expr_as(
            Func::cust("current_setting").arg(Expr::val("is_superuser")),
            "superuser",
        )
        .to_owned();
    let privilege = transaction
        .query_one_raw(transaction.get_database_backend().build(&query))
        .await?
        .ok_or(OperationError::Unavailable)?
        .try_get::<String>("", "superuser")?;
    if privilege != "on" {
        return Err(OperationError::Forbidden);
    }
    super::serialize(&transaction).await?;
    let target = account::Entity::find()
        .filter(account::Column::Email.eq(email.trim()))
        .lock_exclusive()
        .one(&transaction)
        .await?
        .ok_or(OperationError::NotFound)?;
    if !target.active || target.status != 2 {
        return Err(invalid("The account must be verified and enabled."));
    }
    let existing = platform_admin::Entity::find()
        .filter(platform_admin::Column::AccountId.eq(target.id))
        .one(&transaction)
        .await?;
    if existing.is_some_and(|admin| admin.status == "active") {
        transaction.commit().await?;
        return Ok(false);
    }
    if platform_admin::Entity::find()
        .inner_join(account::Entity)
        .filter(platform_admin::Column::Status.eq("active"))
        .filter(account::Column::Active.eq(true))
        .filter(account::Column::Status.eq(2))
        .count(&transaction)
        .await?
        > 0
    {
        return Err(invalid(
            "A system administrator already exists. Use Platform users to grant access.",
        ));
    }
    let now = chrono::Utc::now().naive_utc();
    platform_admin::Entity::insert(platform_admin::ActiveModel {
        account_id: Set(target.id),
        status: Set("active".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(platform_admin::Column::AccountId)
            .update_columns([
                platform_admin::Column::Status,
                platform_admin::Column::UpdatedAt,
            ])
            .to_owned(),
    )
    .exec(&transaction)
    .await?;
    version::ActiveModel {
        item_type: Set("Account".into()), item_id: Set(target.id),
        event: Set("platform/administrator/bootstrapped".into()),
        object: Set(Some(json!({"account_id":target.id,"grant":true}).to_string())),
        audit_context: Set(json!({"authentication_method":"operator_bootstrap","operator":operator.trim(),"reason":reason.trim(),"target_account_id":target.id})),
        created_at: Set(Some(now)), ..Default::default()
    }.insert(&transaction).await?;
    transaction.commit().await?;
    Ok(true)
}

#[async_trait::async_trait]
impl Task for BootstrapTask {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "platform-admin-bootstrap".into(),
            detail: "Appoint the first system administrator using operator database access".into(),
        }
    }

    async fn run(&self, ctx: &AppContext, vars: &Vars) -> loco_rs::Result<()> {
        let changed = appoint(&ctx.db, vars.cli_arg("email")?, vars.cli_arg("operator")?, vars.cli_arg("reason")?).await.map_err(|error| {
            let message = match &error {
                OperationError::Validation { details } => details["error"].as_str().unwrap_or("Invalid bootstrap request"),
                OperationError::Forbidden => "Bootstrap requires database operator credentials; never use these credentials for the web server",
                OperationError::NotFound => "The requested account does not exist",
                _ => "Administrator bootstrap failed; no change was committed",
            };
            loco_rs::Error::string(message)
        })?;
        println!(
            "System administrator {}",
            if changed {
                "appointed and audited"
            } else {
                "already active; no change needed"
            }
        );
        Ok(())
    }
}
