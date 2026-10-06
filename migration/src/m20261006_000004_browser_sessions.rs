use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE public.browser_sessions (id VARCHAR(128) NOT NULL PRIMARY KEY, expires BIGINT NULL, session TEXT NOT NULL); REVOKE ALL ON public.browser_sessions FROM PUBLIC, med_tracker_app, med_tracker_audit_exporter, med_tracker_audit_verifier; GRANT SELECT, INSERT, UPDATE, DELETE ON public.browser_sessions TO med_tracker_app",
        ).await?;
        Ok(())
    }
}
