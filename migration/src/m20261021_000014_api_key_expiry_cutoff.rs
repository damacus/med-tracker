use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
CREATE FUNCTION public.identity_delete_expired_api_keys(cutoff timestamptz) RETURNS bigint
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
DECLARE removed bigint;
BEGIN
    IF cutoff IS NULL OR cutoff > statement_timestamp() THEN
        RAISE EXCEPTION 'Invalid API credential expiration cutoff' USING ERRCODE = '22023';
    END IF;
    WITH deleted AS (
        DELETE FROM public.identity_api_keys
        WHERE expires_at < cutoff AND expires_at < CURRENT_TIMESTAMP
        RETURNING 1
    ) SELECT count(*) INTO removed FROM deleted;
    RETURN removed;
END
$$;
REVOKE ALL ON FUNCTION public.identity_delete_expired_api_keys(timestamptz) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.identity_delete_expired_api_keys(timestamptz) TO med_tracker_app;
"#,
            )
            .await?;
        Ok(())
    }
}
