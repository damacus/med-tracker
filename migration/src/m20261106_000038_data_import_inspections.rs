use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut table = Table::create();
        table.table(Alias::new("data_import_inspections"));
        table.col(
            ColumnDef::new(Alias::new("id"))
                .string()
                .not_null()
                .primary_key(),
        );
        for name in ["household_id", "account_id", "membership_id"] {
            table.col(ColumnDef::new(Alias::new(name)).big_integer().not_null());
        }
        for name in ["source_sha256", "source_timezone"] {
            table.col(ColumnDef::new(Alias::new(name)).string().not_null());
        }
        table.col(
            ColumnDef::new(Alias::new("expires_at"))
                .timestamp()
                .not_null(),
        );
        for (column, target) in [
            ("household_id", "households"),
            ("account_id", "accounts"),
            ("membership_id", "household_memberships"),
        ] {
            table.foreign_key(
                ForeignKey::create()
                    .from(Alias::new("data_import_inspections"), Alias::new(column))
                    .to(Alias::new(target), Alias::new("id")),
            );
        }
        manager.create_table(table.to_owned()).await?;
        // SeaQuery does not provide PostgreSQL RLS policy or role GRANT builders.
        manager.get_connection().execute_unprepared("ALTER TABLE public.data_import_inspections ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.data_import_inspections FORCE ROW LEVEL SECURITY;
CREATE POLICY import_inspection_owner ON public.data_import_inspections TO med_tracker_app USING (household_id = med_tracker.current_household_id() AND account_id = med_tracker.current_account_id() AND membership_id = med_tracker.current_membership_id()) WITH CHECK (household_id = med_tracker.current_household_id() AND account_id = med_tracker.current_account_id() AND membership_id = med_tracker.current_membership_id());
REVOKE ALL PRIVILEGES ON public.data_import_inspections FROM med_tracker_app;
GRANT SELECT, INSERT ON public.data_import_inspections TO med_tracker_app;").await?;
        Ok(())
    }
}
