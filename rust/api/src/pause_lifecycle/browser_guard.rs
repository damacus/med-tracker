use super::*;

#[derive(Clone)]
pub(crate) struct BrowserSourceGuard {
    pub(crate) source_type: String,
    pub(crate) source_id: String,
    pub(crate) original_etag: String,
}

pub(super) async fn validate(
    db: &DatabaseTransaction,
    context: &AuthContext,
    source: &Source,
    guard: &BrowserSourceGuard,
) -> Result<Option<Failure>, ApiError> {
    if guard.original_etag.trim().is_empty() {
        return Ok(Some(Failure::PreconditionRequired));
    }
    if guard.source_type != source.kind().name() || guard.source_id != source.portable_id() {
        return Ok(Some(Failure::Conflict));
    }
    let body = representation::source_body(db, context, source).await?;
    Ok((guard.original_etag != representation_etag(&body)).then_some(Failure::Conflict))
}
