use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SeaQuery 1.0 SchemaStatement has table/index/foreign-key builders, but no
        // CREATE OR REPLACE FUNCTION builder. Keep this static function DDL here.
        manager
            .get_connection()
            .execute_unprepared(include_str!("../history-audit-outcome.sql"))
            .await?;
        Ok(())
    }
}
