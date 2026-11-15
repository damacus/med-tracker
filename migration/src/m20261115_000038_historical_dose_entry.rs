use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("medication_takes"))
                    .add_column(
                        ColumnDef::new(Alias::new("historical_entry"))
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;
        // SeaQuery Table and Index builders cannot define PostgreSQL trigger functions;
        // retain the existing guard and narrow only its timing rejection for historical entries.
        let db = manager.get_connection();
        db.execute_unprepared("SET LOCAL ROLE NONE").await?;
        db.execute_unprepared(include_str!("../historical-dose-guard.sql"))
            .await?;
        db.execute_unprepared("SET LOCAL ROLE med_tracker_owner")
            .await?;
        Ok(())
    }
    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Historical dose provenance is forward-only".into(),
        ))
    }
}
