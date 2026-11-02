use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SeaORM's SchemaManager and SeaQuery do not provide PostgreSQL ALTER FUNCTION SET.
        // Keep this function configuration DDL isolated; runtime queries remain SeaORM.
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER FUNCTION med_tracker.sharing_guardian(bigint) SET plan_cache_mode TO 'force_generic_plan';\
                 ALTER FUNCTION med_tracker.sharing_declaration_eligible(bigint,bigint) SET plan_cache_mode TO 'force_generic_plan';",
            )
            .await?;
        Ok(())
    }
}
