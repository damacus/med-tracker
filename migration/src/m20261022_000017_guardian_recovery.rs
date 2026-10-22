use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("SET LOCAL ROLE NONE").await?;
        // SchemaManager and SeaQuery cannot define PL/pgSQL security-definer bodies,
        // RLS policies or triggers. Their PostgreSQL-only authority DDL is isolated
        // in this migration; runtime queries and calls use SeaORM/SeaQuery.
        db.execute_unprepared(include_str!("../guardian-recovery.sql"))
            .await?;
        db.execute_unprepared("SET LOCAL ROLE med_tracker_owner")
            .await?;
        Ok(())
    }
}
