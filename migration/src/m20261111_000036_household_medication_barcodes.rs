use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("index_medications_on_barcode")
                    .table("medications")
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("index_medications_on_barcode")
                    .table("medications")
                    .unique()
                    .col("household_id")
                    .col("barcode")
                    .and_where(Expr::col("barcode").is_not_null())
                    .and_where(Expr::col("barcode").ne(""))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Medication barcode migration is forward-only".into(),
        ))
    }
}
