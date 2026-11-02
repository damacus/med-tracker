use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_query::{Expr, ExprTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut table = Table::create();
        table.table(Alias::new("legacy_schedule_prescriptions"));
        table.col(
            ColumnDef::new(Alias::new("schedule_id"))
                .big_integer()
                .not_null()
                .primary_key(),
        );
        for column in [
            "person_id",
            "household_id",
            "origin_stock_id",
            "captured_by_account_id",
            "captured_by_membership_id",
        ] {
            table.col(ColumnDef::new(Alias::new(column)).big_integer().not_null());
        }
        for column in [
            "source_portable_id",
            "origin_stock_portable_id",
            "request_id",
        ] {
            table.col(ColumnDef::new(Alias::new(column)).string().not_null());
        }
        for column in [
            "medicine_name",
            "dmd_system",
            "dmd_code",
            "dmd_concept_class",
            "stock_unit",
            "dose_unit",
            "frequency",
            "origin_time_zone",
            "pause_instance_id",
            "adopted_person_schedule_id",
        ] {
            table.col(ColumnDef::new(Alias::new(column)).string());
        }
        for column in ["active", "stock_default_known", "source_closed_active"] {
            table.col(ColumnDef::new(Alias::new(column)).boolean().not_null());
        }
        table.check(Expr::col(Alias::new("source_closed_active")).eq(false));
        table.col(ColumnDef::new(Alias::new("stock_dose_default")).decimal());
        table.col(ColumnDef::new(Alias::new("dose_amount")).decimal_len(10, 2));
        for column in ["dose_cycle", "max_daily_doses", "min_hours_between_doses"] {
            table.col(ColumnDef::new(Alias::new(column)).integer());
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
        for column in ["start_date", "end_date"] {
            table.col(ColumnDef::new(Alias::new(column)).date());
        }
        for column in ["retired_at", "pause_started_at"] {
            table.col(ColumnDef::new(Alias::new(column)).timestamp());
        }
        for column in ["source_created_at", "frozen_at", "source_ended_at"] {
            table.col(ColumnDef::new(Alias::new(column)).timestamp().not_null());
        }
        manager.create_table(table.to_owned()).await?;
        for (name, parent, columns) in [
            (
                "legacy_source_closed_identity",
                "schedules",
                vec!["id", "execution_ended_at", "active"],
            ),
            (
                "legacy_source_person_identity",
                "schedules",
                vec!["id", "person_id", "household_id"],
            ),
            (
                "legacy_canonical_origin_identity",
                "person_schedules",
                vec![
                    "id",
                    "origin_schedule_id",
                    "person_id",
                    "origin_household_id",
                ],
            ),
        ] {
            let mut index = Index::create();
            index.name(name).table(Alias::new(parent)).unique();
            for column in columns {
                index.col(Alias::new(column));
            }
            manager.create_index(index.to_owned()).await?;
        }
        manager
            .create_index(
                Index::create()
                    .name("legacy_adopted_plan_once")
                    .table(Alias::new("legacy_schedule_prescriptions"))
                    .col(Alias::new("adopted_person_schedule_id"))
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("legacy_frozen_source_closure")
                    .from(
                        Alias::new("legacy_schedule_prescriptions"),
                        (
                            Alias::new("schedule_id"),
                            Alias::new("source_ended_at"),
                            Alias::new("source_closed_active"),
                        ),
                    )
                    .to(
                        Alias::new("schedules"),
                        (
                            Alias::new("id"),
                            Alias::new("execution_ended_at"),
                            Alias::new("active"),
                        ),
                    )
                    .on_update(ForeignKeyAction::NoAction)
                    .on_delete(ForeignKeyAction::Restrict)
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("legacy_canonical_source_identity")
                    .from(
                        Alias::new("person_schedules"),
                        (
                            Alias::new("origin_schedule_id"),
                            Alias::new("person_id"),
                            Alias::new("origin_household_id"),
                        ),
                    )
                    .to(
                        Alias::new("schedules"),
                        (
                            Alias::new("id"),
                            Alias::new("person_id"),
                            Alias::new("household_id"),
                        ),
                    )
                    .on_delete(ForeignKeyAction::Restrict)
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("legacy_adopted_canonical_identity")
                    .from_tbl(Alias::new("legacy_schedule_prescriptions"))
                    .from_col(Alias::new("adopted_person_schedule_id"))
                    .from_col(Alias::new("schedule_id"))
                    .from_col(Alias::new("person_id"))
                    .from_col(Alias::new("household_id"))
                    .to_tbl(Alias::new("person_schedules"))
                    .to_col(Alias::new("id"))
                    .to_col(Alias::new("origin_schedule_id"))
                    .to_col(Alias::new("person_id"))
                    .to_col(Alias::new("origin_household_id"))
                    .on_delete(ForeignKeyAction::Restrict)
                    .to_owned(),
            )
            .await?;
        // SeaQuery 1.0.2 ForeignKeyCreateStatement/TableForeignKey support tuple
        // columns and actions, but no deferrability; its PostgreSQL FK renderer
        // ends after actions. Keep only ALTER CONSTRAINT and unsupported RLS,
        // trigger, ownership and grant DDL in this isolated PostgreSQL boundary.
        let db = manager.get_connection();
        db.execute_unprepared("SET LOCAL ROLE NONE").await?;
        db.execute_unprepared(include_str!("../legacy-schedule-prescriptions.sql"))
            .await?;
        db.execute_unprepared("SET LOCAL ROLE med_tracker_owner")
            .await?;
        Ok(())
    }
}
