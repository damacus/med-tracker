use migration::sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::{error::Error, path::Path};

pub async fn reconcile(database: &DatabaseConnection, output: &Path) -> Result<(), Box<dyn Error>> {
    let columns = database
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT table_name,column_name FROM information_schema.columns
         WHERE table_schema='public' AND udt_schema='public' AND udt_name='citext'
         ORDER BY table_name,ordinal_position",
        ))
        .await?;
    for column in columns {
        let table: String = column.try_get("", "table_name")?;
        let column: String = column.try_get("", "column_name")?;
        let path = output.join(format!("{table}.rs"));
        let input = std::fs::read_to_string(&path)?;
        let original = format!(
            "#[sea_orm(ignore, column_type = \"custom(\\\"citext\\\")\", select_as = \"text\")]\n    pub {column}: String,"
        );
        let persisted = format!(
            "#[sea_orm(\n        column_type = \"custom(\\\"citext\\\")\",\n        select_as = \"text\",\n        save_as = \"citext\"\n    )]\n    pub {column}: String,"
        );
        if input.matches(&original).count() != 1 {
            return Err(format!("Expected one generated citext field for {table}.{column}").into());
        }
        std::fs::write(path, input.replace(&original, &persisted))?;
    }
    Ok(())
}
