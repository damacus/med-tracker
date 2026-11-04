use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SeaORM's SchemaManager and SeaQuery do not provide PostgreSQL CREATE OR REPLACE FUNCTION.
        // Keep this function DDL isolated; runtime queries remain SeaORM.
        let db = manager.get_connection();
        db.execute_unprepared("SET LOCAL ROLE NONE").await?;
        db.execute_unprepared(include_str!("../guardian-authority-eligibility.sql"))
            .await?;
        db.execute_unprepared("SET LOCAL ROLE med_tracker_owner")
            .await?;
        Ok(())
    }
}
