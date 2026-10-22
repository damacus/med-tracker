use super::*;
use base64::Engine as _;
use loco_rs::model::TenantQueryExt;
use sea_orm::{Condition, ConnectionTrait, DbBackend, QuerySelect, Statement, sea_query::Expr};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

const PAGE_SIZE: usize = 50;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(transparent)]
pub struct InvitationId(i64);

#[derive(Clone, Copy, Debug)]
pub struct BrowseCursor {
    created_at: NaiveDateTime,
    id: InvitationId,
    filters: [u8; 32],
}

impl BrowseCursor {
    pub fn parse(encoded: &str) -> Result<Self, OperationError> {
        if encoded.len() > 256 {
            return Err(invalid("cursor", "is invalid"));
        }
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| invalid("cursor", "is invalid"))?;
        let value = std::str::from_utf8(&bytes).map_err(|_| invalid("cursor", "is invalid"))?;
        let (created_at, id) = value
            .split_once('|')
            .ok_or_else(|| invalid("cursor", "is invalid"))?;
        let created_at = NaiveDateTime::parse_from_str(created_at, "%Y-%m-%d %H:%M:%S%.f")
            .map_err(|_| invalid("cursor", "is invalid"))?;
        let (id, filters) = id
            .split_once('|')
            .ok_or_else(|| invalid("cursor", "is invalid"))?;
        let filters: [u8; 32] = hex::decode(filters)
            .map_err(|_| invalid("cursor", "is invalid"))?
            .try_into()
            .map_err(|_| invalid("cursor", "is invalid"))?;
        let id = id
            .parse::<i64>()
            .map_err(|_| invalid("cursor", "is invalid"))?;
        if id <= 0 {
            return Err(invalid("cursor", "is invalid"));
        }
        Ok(Self {
            created_at,
            id: InvitationId(id),
            filters,
        })
    }

    pub fn validate(
        &self,
        household_id: i64,
        filters: &BrowseFilters,
    ) -> Result<(), OperationError> {
        if self.filters != filters.fingerprint(household_id) {
            return Err(invalid("cursor", "is invalid"));
        }
        Ok(())
    }

    pub fn encode(self) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!(
            "{}|{}|{}",
            self.created_at,
            self.id.0,
            hex::encode(self.filters)
        ))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Revoked,
    Expired,
}

impl InvitationStatus {
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
        }
    }

    #[must_use]
    pub const fn actionable(self) -> bool {
        matches!(self, Self::Pending | Self::Expired)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum InvitationRole {
    Member,
    Administrator,
}

impl InvitationRole {
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Member => "member",
            Self::Administrator => "administrator",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BrowseFilters {
    pub search: String,
    pub role: Option<InvitationRole>,
    pub status: Option<InvitationStatus>,
}

impl BrowseFilters {
    pub fn parse(search: &str, role: &str, status: &str) -> Result<Self, OperationError> {
        if search.len() > 254 {
            return Err(invalid("q", "is invalid"));
        }
        Ok(Self {
            search: search.into(),
            role: match role {
                "" => None,
                "member" => Some(InvitationRole::Member),
                "administrator" => Some(InvitationRole::Administrator),
                _ => return Err(invalid("role", "is invalid")),
            },
            status: match status {
                "" => None,
                "pending" => Some(InvitationStatus::Pending),
                "accepted" => Some(InvitationStatus::Accepted),
                "revoked" => Some(InvitationStatus::Revoked),
                "expired" => Some(InvitationStatus::Expired),
                _ => return Err(invalid("status", "is invalid")),
            },
        })
    }

    fn fingerprint(&self, household_id: i64) -> [u8; 32] {
        Sha256::digest(format!(
            "{household_id}|{}|{}|{}",
            self.search.to_lowercase(),
            self.role.map_or("", InvitationRole::key),
            self.status.map_or("", InvitationStatus::key)
        ))
        .into()
    }
}

#[derive(Clone, Copy)]
pub enum DeliveryStatus {
    Unknown,
    Queued,
    SentToProvider,
    Failed,
}
impl DeliveryStatus {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Queued => "queued",
            Self::SentToProvider => "sent_to_provider",
            Self::Failed => "failed",
        }
    }
}

pub use super::delivery::FailureCategory;

pub struct DeliveryAttempt {
    pub status: DeliveryStatus,
    pub category: Option<FailureCategory>,
    pub at: Option<chrono::DateTime<Utc>>,
}

pub struct InvitationRow {
    pub id: InvitationId,
    pub email: String,
    pub membership_role: InvitationRole,
    pub status: InvitationStatus,
    pub expires_at: chrono::DateTime<Utc>,
    pub created_at: chrono::DateTime<Utc>,
    pub delivery: DeliveryAttempt,
}

pub struct BrowsePage {
    pub rows: Vec<InvitationRow>,
    pub next_cursor: Option<String>,
}

