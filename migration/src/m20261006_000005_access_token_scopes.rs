use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("ALTER TABLE public.oauth_grants ADD COLUMN access_token_scopes text, ADD COLUMN access_token_scope_hash text").await?;
        Ok(())
    }
}
