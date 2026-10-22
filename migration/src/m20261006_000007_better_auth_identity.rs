use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        crate::native_authentication::create(manager).await?;
        // SeaORM has no builders for PostgreSQL roles, RLS policies or trigger bodies.
        manager
            .get_connection()
            .execute_unprepared(include_str!("../native-authentication.sql"))
            .await?;
        Ok(())
    }
}
