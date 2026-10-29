use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // A ColumnDef without a type emits only ALTER COLUMN ... DROP NOT NULL;
        // restating the varchar type would fail because RLS policies reference
        // the column. A NULL person_schedule_id marks immutable frozen-departure
        // evidence whose canonical binding is the legacy_schedule_prescriptions
        // parent; live receipts keep their prospective NOT NULL binding through
        // the guard and policy forks in the companion SQL.
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("shared_schedule_adoption_facts"))
                    .modify_column(ColumnDef::new(Alias::new("person_schedule_id")).null())
                    .to_owned(),
            )
            .await?;
        let db = manager.get_connection();
        db.execute_unprepared("SET LOCAL ROLE NONE").await?;
        db.execute_unprepared(include_str!("../frozen-adoption-history.sql"))
            .await?;
        db.execute_unprepared("SET LOCAL ROLE med_tracker_owner")
            .await?;
        Ok(())
    }
}
