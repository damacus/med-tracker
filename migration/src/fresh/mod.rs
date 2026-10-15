use sea_orm_migration::prelude::*;

mod foreign_keys;
mod indexes;
mod tables;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20261015_000001_create_medtracker"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(include_str!("roles.sql")).await?;
        db.execute_unprepared("SET LOCAL ROLE med_tracker_owner")
            .await?;
        Box::pin(tables::create(manager)).await?;
        db.execute_unprepared(include_str!("sequences.sql")).await?;
        indexes::create(manager).await?;
        db.execute_unprepared(include_str!("constraints.sql"))
            .await?;
        foreign_keys::create(manager).await?;
        db.execute_unprepared(include_str!("deferred_constraints.sql"))
            .await?;
        db.execute_unprepared("SET LOCAL ROLE NONE").await?;
        db.execute_unprepared(include_str!("functions.sql")).await?;
        db.execute_unprepared(include_str!("security.sql")).await?;
        db.execute_unprepared("SET LOCAL ROLE med_tracker_owner")
            .await?;
        db.execute_unprepared(include_str!("identity.sql")).await?;
        db.execute_unprepared(include_str!("report_denials.sql"))
            .await?;
        db.execute_unprepared(include_str!("care_continuity.sql"))
            .await?;
        db.execute_unprepared(include_str!("self_history.sql"))
            .await?;
        db.execute_unprepared(include_str!("account_clinical_access.sql"))
            .await?;
        Ok(())
    }
}
