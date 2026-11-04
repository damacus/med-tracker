use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SeaORM's SchemaManager and SeaQuery do not provide PostgreSQL ALTER POLICY.
        // Keep this policy DDL isolated; runtime queries remain SeaORM.
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../shared-schedule-current-execution-policy.sql"
            ))
            .await?;
        Ok(())
    }
}
