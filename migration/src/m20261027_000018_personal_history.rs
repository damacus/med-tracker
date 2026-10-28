use sea_orm::sea_query::{Expr, ExprTrait, Func, Query};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let connection = manager.get_connection();
        // SeaQuery 1.0 SchemaStatement has table/index/foreign-key builders, but no
        // CREATE VIEW, security_barrier or RLS policy builders. Keep that DDL here;
        // the view SELECT and all application reads use SeaORM query builders.
        connection
            .execute_unprepared(include_str!("../personal-history.sql"))
            .await?;
        let query = Query::select()
            .column(("medication_takes", "id"))
            .expr_as(
                Func::coalesce([
                    Expr::col(("schedules", "person_id")),
                    Expr::col(("person_medications", "person_id")),
                ]),
                "person_id",
            )
            .column(("medication_takes", "taken_at"))
            .column(("medication_takes", "dose_amount"))
            .column(("medication_takes", "dose_unit"))
            .expr_as(Expr::col(("medications", "name")), "medicine")
            .from(("public", "medication_takes"))
            .left_join(
                ("public", "schedules"),
                Expr::col(("schedules", "id")).equals(("medication_takes", "schedule_id")),
            )
            .left_join(
                ("public", "person_medications"),
                Expr::col(("person_medications", "id"))
                    .equals(("medication_takes", "person_medication_id")),
            )
            .inner_join(
                ("public", "accounts"),
                Expr::col(("accounts", "person_id")).eq(Func::coalesce([
                    Expr::col(("schedules", "person_id")),
                    Expr::col(("person_medications", "person_id")),
                ])),
            )
            .inner_join(
                ("public", "medications"),
                Expr::col(("medications", "id")).eq(Func::coalesce([
                    Expr::col(("medication_takes", "taken_from_medication_id")),
                    Expr::col(("schedules", "medication_id")),
                    Expr::col(("person_medications", "medication_id")),
                ])),
            )
            .and_where(
                Expr::col(("accounts", "id")).eq(Func::cust("med_tracker.current_account_id")),
            )
            .and_where(Expr::col(("accounts", "status")).eq(2))
            .and_where(Expr::col(("medication_takes", "taken_at")).is_not_null())
            .to_owned();
        let statement = connection.get_database_backend().build(&query);
        connection.execute_unprepared(&format!("CREATE VIEW public.personal_taken_doses WITH (security_barrier=true) AS {statement}")).await?;
        connection.execute_unprepared("ALTER VIEW public.personal_taken_doses OWNER TO med_tracker_owner; REVOKE ALL ON public.personal_taken_doses FROM PUBLIC, med_tracker_app; GRANT SELECT ON public.personal_taken_doses TO med_tracker_app").await?;
        Ok(())
    }
}
