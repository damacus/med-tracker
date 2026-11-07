use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::models::errors::OperationError;

#[derive(Clone)]
pub(crate) struct UploadLimiter(Arc<Semaphore>);

impl UploadLimiter {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Semaphore::new(1)))
    }

    pub(crate) fn try_acquire(&self) -> Result<OwnedSemaphorePermit, OperationError> {
        self.0
            .clone()
            .try_acquire_owned()
            .map_err(|_| OperationError::unavailable("import_upload_busy"))
    }
}
