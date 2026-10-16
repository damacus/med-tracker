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
mod tests {
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
    fn sqlstate_classification_uses_only_fixed_categories() {
        assert_eq!(super::sqlstate_category("23505"), Some("constraint"));
        assert_eq!(super::sqlstate_category("23514"), Some("constraint"));
        assert_eq!(super::sqlstate_category("08006"), Some("connection"));
        assert_eq!(super::sqlstate_category("57P01"), Some("connection"));
        assert_eq!(super::sqlstate_category("57014"), None);
        assert_eq!(super::sqlstate_category("TEST-SECRET"), None);
    }
}
