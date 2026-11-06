use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let queue = Alias::new("pg_loco_queue");
        manager
            .alter_table(
                Table::alter()
                    .table(queue.clone())
                    .add_column(ColumnDef::new("attempts").integer().not_null().default(0))
                    .add_column(ColumnDef::new("unique_key").string())
                    .to_owned(),
            )
            .await?;
        // SeaQuery's TableAlterStatement supports foreign keys and dropping
        // constraints, but cannot add a primary-key constraint to an existing table.
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE public.pg_loco_queue ADD PRIMARY KEY (id)")
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("pg_loco_queue_unique_queued")
                    .table(queue.clone())
                    .unique()
                    .col("name")
                    .col("unique_key")
                    .and_where(Expr::col("status").eq("queued"))
                    .and_where(Expr::col("unique_key").is_not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("pg_loco_queue_dequeue")
                    .table(queue.clone())
                    .col(("priority", sea_orm::sea_query::IndexOrder::Desc))
                    .col("run_at")
                    .col("id")
                    .and_where(Expr::col("status").eq("queued"))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("pg_loco_queue_status_updated_at")
                    .table(queue)
                    .col("status")
                    .col("updated_at")
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration("Queue migration is forward-only".into()))
    }
}
