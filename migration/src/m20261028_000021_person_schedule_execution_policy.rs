use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SeaQuery 1.0 has no ALTER POLICY/RLS builders. Keep only this unsupported
        // policy DDL isolated; runtime queries and clinical writes remain SeaORM.
        manager
            .get_connection()
            .execute_unprepared(include_str!("../person-schedule-execution-policy.sql"))
            .await?;
        Ok(())
    }
}
