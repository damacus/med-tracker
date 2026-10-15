use migration::{Migrator, MigratorTrait, sea_orm::Database};
#[path = "generate_entities/citext.rs"]
mod citext;
#[path = "generate_entities/relations.rs"]
mod relations;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("MEDTRACKER_OWNED_DATABASE").as_deref() != Ok("1") {
        return Err("Entity generation requires a disposable owned database".into());
    }
    let database = Database::connect(std::env::var("DATABASE_URL")?).await?;
    Migrator::up(&database, None).await?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("Missing workspace directory")?;
    let output = root.join("src/models/_entities");
    let status = std::process::Command::new("sea-orm-cli")
        .args(["generate", "entity", "--compact-format", "--with-serde", "both", "--output-dir"])
        .arg(&output)
        .args(["--ignore-tables", "seaql_migrations,account_active_session_keys,account_login_change_keys,account_password_reset_keys,account_remember_keys,account_verification_keys,pg_loco_queue"])
        .status()?;
    if !status.success() {
        return Err("SeaORM entity generation failed".into());
    }
    relations::reconcile(&database, &output).await?;
    citext::reconcile(&database, &output).await?;
    database.close().await?;
    for table in [
        "users",
        "account_active_session_keys",
        "account_login_change_keys",
        "account_password_reset_keys",
        "account_remember_keys",
        "account_verification_keys",
        "pg_loco_queue",
    ] {
        let removed = output.join(format!("{table}.rs"));
        if removed.exists() {
            std::fs::remove_file(removed)?;
        }
    }
    Ok(())
}
