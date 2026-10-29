#[derive(Clone, Debug, PartialEq)]
pub enum OperationError {
    Unauthenticated,
    Forbidden,
    NotFound,
    Validation {
        details: serde_json::Value,
    },
    Conflict {
        code: String,
        details: serde_json::Value,
    },
    Unavailable,
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Unauthenticated => "Authentication required",
            Self::Forbidden => "You are not authorized to perform this action",
            Self::NotFound => "Record not found",
            Self::Validation { .. } => "Invalid operation",
            Self::Conflict { .. } => "Operation conflicts with the current state",
            Self::Unavailable => "Operation temporarily unavailable",
        })
    }
}

impl std::error::Error for OperationError {}

impl From<sea_orm::DbErr> for OperationError {
    fn from(error: sea_orm::DbErr) -> Self {
        let category = database_error_category(&error);
        tracing::error!(category, error_trace_id = %uuid::Uuid::new_v4(), "Database operation failed");
        Self::Unavailable
    }
}

impl From<opendal::Error> for OperationError {
    fn from(error: opendal::Error) -> Self {
        let category = storage_error_category(&error);
        tracing::error!(category, error_trace_id = %uuid::Uuid::new_v4(), "Storage operation failed");
        Self::Unavailable
    }
}

impl From<tokio::time::error::Elapsed> for OperationError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        tracing::error!(category = "timeout", error_trace_id = %uuid::Uuid::new_v4(), "Operation timed out");
        Self::Unavailable
    }
}

impl From<std::io::Error> for OperationError {
    fn from(error: std::io::Error) -> Self {
        let category = io_error_category(&error);
        tracing::error!(category, error_trace_id = %uuid::Uuid::new_v4(), "I/O operation failed");
        Self::Unavailable
    }
}

impl OperationError {
    /// Safe diagnostic for operational failures that carry no error value to
    /// convert, such as an absent configured resource or a violated
    /// persistence invariant.
    pub(crate) fn unavailable(category: &'static str) -> Self {
        tracing::error!(category, error_trace_id = %uuid::Uuid::new_v4(), "Operation dependency unavailable");
        Self::Unavailable
    }
}

fn storage_error_category(error: &opendal::Error) -> &'static str {
    if error.is_temporary() {
        return "transient";
    }
    match error.kind() {
        opendal::ErrorKind::NotFound => "not_found",
        opendal::ErrorKind::PermissionDenied => "permission",
        opendal::ErrorKind::ConfigInvalid | opendal::ErrorKind::Unsupported => "configuration",
        _ => "storage",
    }
}

fn io_error_category(error: &std::io::Error) -> &'static str {
    match error.kind() {
        std::io::ErrorKind::NotFound => "not_found",
        std::io::ErrorKind::PermissionDenied => "permission",
        std::io::ErrorKind::TimedOut => "timeout",
        _ => "io",
    }
}

fn database_error_category(error: &sea_orm::DbErr) -> &'static str {
    use sea_orm::{ConnAcquireErr, DbErr, RuntimeErr};

    match error {
        DbErr::ConnectionAcquire(ConnAcquireErr::Timeout) => "timeout",
        DbErr::ConnectionAcquire(ConnAcquireErr::ConnectionClosed) => "connection",
        DbErr::Conn(RuntimeErr::SqlxError(error)) => sqlx_error_category(error, "connection"),
        DbErr::Conn(_) => "connection",
        DbErr::Query(RuntimeErr::SqlxError(error)) => sqlx_error_category(error, "query"),
        DbErr::Exec(RuntimeErr::SqlxError(error)) => sqlx_error_category(error, "execution"),
        DbErr::Query(_) => "query",
        DbErr::Exec(_) => "execution",
        _ => "other",
    }
}

