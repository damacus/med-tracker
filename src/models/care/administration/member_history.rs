use super::{audit_logs, authorize, member_browser::MembershipId};
use crate::models::{access::TenantTransaction, errors::OperationError};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::NaiveDateTime;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryCursor {
    version: u8,
    membership_id: i64,
    created_at: NaiveDateTime,
    id: i64,
}
impl HistoryCursor {
    pub fn parse(raw: Option<&str>, member: MembershipId) -> Result<Option<Self>, OperationError> {
        let Some(raw) = raw else { return Ok(None) };
        let invalid = || OperationError::Validation {
            details: serde_json::json!({"errors":{"history_cursor":["is invalid"]}}),
        };
        if raw.len() > 512 {
            return Err(invalid());
        }
        let bytes = URL_SAFE_NO_PAD.decode(raw).map_err(|_| invalid())?;
        let cursor: Self = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
        if cursor.version != 1 || cursor.membership_id != member.get() || cursor.id <= 0 {
            return Err(invalid());
        }
        Ok(Some(cursor))
    }
    fn encode(self) -> Result<String, OperationError> {
        serde_json::to_vec(&self)
            .map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
            .map_err(|_| OperationError::Unavailable)
    }
}
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryKind {
    Role,
    Membership,
    Created,
}
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryOutcome {
    Success,
    NoChange,
    Rejected,
    Failure,
    Recorded,
}
pub struct HistoryEvent {
    pub occurred_at: NaiveDateTime,
    pub summary: audit_logs::AuditSummary,
    pub kind: HistoryKind,
    pub outcome: HistoryOutcome,
    pub actor_name: Option<String>,
    pub fields: Vec<audit_logs::AuditField>,
}
pub struct HistoryPage {
    pub events: Vec<HistoryEvent>,
    pub next_cursor: Option<String>,
}
pub async fn read(
    tenant: &TenantTransaction,
    member: MembershipId,
    before: Option<HistoryCursor>,
) -> Result<HistoryPage, OperationError> {
    authorize(tenant).await?;
    let rows=tenant.transaction().query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "WITH directory AS MATERIALIZED (SELECT * FROM med_tracker.member_directory_identity()), events AS (SELECT id,event_type,actor_account_id,actor_membership_id,created_at,metadata FROM security_audit_events WHERE household_id=$1 AND metadata->'target_membership_id'=to_jsonb($2::bigint) AND event_type IN ('household_membership.role_updated','household_access.membership_changed','household_access.membership_created') AND ($3::timestamp IS NULL OR (created_at,id)<($3,$4)) ORDER BY created_at DESC,id DESC LIMIT 26) SELECT events.*,directory.name AS actor_name FROM events LEFT JOIN directory ON directory.membership_id=events.actor_membership_id AND (events.actor_account_id IS NULL OR EXISTS(SELECT 1 FROM household_memberships actor JOIN accounts account ON account.person_id=actor.person_id WHERE actor.id=events.actor_membership_id AND actor.household_id=$1 AND account.id=events.actor_account_id)) ORDER BY events.created_at DESC,events.id DESC",
        [tenant.scope().household_id.into(),member.get().into(),before.map(|cursor|cursor.created_at).into(),before.map_or(0,|cursor|cursor.id).into()]
    )).await?;
    let has_more = rows.len() > 25;
    let mut events = Vec::new();
    let mut last = None;
    for row in rows.into_iter().take(25) {
        let id: i64 = row.try_get("", "id")?;
        let created_at: NaiveDateTime = row.try_get("", "created_at")?;
        let event_type: String = row.try_get("", "event_type")?;
        let metadata: Value = row.try_get("", "metadata")?;
        let kind = match event_type.as_str() {
            "household_membership.role_updated" => HistoryKind::Role,
            "household_access.membership_changed" => HistoryKind::Membership,
            "household_access.membership_created" => HistoryKind::Created,
            _ => return Err(OperationError::Unavailable),
        };
        let outcome = match metadata.get("outcome").and_then(Value::as_str) {
            Some("success") => HistoryOutcome::Success,
            Some("no_change") => HistoryOutcome::NoChange,
            Some("rejected") => HistoryOutcome::Rejected,
            Some("failure") => HistoryOutcome::Failure,
            _ => HistoryOutcome::Recorded,
        };
        let fields = audit_logs::safe_fields(&event_type, &metadata);
        events.push(HistoryEvent {
            occurred_at: created_at,
            summary: audit_logs::make_summary(
                id,
                event_type,
                row.try_get("", "actor_account_id")?,
                row.try_get("", "actor_membership_id")?,
                created_at,
            ),
            kind,
            outcome,
            actor_name: row.try_get("", "actor_name")?,
            fields,
        });
        last = Some(HistoryCursor {
            version: 1,
            membership_id: member.get(),
            created_at,
            id,
        });
    }
    Ok(HistoryPage {
        events,
        next_cursor: if has_more {
            last.map(HistoryCursor::encode).transpose()?
        } else {
            None
        },
    })
}
