use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut table = Table::create();
        table.table(Alias::new("person_schedules"));
        table.col(
            ColumnDef::new(Alias::new("id"))
                .string()
                .not_null()
                .primary_key(),
        );
        for name in [
            "person_id",
            "origin_household_id",
            "origin_schedule_id",
            "origin_stock_id",
            "clinical_revision",
        ] {
            table.col(ColumnDef::new(Alias::new(name)).big_integer().not_null());
        }
        for name in [
            "origin_stock_portable_id",
            "origin_time_zone",
            "timezone_provenance",
        ] {
            table.col(ColumnDef::new(Alias::new(name)).string().not_null());
        }
        for name in [
            "medicine_name",
            "dmd_system",
            "dmd_code",
            "dmd_concept_class",
            "stock_unit",
            "dose_unit",
            "frequency",
        ] {
            table.col(ColumnDef::new(Alias::new(name)).string());
        }
        table.col(ColumnDef::new(Alias::new("active")).boolean().not_null());
        table.col(ColumnDef::new(Alias::new("dose_amount")).decimal_len(10, 2));
        table.col(ColumnDef::new(Alias::new("stock_dose_default")).decimal());
        table.col(
            ColumnDef::new(Alias::new("stock_default_known"))
                .boolean()
                .not_null(),
        );
        table.col(ColumnDef::new(Alias::new("pause_started_at")).timestamp());
        table.col(ColumnDef::new(Alias::new("pause_reason")).string());
        table.col(ColumnDef::new(Alias::new("pause_note")).text());
        for name in ["dose_cycle", "max_daily_doses", "min_hours_between_doses"] {
            table.col(ColumnDef::new(Alias::new(name)).integer());
        }
        table.col(
            ColumnDef::new(Alias::new("schedule_type"))
                .integer()
                .not_null(),
        );
        table.col(
            ColumnDef::new(Alias::new("schedule_config"))
                .json_binary()
                .not_null(),
        );
        table.col(ColumnDef::new(Alias::new("notes")).text());
        for name in ["start_date", "end_date"] {
            table.col(ColumnDef::new(Alias::new(name)).date());
        }
        for name in ["created_at", "updated_at"] {
            table.col(ColumnDef::new(Alias::new(name)).timestamp().not_null());
        }
        table.col(ColumnDef::new(Alias::new("retired_at")).timestamp());
        table.foreign_key(
            ForeignKey::create()
                .name("person_schedules_person")
                .from(Alias::new("person_schedules"), Alias::new("person_id"))
                .to(Alias::new("people"), Alias::new("id")),
        );
        manager.create_table(table.to_owned()).await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("person_schedules_origin_source")
                    .from(
                        Alias::new("person_schedules"),
                        Alias::new("origin_schedule_id"),
                    )
                    .to(Alias::new("schedules"), Alias::new("id"))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("person_schedules_origin_source_unique")
                    .table(Alias::new("person_schedules"))
                    .col(Alias::new("origin_schedule_id"))
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("schedules"))
                    .add_column(ColumnDef::new(Alias::new("person_schedule_id")).string())
                    .add_column(ColumnDef::new(Alias::new("clinical_revision")).big_integer())
                    .add_column(ColumnDef::new(Alias::new("origin_time_zone")).string())
                    .add_column(ColumnDef::new(Alias::new("execution_ended_at")).timestamp())
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("schedules_person_schedule")
                    .from(Alias::new("schedules"), Alias::new("person_schedule_id"))
                    .to(Alias::new("person_schedules"), Alias::new("id"))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("schedules_person_execution")
                    .table(Alias::new("schedules"))
                    .col(Alias::new("person_schedule_id"))
                    .col(Alias::new("household_id"))
                    .unique()
                    .to_owned(),
            )
            .await?;
        // SeaQuery has no RLS policy, role ownership or GRANT builders. Only that
        // unsupported DDL is isolated here; adoption and clinical writes use SeaORM.
        manager
            .get_connection()
            .execute_unprepared(include_str!("../person-schedules.sql"))
            .await?;
        Ok(())
    }
}
