use crate::scenario::setup_connection;
use med_tracker::models::_entities::{
    barcode_catalog_entries, medication_review_evidence_records,
    medication_review_evidence_refresh_runs, medication_review_prompts, nhs_dmd_amp_trade_families,
    nhs_dmd_ampp_relationships, nhs_dmd_barcodes, nhs_dmd_imports, nhs_dmd_supplementary_releases,
    nhs_dmd_trade_families, nhs_dmd_trade_family_groups,
};
use sea_orm::sea_query::Query;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, QueryFilter, TransactionTrait,
};

// Call once before booting a global catalogue case. Native nextest must reserve
// every worker for that case; ordinary tenant fixtures must never call these.
pub async fn clean_nhs_catalogue() {
    let setup = setup_connection().await;
    let transaction = setup.begin().await.unwrap();
    clear_queue(&transaction).await;
    nhs_dmd_amp_trade_families::Entity::delete_many()
        .exec(&transaction)
        .await
        .unwrap();
    nhs_dmd_trade_families::Entity::delete_many()
        .exec(&transaction)
        .await
        .unwrap();
    nhs_dmd_trade_family_groups::Entity::delete_many()
        .exec(&transaction)
        .await
        .unwrap();
    nhs_dmd_ampp_relationships::Entity::delete_many()
        .exec(&transaction)
        .await
        .unwrap();
    nhs_dmd_barcodes::Entity::delete_many()
        .exec(&transaction)
        .await
        .unwrap();
    nhs_dmd_supplementary_releases::Entity::delete_many()
        .exec(&transaction)
        .await
        .unwrap();
    nhs_dmd_imports::Entity::delete_many()
        .exec(&transaction)
        .await
        .unwrap();
    barcode_catalog_entries::Entity::delete_many()
        .filter(barcode_catalog_entries::Column::Source.eq("nhs_dmd"))
        .exec(&transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    setup.close().await.unwrap();
}

pub async fn clean_review_catalogue() {
    let setup = setup_connection().await;
    let transaction = setup.begin().await.unwrap();
    clear_queue(&transaction).await;
    medication_review_prompts::Entity::delete_many()
        .exec(&transaction)
        .await
        .unwrap();
    medication_review_evidence_records::Entity::delete_many()
        .exec(&transaction)
        .await
        .unwrap();
    medication_review_evidence_refresh_runs::Entity::delete_many()
        .exec(&transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    setup.close().await.unwrap();
}

pub async fn clean_queue() {
    let setup = setup_connection().await;
    let transaction = setup.begin().await.unwrap();
    clear_queue(&transaction).await;
    transaction.commit().await.unwrap();
    setup.close().await.unwrap();
}

async fn clear_queue(transaction: &DatabaseTransaction) {
    let delete = Query::delete()
        .from_table(("public", "pg_loco_queue"))
        .to_owned();
    transaction
        .execute_raw(transaction.get_database_backend().build(&delete))
        .await
        .unwrap();
}
