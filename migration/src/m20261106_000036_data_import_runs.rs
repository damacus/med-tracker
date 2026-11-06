use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut table = Table::create();
        table.table(Alias::new("data_import_runs"));
        table.col(
            ColumnDef::new(Alias::new("id"))
                .string()
                .not_null()
                .primary_key(),
        );
        table.col(
            ColumnDef::new(Alias::new("household_id"))
                .big_integer()
                .not_null(),
        );
        table.col(
            ColumnDef::new(Alias::new("created_by_membership_id"))
                .big_integer()
                .not_null(),
        );
        table.col(
            ColumnDef::new(Alias::new("source_timezone"))
                .string()
                .not_null(),
        );
        table.col(
            ColumnDef::new(Alias::new("encrypted_prepared"))
                .binary()
                .not_null(),
        );
        for name in [
            "report",
            "receipt",
            "mappings",
            "consents",
            "acknowledged_omissions",
        ] {
            table.col(ColumnDef::new(Alias::new(name)).json_binary().not_null());
        }
        table.col(ColumnDef::new(Alias::new("confirmation")).string());
        for (column, target) in [
            ("household_id", "households"),
            ("created_by_membership_id", "household_memberships"),
        ] {
            table.foreign_key(
                ForeignKey::create()
                    .from(Alias::new("data_import_runs"), Alias::new(column))
                    .to(Alias::new(target), Alias::new("id")),
            );
        }
        manager.create_table(table.to_owned()).await?;
        // SeaQuery has no PostgreSQL RLS policy or role GRANT builders.
        manager.get_connection().execute_unprepared("ALTER TABLE public.data_import_runs ENABLE ROW LEVEL SECURITY; ALTER TABLE public.data_import_runs FORCE ROW LEVEL SECURITY; CREATE POLICY data_import_select ON public.data_import_runs FOR SELECT USING (household_id = med_tracker.current_household_id()); CREATE POLICY data_import_insert ON public.data_import_runs FOR INSERT WITH CHECK (household_id = med_tracker.current_household_id() AND created_by_membership_id = med_tracker.current_membership_id()); CREATE POLICY data_import_update ON public.data_import_runs FOR UPDATE USING (household_id = med_tracker.current_household_id()) WITH CHECK (household_id = med_tracker.current_household_id()); GRANT SELECT, INSERT ON public.data_import_runs TO med_tracker_app; GRANT UPDATE (report, confirmation) ON public.data_import_runs TO med_tracker_app;").await?;
        Ok(())
    }
}
