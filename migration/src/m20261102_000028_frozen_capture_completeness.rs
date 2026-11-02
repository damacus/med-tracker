use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::sea_query::{Expr, JoinType, Query};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let fk = Query::select()
            .expr(Expr::val(1))
            .from_as(
                (Alias::new("pg_catalog"), Alias::new("pg_constraint")),
                Alias::new("constraint"),
            )
            .join_as(
                JoinType::InnerJoin,
                (Alias::new("pg_catalog"), Alias::new("pg_class")),
                Alias::new("relation"),
                Expr::col((Alias::new("relation"), Alias::new("oid")))
                    .equals((Alias::new("constraint"), Alias::new("conrelid"))),
            )
            .join_as(
                JoinType::InnerJoin,
                (Alias::new("pg_catalog"), Alias::new("pg_namespace")),
                Alias::new("namespace"),
                Expr::col((Alias::new("namespace"), Alias::new("oid")))
                    .equals((Alias::new("relation"), Alias::new("relnamespace"))),
            )
            .and_where(Expr::col((Alias::new("namespace"), Alias::new("nspname"))).eq("public"))
            .and_where(
                Expr::col((Alias::new("relation"), Alias::new("relname"))).eq("medication_takes"),
            )
            .and_where(
                Expr::col((Alias::new("constraint"), Alias::new("conname")))
                    .eq("fk_medication_takes_schedule_id_household"),
            )
            .and_where(Expr::col((Alias::new("constraint"), Alias::new("condeferrable"))).eq(false))
            .and_where(Expr::col((Alias::new("constraint"), Alias::new("condeferred"))).eq(false))
            .to_owned();
        if db
            .query_one_raw(db.get_database_backend().build(&fk))
            .await?
            .is_none()
        {
            return Err(DbErr::Migration(
                "Immediate take/source foreign key required".into(),
            ));
        }
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("legacy_schedule_prescriptions"))
                    .add_column(ColumnDef::new(Alias::new("capture_complete")).boolean())
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("frozen_history_invalidations"))
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("source_schedule_id"))
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("frozen_history_invalidation_source")
                    .table(Alias::new("frozen_history_invalidations"))
                    .col(Alias::new("source_schedule_id"))
                    .to_owned(),
            )
            .await?;
        // SeaORM/SeaQuery cannot express PostgreSQL transition-table triggers,
        // deferred constraint triggers, RLS policies, or SECURITY DEFINER ACLs.
        db.execute_unprepared("SET LOCAL ROLE NONE").await?;
        db.execute_unprepared(include_str!("../frozen-capture-completeness.sql"))
            .await?;
        db.execute_unprepared("SET LOCAL ROLE med_tracker_owner")
            .await?;
        Ok(())
    }
}
