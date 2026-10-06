use sea_orm_migration::prelude::*;

pub const CANONICAL_CLIENT_UUID_SQL: &str = "CASE WHEN client_uuid ~* '^([0-9a-f]{32}|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}|\\{[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\\}|urn:uuid:[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$' THEN lower(replace(replace(replace(regexp_replace(client_uuid, '^urn:uuid:', '', 'i'), '-', ''), '{', ''), '}', '')) ELSE NULL END";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let connection = manager.get_connection();
        connection.execute_unprepared(&format!("CREATE UNIQUE INDEX index_medication_takes_on_client_uuid_canonical ON public.medication_takes (({CANONICAL_CLIENT_UUID_SQL}))")).await?;
        Ok(())
    }
}
