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
    fn from(_: sea_orm::DbErr) -> Self {
        Self::Unavailable
    }
}
