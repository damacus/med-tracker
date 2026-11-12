use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager
            .has_column("households", "ai_suggestions_enabled")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table("households")
                        .add_column(
                            ColumnDef::new(Alias::new("ai_suggestions_enabled"))
                                .boolean()
                                .not_null()
                                .default(false),
                        )
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Household AI consent migration is forward-only".into(),
        ))
    }
}
