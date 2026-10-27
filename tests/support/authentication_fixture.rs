pub async fn reset_source_quota() {
    use med_tracker::models::_entities::identity_rate_limits;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    use sha2::{Digest, Sha256};

    let setup = crate::scenario::setup_connection().await;
    let buckets = ["source:127.0.0.1", "source:unknown"]
        .map(|source| hex::encode(Sha256::digest(source.as_bytes())));
    identity_rate_limits::Entity::delete_many()
        .filter(identity_rate_limits::Column::Bucket.is_in(buckets))
        .exec(&setup)
        .await
        .unwrap();
    setup.close().await.unwrap();
}
