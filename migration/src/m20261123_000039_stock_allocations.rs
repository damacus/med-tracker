use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_query::{Expr, ExprTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut table = Table::create();
        table.table("stock_allocations").col(
            ColumnDef::new("id")
                .big_integer()
                .not_null()
                .auto_increment()
                .primary_key(),
        );
        for column in [
            "household_id",
            "medication_id",
            "person_id",
            "actor_account_id",
        ] {
            table.col(ColumnDef::new(column).big_integer().not_null());
        }
        table
            .col(ColumnDef::new("dosage_id").big_integer())
            .col(ColumnDef::new("submission_id").uuid().not_null())
            .col(ColumnDef::new("stock_unit").string().not_null())
            .col(ColumnDef::new("quantity").decimal_len(10, 2).not_null())
            .col(
                ColumnDef::new("remaining_quantity")
                    .decimal_len(10, 2)
                    .not_null(),
            )
            .col(ColumnDef::new("created_at").timestamp().not_null())
            .check(Expr::col("quantity").gt(0))
            .check(Expr::col("remaining_quantity").gte(0))
            .check(Expr::col("remaining_quantity").lte(Expr::col("quantity")));
        manager.create_table(table.to_owned()).await?;
        manager
            .create_index(
                Index::create()
                    .name("stock_allocations_unique_submission")
                    .table("stock_allocations")
                    .col("household_id")
                    .col("submission_id")
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("stock_allocations_active_stock_source")
                    .table("stock_allocations")
                    .col("household_id")
                    .col("medication_id")
                    .col("dosage_id")
                    .and_where(Expr::col("remaining_quantity").gt(0))
                    .to_owned(),
            )
            .await?;
        // SeaORM SchemaManager and SeaQuery TableAlterStatement have no
        // PostgreSQL RLS or trigger DDL. These statements only define the
        // tenant policy, runtime grants and checks across stock tables.
        manager
            .get_connection()
            .execute_unprepared(include_str!("../stock-allocations.sql"))
            .await?;
        Ok(())
    }
}
