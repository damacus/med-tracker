use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_query::{Condition, Expr, ExprTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut table = Table::create();
        table.table(Alias::new("person_household_approvals"));
        table.col(
            ColumnDef::new(Alias::new("id"))
                .big_integer()
                .not_null()
                .auto_increment()
                .primary_key(),
        );
        for name in [
            "household_id",
            "person_id",
            "invitation_id",
            "approved_by_account_id",
            "approved_by_person_id",
        ] {
            table.col(ColumnDef::new(Alias::new(name)).big_integer().not_null());
        }
        table.col(ColumnDef::new(Alias::new("guardian_declaration_id")).big_integer());
        table.col(
            ColumnDef::new(Alias::new("statement_version"))
                .small_integer()
                .not_null(),
        );
        table.col(
            ColumnDef::new(Alias::new("approved_at"))
                .timestamp()
                .not_null(),
        );
        table.col(ColumnDef::new(Alias::new("ended_at")).timestamp());
        for (column, target) in [
            ("household_id", "households"),
            ("person_id", "people"),
            ("invitation_id", "household_invitations"),
            ("approved_by_account_id", "accounts"),
            ("approved_by_person_id", "people"),
            ("guardian_declaration_id", "guardian_declarations"),
        ] {
            table.foreign_key(
                ForeignKey::create()
                    .name(format!("sharing_approval_{column}"))
                    .from(Alias::new("person_household_approvals"), Alias::new(column))
                    .to(Alias::new(target), Alias::new("id")),
            );
        }
        manager.create_table(table.to_owned()).await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("household_invitations"))
                    .add_column(
                        ColumnDef::new(Alias::new("dependent_sharing"))
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("schedules"))
                    .add_column(ColumnDef::new(Alias::new("sharing_approval_id")).big_integer())
                    .add_column(
                        ColumnDef::new(Alias::new("canonical_origin"))
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("schedule_sharing_approval")
                    .from(Alias::new("schedules"), Alias::new("sharing_approval_id"))
                    .to(Alias::new("person_household_approvals"), Alias::new("id"))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("person_schedules"))
                    .add_column(ColumnDef::new(Alias::new("pause_instance_id")).string())
                    .to_owned(),
            )
            .await?;
        let mut safe = Table::create();
        safe.table(Alias::new("shared_schedule_states"));
        safe.col(
            ColumnDef::new(Alias::new("id"))
                .string()
                .not_null()
                .primary_key(),
        );
        safe.col(
            ColumnDef::new(Alias::new("person_id"))
                .big_integer()
                .not_null(),
        );
        safe.col(
            ColumnDef::new(Alias::new("clinical_revision"))
                .big_integer()
                .not_null(),
        );
        for name in [
            "medicine_name",
            "dmd_system",
            "dmd_code",
            "dmd_concept_class",
            "stock_unit",
            "dose_unit",
            "frequency",
            "pause_instance_id",
            "last_closed_pause_instance_id",
        ] {
            safe.col(ColumnDef::new(Alias::new(name)).string());
        }
        safe.col(
            ColumnDef::new(Alias::new("origin_time_zone"))
                .string()
                .not_null(),
        );
        for name in ["stock_default_known", "active"] {
            safe.col(ColumnDef::new(Alias::new(name)).boolean().not_null());
        }
        safe.col(ColumnDef::new(Alias::new("stock_dose_default")).decimal());
        safe.col(ColumnDef::new(Alias::new("dose_amount")).decimal_len(10, 2));
        for name in ["pause_started_at", "retired_at", "latest_taken_at"] {
            safe.col(ColumnDef::new(Alias::new(name)).timestamp());
        }
        for name in ["created_at", "updated_at"] {
            safe.col(ColumnDef::new(Alias::new(name)).timestamp().not_null());
        }
        for name in ["start_date", "end_date"] {
            safe.col(ColumnDef::new(Alias::new(name)).date());
        }
        for name in ["dose_cycle", "max_daily_doses", "min_hours_between_doses"] {
            safe.col(ColumnDef::new(Alias::new(name)).integer());
        }
        safe.col(
            ColumnDef::new(Alias::new("schedule_type"))
                .integer()
                .not_null(),
        );
        safe.col(
            ColumnDef::new(Alias::new("schedule_config"))
                .json_binary()
                .not_null(),
        );
        safe.foreign_key(
            ForeignKey::create()
                .name("shared_schedule_canonical")
                .from(Alias::new("shared_schedule_states"), Alias::new("id"))
                .to(Alias::new("person_schedules"), Alias::new("id")),
        );
        manager.create_table(safe.to_owned()).await?;
        let mut closures = Table::create();
        closures.table(Alias::new("shared_schedule_pause_closures"));
        closures.col(
            ColumnDef::new(Alias::new("pause_instance_id"))
                .string()
                .not_null()
                .primary_key(),
        );
        for name in ["person_schedule_id", "request_id"] {
            closures.col(ColumnDef::new(Alias::new(name)).string().not_null());
        }
        for name in ["authentication_method", "session_reference"] {
            closures.col(ColumnDef::new(Alias::new(name)).string());
        }
        for name in ["clinical_revision", "actor_account_id", "actor_person_id"] {
            closures.col(ColumnDef::new(Alias::new(name)).big_integer().not_null());
        }
        for name in ["actor_membership_id", "guardian_declaration_id"] {
            closures.col(ColumnDef::new(Alias::new(name)).big_integer());
        }
        for name in ["ended_at", "created_at"] {
            closures.col(ColumnDef::new(Alias::new(name)).timestamp().not_null());
        }
        for (column, target) in [
            ("person_schedule_id", "shared_schedule_states"),
            ("actor_account_id", "accounts"),
            ("actor_person_id", "people"),
            ("actor_membership_id", "household_memberships"),
            ("guardian_declaration_id", "guardian_declarations"),
        ] {
            closures.foreign_key(
                ForeignKey::create()
                    .name(format!("pause_closure_{column}"))
                    .from(
                        Alias::new("shared_schedule_pause_closures"),
                        Alias::new(column),
                    )
                    .to(Alias::new(target), Alias::new("id")),
            );
        }
        manager.create_table(closures.to_owned()).await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("medication_pause_periods"))
                    .add_column(ColumnDef::new(Alias::new("canonical_person_schedule_id")).string())
                    .add_column(
                        ColumnDef::new(Alias::new("canonical_pause_revision")).big_integer(),
                    )
                    .add_column(
                        ColumnDef::new(Alias::new("canonical_resume_id"))
                            .string()
                            .check(Expr::expr(
                                Condition::any()
                                    .add(
                                        Condition::all()
                                            .add(Expr::col(Alias::new("ended_at")).is_null())
                                            .add(
                                                Expr::col(Alias::new("resumed_by_membership_id"))
                                                    .is_null(),
                                            )
                                            .add(
                                                Expr::col(Alias::new("canonical_resume_id"))
                                                    .is_null(),
                                            ),
                                    )
                                    .add(
                                        Condition::all()
                                            .add(Expr::col(Alias::new("ended_at")).is_not_null())
                                            .add(
                                                Condition::any()
                                                    .add(
                                                        Expr::col(Alias::new(
                                                            "resumed_by_membership_id",
                                                        ))
                                                        .is_not_null(),
                                                    )
                                                    .add(
                                                        Expr::col(Alias::new("imported_context"))
                                                            .eq(true),
                                                    )
                                                    .add(
                                                        Expr::col(Alias::new(
                                                            "canonical_resume_id",
                                                        ))
                                                        .is_not_null(),
                                                    ),
                                            ),
                                    ),
                            )),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("medication_pause_periods"))
                    .drop_constraint(Alias::new("chk_medication_pause_periods_resuming_actor"))
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("pause_period_canonical_resume")
                    .from(
                        Alias::new("medication_pause_periods"),
                        Alias::new("canonical_resume_id"),
                    )
                    .to(
                        Alias::new("shared_schedule_pause_closures"),
                        Alias::new("pause_instance_id"),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("pause_period_canonical_plan")
                    .from(
                        Alias::new("medication_pause_periods"),
                        Alias::new("canonical_person_schedule_id"),
                    )
                    .to(Alias::new("shared_schedule_states"), Alias::new("id"))
                    .to_owned(),
            )
            .await?;
        let mut links = Table::create();
        links.table(Alias::new("shared_schedule_executions"));
        links.col(
            ColumnDef::new(Alias::new("schedule_id"))
                .big_integer()
                .not_null()
                .primary_key(),
        );
        for name in ["household_id", "person_id"] {
            links.col(ColumnDef::new(Alias::new(name)).big_integer().not_null());
        }
        links.col(ColumnDef::new(Alias::new("approval_id")).big_integer());
        for name in ["portable_id", "person_schedule_id"] {
            links.col(ColumnDef::new(Alias::new(name)).string().not_null());
        }
        for (column, target, target_column) in [
            ("schedule_id", "schedules", "id"),
            ("household_id", "households", "id"),
            ("approval_id", "person_household_approvals", "id"),
            ("person_id", "people", "id"),
            ("person_schedule_id", "shared_schedule_states", "id"),
        ] {
            links.foreign_key(
                ForeignKey::create()
                    .name(format!("shared_execution_{column}"))
                    .from(Alias::new("shared_schedule_executions"), Alias::new(column))
                    .to(Alias::new(target), Alias::new(target_column)),
            );
        }
        manager.create_table(links.to_owned()).await?;
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("shared_schedule_administrations"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("person_schedule_id"))
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("household_id"))
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("medication_take_id"))
                            .big_integer()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("taken_at"))
                            .timestamp()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Alias::new("occurrence_starts_on")).date())
                    .col(ColumnDef::new(Alias::new("occurrence_position")).integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("shared_administration_plan")
                            .from(
                                Alias::new("shared_schedule_administrations"),
                                Alias::new("person_schedule_id"),
                            )
                            .to(Alias::new("shared_schedule_states"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("shared_administration_take")
                            .from(
                                Alias::new("shared_schedule_administrations"),
                                Alias::new("medication_take_id"),
                            )
                            .to(Alias::new("medication_takes"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await?;
        // SeaQuery Table/Index/ForeignKey builders cover ordinary schema changes;
        // PostgreSQL RLS, role ACLs and immutable-authority triggers have no builders.
        let db = manager.get_connection();
        db.execute_unprepared("SET LOCAL ROLE NONE").await?;
        db.execute_unprepared(include_str!("../approved-sharing.sql"))
            .await?;
        db.execute_unprepared("SET LOCAL ROLE med_tracker_owner")
            .await?;
        Ok(())
    }
}
