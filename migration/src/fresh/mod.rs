use sea_orm_migration::prelude::*;

mod foreign_keys;
mod indexes;
mod tables;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261013_000001_create_medtracker"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(include_str!("roles.sql")).await?;
        db.execute_unprepared("SET LOCAL ROLE med_tracker_owner")
            .await?;
        tables::create(manager).await?;
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
        db.execute_unprepared(include_str!("recovery_eligibility.sql"))
            .await?;
        db.execute_unprepared("SET LOCAL ROLE med_tracker_owner")
            .await?;
        Ok(())
    }
}
