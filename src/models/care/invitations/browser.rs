use super::*;
use base64::Engine as _;
use loco_rs::model::TenantQueryExt;
use sea_orm::{Condition, QuerySelect};
use serde::Serialize;

const PAGE_SIZE: usize = 50;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(transparent)]
pub struct InvitationId(i64);

#[derive(Clone, Copy, Debug)]
pub struct BrowseCursor {
    created_at: NaiveDateTime,
    id: InvitationId,
}

impl BrowseCursor {
    pub fn parse(encoded: &str) -> Result<Self, OperationError> {
        if encoded.len() > 128 {
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
        let id = id
            .parse::<i64>()
            .map_err(|_| invalid("cursor", "is invalid"))?;
        if id <= 0 {
            return Err(invalid("cursor", "is invalid"));
        }
        Ok(Self {
            created_at,
            id: InvitationId(id),
        })
    }

    fn encode(self) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(format!("{}|{}", self.created_at, self.id.0))
    }
}

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
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

pub struct InvitationRow {
    pub id: InvitationId,
    pub email: String,
    pub membership_role: InvitationRole,
    pub status: InvitationStatus,
    pub expires_at: chrono::DateTime<Utc>,
}

pub struct BrowsePage {
    pub rows: Vec<InvitationRow>,
    pub next_cursor: Option<String>,
}

pub async fn browse(
    tenant: &TenantTransaction,
    before: Option<BrowseCursor>,
) -> Result<BrowsePage, OperationError> {
    administration::authorize(tenant).await?;
    let mut query = household_invitation::Entity::find().in_tenant(tenant.scope().household_id);
    if let Some(before) = before {
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
            }
            .encode()
        })
    } else {
        None
    };
    let now = Utc::now().naive_utc();
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
            })
        })
        .collect::<Result<Vec<_>, OperationError>>()?;
    Ok(BrowsePage { rows, next_cursor })
}