pub async fn browse(
    tenant: &TenantTransaction,
    before: Option<BrowseCursor>,
    filters: &BrowseFilters,
) -> Result<BrowsePage, OperationError> {
    administration::authorize(tenant).await?;
    let mut query = household_invitation::Entity::find().in_tenant(tenant.scope().household_id);
    if !filters.search.is_empty() {
        query = query.filter(Expr::cust_with_values(
            "strpos(lower(email),lower($1))>0",
            [filters.search.clone()],
        ));
    }
    if let Some(role) = filters.role {
        query = query.filter(household_invitation::Column::MembershipRole.eq(role.key()));
    }
    let now = Utc::now().naive_utc();
    if let Some(status) = filters.status {
        query = match status {
            InvitationStatus::Accepted => {
                query.filter(household_invitation::Column::AcceptedAt.is_not_null())
            }
            InvitationStatus::Revoked => query
                .filter(household_invitation::Column::AcceptedAt.is_null())
                .filter(household_invitation::Column::RevokedAt.is_not_null()),
            InvitationStatus::Pending | InvitationStatus::Expired => {
                let query = query
                    .filter(household_invitation::Column::AcceptedAt.is_null())
                    .filter(household_invitation::Column::RevokedAt.is_null());
                if matches!(status, InvitationStatus::Pending) {
                    query.filter(household_invitation::Column::ExpiresAt.gt(now))
                } else {
                    query.filter(household_invitation::Column::ExpiresAt.lte(now))
                }
            }
        };
    }
    let fingerprint = filters.fingerprint(tenant.scope().household_id);
    if let Some(before) = before {
        before.validate(tenant.scope().household_id, filters)?;
        query = query.filter(
            Condition::any()
                .add(household_invitation::Column::CreatedAt.lt(before.created_at))
                .add(
                    Condition::all()
                        .add(household_invitation::Column::CreatedAt.eq(before.created_at))
                        .add(household_invitation::Column::Id.lt(before.id.0)),
                ),
        );
    }
    let mut rows = query
        .order_by_desc(household_invitation::Column::CreatedAt)
        .order_by_desc(household_invitation::Column::Id)
        .limit(51)
        .all(tenant.transaction())
        .await?;
    let has_next = rows.len() > PAGE_SIZE;
    rows.truncate(PAGE_SIZE);
    let next_cursor = if has_next {
        rows.last().map(|row| {
            BrowseCursor {
                created_at: row.created_at,
                id: InvitationId(row.id),
                filters: fingerprint,
            }
            .encode()
        })
    } else {
        None
    };
    let mut attempts = HashMap::new();
    if !rows.is_empty() {
        let ids: Vec<i64> = rows.iter().map(|row| row.id).collect();
        let records=tenant.transaction().query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT DISTINCT ON (original_invitation_id) original_invitation_id,status,category,coalesce(finished_at,started_at,queued_at) AS at FROM invitation_mail_attempts WHERE household_id=$1 AND original_invitation_id=ANY($2) ORDER BY original_invitation_id,id DESC",
            [tenant.scope().household_id.into(),ids.into()],
        )).await?;
        for record in records {
            let id: i64 = record.try_get("", "original_invitation_id")?;
            let status: String = record.try_get("", "status")?;
            let category: Option<String> = record.try_get("", "category")?;
            let at: NaiveDateTime = record.try_get("", "at")?;
            attempts.insert(
                id,
                DeliveryAttempt {
                    status: match status.as_str() {
                        "queued" => DeliveryStatus::Queued,
                        "sent_to_provider" => DeliveryStatus::SentToProvider,
                        "failed" => DeliveryStatus::Failed,
                        _ => return Err(OperationError::Unavailable),
                    },
                    category: category
                        .as_deref()
                        .map(FailureCategory::parse)
                        .transpose()?,
                    at: Some(at.and_utc()),
                },
            );
        }
    }
    let rows = rows
        .into_iter()
        .map(|row| {
            Ok(InvitationRow {
                id: InvitationId(row.id),
                email: row.email,
                membership_role: match row.membership_role.as_str() {
                    "member" => InvitationRole::Member,
                    "administrator" => InvitationRole::Administrator,
                    _ => return Err(OperationError::Unavailable),
                },
                status: if row.accepted_at.is_some() {
                    InvitationStatus::Accepted
                } else if row.revoked_at.is_some() {
                    InvitationStatus::Revoked
                } else if row.expires_at > now {
                    InvitationStatus::Pending
                } else {
                    InvitationStatus::Expired
                },
                expires_at: row.expires_at.and_utc(),
                created_at: row.created_at.and_utc(),
                delivery: attempts.remove(&row.id).unwrap_or(DeliveryAttempt {
                    status: DeliveryStatus::Unknown,
                    category: None,
                    at: None,
                }),
            })
        })
        .collect::<Result<Vec<_>, OperationError>>()?;
    Ok(BrowsePage { rows, next_cursor })
}
