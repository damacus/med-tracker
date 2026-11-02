use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("nhs_dmd_imports"))
                    .add_column(ColumnDef::new(Alias::new("trace_id")).string_len(32).null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("nhs_dmd_imports"))
                    .drop_column(Alias::new("trace_id"))
                    .to_owned(),
            )
            .await
    }
}