fn sqlx_error_category(error: &sea_orm::SqlxError, fallback: &'static str) -> &'static str {
    match error {
        sea_orm::SqlxError::Database(database_error) => database_error
            .code()
            .as_deref()
            .and_then(sqlstate_category)
            .unwrap_or(fallback),
        sea_orm::SqlxError::PoolTimedOut => "timeout",
        sea_orm::SqlxError::PoolClosed
        | sea_orm::SqlxError::Io(_)
        | sea_orm::SqlxError::Tls(_)
        | sea_orm::SqlxError::Protocol(_)
        | sea_orm::SqlxError::WorkerCrashed => "connection",
        _ => fallback,
    }
}

fn sqlstate_category(code: &str) -> Option<&'static str> {
    if code.starts_with("23") {
        Some("constraint")
    } else if code.starts_with("08") || code == "57P01" {
        Some("connection")
    } else {
        None
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::OperationError;
    use sea_orm::{ConnAcquireErr, DbErr, RuntimeErr, SqlxError};
    use std::{
        borrow::Cow,
        sync::{Arc, Mutex},
    };
    use tracing::{
        Event, Metadata, Subscriber,
        field::{Field, Visit},
        span::{Attributes, Id, Record},
    };

    type CapturedEvent = Vec<(String, String)>;

    #[derive(Clone, Default)]
    struct CapturedEvents(Arc<Mutex<Vec<CapturedEvent>>>);

    pub(crate) fn capture_operation(
        operation: impl FnOnce() -> OperationError,
    ) -> Result<(OperationError, Vec<CapturedEvent>), String> {
        let events = CapturedEvents::default();
        let result = tracing::subscriber::with_default(events.clone(), operation);
        let recorded = events
            .0
            .lock()
            .map_err(|_| "Captured database event lock poisoned".to_owned())?
            .clone();
        Ok((result, recorded))
    }

    #[derive(Default)]
    struct CapturedFields(CapturedEvent);

    #[derive(Debug)]
    struct TestDatabaseError {
        code: &'static str,
        message: &'static str,
    }

    impl std::fmt::Display for TestDatabaseError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(self.message)
        }
    }

    impl std::error::Error for TestDatabaseError {}

    impl sea_orm::sqlx::error::DatabaseError for TestDatabaseError {
        fn message(&self) -> &str {
            self.message
        }

        fn code(&self) -> Option<Cow<'_, str>> {
            Some(Cow::Borrowed(self.code))
        }

        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }

        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }

        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }

        fn kind(&self) -> sea_orm::sqlx::error::ErrorKind {
            sea_orm::sqlx::error::ErrorKind::Other
        }
    }

    impl Visit for CapturedFields {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.0.push((field.name().to_owned(), format!("{value:?}")));
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            self.0.push((field.name().to_owned(), value.to_owned()));
        }
    }

    impl Subscriber for CapturedEvents {
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _: &Attributes<'_>) -> Id {
            Id::from_u64(1)
        }

        fn record(&self, _: &Id, _: &Record<'_>) {}

        fn record_follows_from(&self, _: &Id, _: &Id) {}

        fn event(&self, event: &Event<'_>) {
            assert_eq!(*event.metadata().level(), tracing::Level::ERROR);
            let mut fields = CapturedFields::default();
            event.record(&mut fields);
            let Ok(mut events) = self.0.lock() else {
                panic!("captured event lock poisoned");
            };
            events.push(fields.0);
        }

        fn enter(&self, _: &Id) {}

        fn exit(&self, _: &Id) {}
    }

    #[test]
    fn database_errors_emit_safe_category_and_error_trace_id() {
        let events = CapturedEvents::default();
        let sentinel = "patient=TEST-CLINICAL;password=TEST-SECRET;SQL=SELECT * FROM private";
        tracing::subscriber::with_default(events.clone(), || {
            for error in [
                DbErr::ConnectionAcquire(ConnAcquireErr::Timeout),
                DbErr::Conn(RuntimeErr::Internal(sentinel.to_owned())),
                DbErr::Conn(RuntimeErr::SqlxError(Arc::new(SqlxError::PoolTimedOut))),
                DbErr::Query(RuntimeErr::Internal(sentinel.to_owned())),
                DbErr::Exec(RuntimeErr::Internal(sentinel.to_owned())),
                DbErr::Custom(sentinel.to_owned()),
                DbErr::Exec(RuntimeErr::SqlxError(Arc::new(SqlxError::Database(
                    Box::new(TestDatabaseError {
                        code: "23505",
                        message: sentinel,
                    }),
                )))),
                DbErr::Query(RuntimeErr::SqlxError(Arc::new(SqlxError::PoolTimedOut))),
            ] {
                let operation_error = OperationError::from(error);
                assert_eq!(operation_error, OperationError::Unavailable);
                assert_eq!(
                    operation_error.to_string(),
                    "Operation temporarily unavailable"
                );
            }
        });

        let Ok(recorded) = events.0.lock() else {
            panic!("captured event lock poisoned");
        };
        assert_eq!(recorded.len(), 8);
        let mut trace_ids = std::collections::HashSet::new();
        for (fields, category) in recorded.iter().zip([
            "timeout",
            "connection",
            "timeout",
            "query",
            "execution",
            "other",
            "constraint",
            "timeout",
        ]) {
            assert_eq!(fields.len(), 3);
            let category_field = fields.iter().find(|(name, _)| name == "category");
            assert_eq!(
                category_field.map(|(_, value)| value.as_str()),
                Some(category)
            );
            let trace_id = fields.iter().find(|(name, _)| name == "error_trace_id");
            let trace_id = trace_id
                .map(|(_, value)| value.as_str())
                .unwrap_or_default();
            assert!(uuid::Uuid::parse_str(trace_id).is_ok());
            assert!(trace_ids.insert(trace_id));
            let message = fields.iter().find(|(name, _)| name == "message");
            assert_eq!(
                message.map(|(_, value)| value.as_str()),
                Some("Database operation failed")
            );
            let rendered = format!("{fields:?}");
            assert!(!rendered.contains(sentinel));
            assert!(!rendered.contains("TEST-CLINICAL"));
            assert!(!rendered.contains("TEST-SECRET"));
        }
    }

    #[test]
    fn storage_errors_emit_safe_category_and_error_trace_id() {
        let events = CapturedEvents::default();
        let sentinel = "bucket=TEST-BUCKET;key=TEST-CLINICAL;credential=TEST-SECRET";
        tracing::subscriber::with_default(events.clone(), || {
            for error in [
                opendal::Error::new(opendal::ErrorKind::Unexpected, sentinel),
                opendal::Error::new(opendal::ErrorKind::NotFound, sentinel),
                opendal::Error::new(opendal::ErrorKind::PermissionDenied, sentinel),
                opendal::Error::new(opendal::ErrorKind::ConfigInvalid, sentinel),
                opendal::Error::new(opendal::ErrorKind::RateLimited, sentinel).set_temporary(),
            ] {
                let operation_error = OperationError::from(error);
                assert_eq!(operation_error, OperationError::Unavailable);
                assert_eq!(
                    operation_error.to_string(),
                    "Operation temporarily unavailable"
                );
            }
        });

        let Ok(recorded) = events.0.lock() else {
            panic!("captured event lock poisoned");
        };
        assert_eq!(recorded.len(), 5);
        let mut trace_ids = std::collections::HashSet::new();
        for (fields, category) in recorded.iter().zip([
            "storage",
            "not_found",
            "permission",
            "configuration",
            "transient",
        ]) {
            assert_eq!(fields.len(), 3);
            let category_field = fields.iter().find(|(name, _)| name == "category");
            assert_eq!(
                category_field.map(|(_, value)| value.as_str()),
                Some(category)
            );
            let trace_id = fields.iter().find(|(name, _)| name == "error_trace_id");
            let trace_id = trace_id
                .map(|(_, value)| value.as_str())
                .unwrap_or_default();
            assert!(uuid::Uuid::parse_str(trace_id).is_ok());
            assert!(trace_ids.insert(trace_id));
            let message = fields.iter().find(|(name, _)| name == "message");
            assert_eq!(
                message.map(|(_, value)| value.as_str()),
                Some("Storage operation failed")
            );
            let rendered = format!("{fields:?}");
            assert!(!rendered.contains(sentinel));
            assert!(!rendered.contains("TEST-CLINICAL"));
            assert!(!rendered.contains("TEST-SECRET"));
            assert!(!rendered.contains("TEST-BUCKET"));
        }
    }

    #[tokio::test]
    async fn timed_out_operations_emit_safe_category_and_error_trace_id() {
        let elapsed = tokio::time::timeout(std::time::Duration::ZERO, std::future::pending::<()>())
            .await
            .unwrap_err();
        let Ok((result, recorded)) = capture_operation(|| OperationError::from(elapsed)) else {
            panic!("captured event lock poisoned");
        };
        assert_eq!(result, OperationError::Unavailable);
        assert_eq!(recorded.len(), 1);
        let fields = &recorded[0];
        let category = fields.iter().find(|(name, _)| name == "category");
        assert_eq!(category.map(|(_, value)| value.as_str()), Some("timeout"));
        let trace_id = fields.iter().find(|(name, _)| name == "error_trace_id");
        let trace_id = trace_id
            .map(|(_, value)| value.as_str())
            .unwrap_or_default();
        assert!(uuid::Uuid::parse_str(trace_id).is_ok());
    }

    #[test]
    fn unreported_failures_emit_safe_category_and_error_trace_id() {
        let Ok((result, recorded)) = capture_operation(|| OperationError::unavailable("storage"))
        else {
            panic!("captured event lock poisoned");
        };
        assert_eq!(result, OperationError::Unavailable);
        assert_eq!(recorded.len(), 1);
        let fields = &recorded[0];
        assert_eq!(fields.len(), 3);
        let category = fields.iter().find(|(name, _)| name == "category");
        assert_eq!(category.map(|(_, value)| value.as_str()), Some("storage"));
        let trace_id = fields.iter().find(|(name, _)| name == "error_trace_id");
        let trace_id = trace_id
            .map(|(_, value)| value.as_str())
            .unwrap_or_default();
        assert!(uuid::Uuid::parse_str(trace_id).is_ok());
    }

    #[test]
    fn sqlstate_classification_uses_only_fixed_categories() {
        assert_eq!(super::sqlstate_category("23505"), Some("constraint"));
        assert_eq!(super::sqlstate_category("23514"), Some("constraint"));
        assert_eq!(super::sqlstate_category("08006"), Some("connection"));
        assert_eq!(super::sqlstate_category("57P01"), Some("connection"));
        assert_eq!(super::sqlstate_category("57014"), None);
        assert_eq!(super::sqlstate_category("TEST-SECRET"), None);
    }
}
