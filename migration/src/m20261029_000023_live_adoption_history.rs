use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut table = Table::create();
        table.table(Alias::new("shared_schedule_adoption_facts"));
        table.col(
            ColumnDef::new(Alias::new("medication_take_id"))
                .big_integer()
                .not_null()
                .primary_key(),
        );
        for name in [
            "source_schedule_id",
            "person_id",
            "household_id",
            "original_version_id",
            "captured_by_account_id",
            "captured_by_person_id",
            "captured_by_membership_id",
        ] {
            table.col(ColumnDef::new(Alias::new(name)).big_integer().not_null());
        }
        for name in [
            "person_schedule_id",
            "source_portable_id",
            "take_portable_id",
            "request_id",
        ] {
            table.col(ColumnDef::new(Alias::new(name)).string().not_null());
        }
        for name in [
            "client_uuid",
            "dose_unit",
            "authentication_method",
            "credential_reference",
        ] {
            table.col(ColumnDef::new(Alias::new(name)).string());
        }
        for name in [
            "taken_from_medication_id",
            "taken_from_location_id",
            "guardian_declaration_id",
        ] {
            table.col(ColumnDef::new(Alias::new(name)).big_integer());
        }
        for name in [
            "taken_at",
            "take_created_at",
            "take_updated_at",
            "captured_at",
        ] {
            table.col(ColumnDef::new(Alias::new(name)).timestamp().not_null());
        }
        table.col(
            ColumnDef::new(Alias::new("dose_amount"))
                .decimal_len(10, 2)
                .not_null(),
        );
        table.col(ColumnDef::new(Alias::new("fact_hash")).binary().not_null());
        for (column, target) in [
            ("medication_take_id", "medication_takes"),
            ("person_schedule_id", "person_schedules"),
            ("source_schedule_id", "schedules"),
            ("person_id", "people"),
            ("household_id", "households"),
            ("original_version_id", "versions"),
            ("captured_by_account_id", "accounts"),
            ("captured_by_person_id", "people"),
            ("captured_by_membership_id", "household_memberships"),
            ("guardian_declaration_id", "guardian_declarations"),
        ] {
            table.foreign_key(
                ForeignKey::create()
                    .name(format!("adoption_fact_{column}"))
                    .from(
                        Alias::new("shared_schedule_adoption_facts"),
                        Alias::new(column),
                    )
                    .to(Alias::new(target), Alias::new("id")),
            );
        }
        manager.create_table(table.to_owned()).await?;
        let db = manager.get_connection();
        db.execute_unprepared("SET LOCAL ROLE NONE").await?;
        db.execute_unprepared(include_str!("../adoption-history.sql"))
            .await?;
        db.execute_unprepared("SET LOCAL ROLE med_tracker_owner")
            .await?;
        Ok(())
    }
}
