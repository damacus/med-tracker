use super::*;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{NaiveDate, NaiveDateTime};
use sea_orm::{Condition, FromQueryResult};
use serde::{Deserialize, Serialize};

pub async fn index(tenant: &TenantTransaction) -> Result<Vec<Value>, OperationError> {
    access::recheck(tenant).await?;
    if !access::can_manage_household(tenant) {
        return Err(OperationError::Forbidden);
    }
    let events = security_audit_event::Entity::find()
        .filter(security_audit_event::Column::HouseholdId.eq(tenant.scope().household_id))
        .order_by_desc(security_audit_event::Column::CreatedAt)
        .order_by_desc(security_audit_event::Column::Id)
        .limit(100)
        .all(tenant.transaction())
        .await
        .map_err(|_| OperationError::Unavailable)?;
    Ok(events
        .iter()
        .map(|event| {
            json!({
                "id": event.id,
                "event_type": event.event_type,
                "actor_account_id": event.actor_account_id,
                "actor_membership_id": event.actor_membership_id,
                "request_id": event.request_id,
                "metadata": event.metadata,
                "created_at": event.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string()
            })
        })
        .collect())
}

const BROWSER_PAGE_SIZE: usize = 50;

#[derive(Clone, Debug, Eq, PartialEq)]
struct EventType(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuditEventId(i64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowseFilter {
    event_type: Option<EventType>,
    from_utc: Option<NaiveDate>,
    through_utc: Option<NaiveDate>,
    before: Option<BrowseCursor>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BrowseCursor {
    version: u8,
    event_type: Option<String>,
    from_utc: Option<NaiveDate>,
    through_utc: Option<NaiveDate>,
    created_at: NaiveDateTime,
    id: i64,
}

#[derive(Serialize)]
pub struct BrowseFilterView {
    pub event_type: String,
    pub from_utc: String,
    pub through_utc: String,
}

#[derive(Serialize)]
pub struct BrowsePage {
    pub rows: Vec<AuditSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Serialize)]
pub struct AuditSummary {
    pub id: i64,
    pub event_type: String,
    pub event_label: String,
    pub occurred_at_utc: String,
    pub actor: String,
    pub actor_account_id: Option<i64>,
    pub actor_membership_id: Option<i64>,
}

#[derive(Serialize)]
pub struct AuditDetail {
    pub summary: AuditSummary,
    pub request_id: Option<String>,
    pub fields: Vec<AuditField>,
}

#[derive(Clone, Copy, Serialize)]
pub enum AuditFieldLabel {
    #[serde(rename = "Method")]
    Method,
    #[serde(rename = "Controller")]
    Controller,
    #[serde(rename = "Action")]
    Action,
    #[serde(rename = "Outcome")]
    Outcome,
    #[serde(rename = "HTTP status")]
    HttpStatus,
    #[serde(rename = "Target account")]
    TargetAccount,
    #[serde(rename = "Target membership")]
    TargetMembership,
    #[serde(rename = "Previous role")]
    PreviousRole,
    #[serde(rename = "New role")]
    NewRole,
    #[serde(rename = "Previous status")]
    PreviousStatus,
    #[serde(rename = "New status")]
    NewStatus,
}

impl AuditFieldLabel {
    #[must_use]
    pub const fn translation_key(self) -> &'static str {
        match self {
            Self::Method => "method",
            Self::Controller => "controller",
            Self::Action => "action",
            Self::Outcome => "outcome",
            Self::HttpStatus => "http_status",
            Self::TargetAccount => "target_account",
            Self::TargetMembership => "target_membership",
            Self::PreviousRole => "previous_role",
            Self::NewRole => "new_role",
            Self::PreviousStatus => "previous_status",
            Self::NewStatus => "new_status",
        }
    }
}

#[derive(Serialize)]
pub struct AuditField {
    pub label: AuditFieldLabel,
    pub value: String,
}

#[derive(FromQueryResult)]
struct SummaryRow {
    id: i64,
    event_type: String,
    actor_account_id: Option<i64>,
    actor_membership_id: Option<i64>,
    created_at: NaiveDateTime,
}

#[derive(FromQueryResult)]
struct DetailRow {
    id: i64,
    event_type: String,
    actor_account_id: Option<i64>,
    actor_membership_id: Option<i64>,
    created_at: NaiveDateTime,
    request_id: Option<String>,
    metadata: Value,
}

impl BrowseFilter {
    pub fn parse(
        event_type: Option<&str>,
        from_utc: Option<&str>,
        through_utc: Option<&str>,
        cursor: Option<&str>,
    ) -> Result<Self, OperationError> {
        let event_type = event_type
            .filter(|value| !value.is_empty())
            .map(EventType::parse)
            .transpose()?;
        let from_utc = parse_date(from_utc, "from")?;
        let through_utc = parse_date(through_utc, "to")?;
        if from_utc
            .zip(through_utc)
            .is_some_and(|(from, through)| from > through)
            || through_utc.is_some_and(|through| through.succ_opt().is_none())
        {
            return Err(invalid("to", "is invalid"));
        }
        let mut filter = Self {
            event_type,
            from_utc,
            through_utc,
            before: None,
        };
        if let Some(encoded) = cursor {
            if encoded.is_empty() || encoded.len() > 512 {
                return Err(invalid("cursor", "is invalid"));
            }
            let bytes = URL_SAFE_NO_PAD
                .decode(encoded)
                .map_err(|_| invalid("cursor", "is invalid"))?;
            let before: BrowseCursor =
                serde_json::from_slice(&bytes).map_err(|_| invalid("cursor", "is invalid"))?;
            if before.version != 1
                || before.id <= 0
                || before.event_type != filter.event_type.as_ref().map(|value| value.0.clone())
                || before.from_utc != filter.from_utc
                || before.through_utc != filter.through_utc
            {
                return Err(invalid("cursor", "is invalid"));
            }
            filter.before = Some(before);
        }
        Ok(filter)
    }

    pub fn view(&self) -> BrowseFilterView {
        BrowseFilterView {
            event_type: self
                .event_type
                .as_ref()
                .map(|value| value.0.clone())
                .unwrap_or_default(),
            from_utc: self
                .from_utc
                .map(|date| date.format("%Y-%m-%d").to_string())
                .unwrap_or_default(),
            through_utc: self
                .through_utc
                .map(|date| date.format("%Y-%m-%d").to_string())
                .unwrap_or_default(),
        }
    }

    fn cursor_for(&self, row: &SummaryRow) -> Result<String, OperationError> {
        let cursor = BrowseCursor {
            version: 1,
            event_type: self.event_type.as_ref().map(|value| value.0.clone()),
            from_utc: self.from_utc,
            through_utc: self.through_utc,
            created_at: row.created_at,
            id: row.id,
        };
        serde_json::to_vec(&cursor)
            .map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
            .map_err(|_| OperationError::Unavailable)
    }
}

impl EventType {
    fn parse(value: &str) -> Result<Self, OperationError> {
        if value.is_empty()
            || value.len() > 128
            || !value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'/' | b'-')
            })
        {
            return Err(invalid("event_type", "is invalid"));
        }
        Ok(Self(value.to_owned()))
    }
}

impl AuditEventId {
    pub fn parse(raw: &str) -> Result<Self, OperationError> {
        raw.parse::<i64>()
            .ok()
            .filter(|id| *id > 0)
            .map(Self)
            .ok_or(OperationError::NotFound)
    }
}

fn parse_date(raw: Option<&str>, field: &str) -> Result<Option<NaiveDate>, OperationError> {
    raw.filter(|value| !value.is_empty())
        .map(|value| {
            let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|_| invalid(field, "is invalid"))?;
            if date.format("%Y-%m-%d").to_string() != value {
                return Err(invalid(field, "is invalid"));
            }
            Ok(date)
        })
        .transpose()
}

async fn authorize_browse(tenant: &TenantTransaction) -> Result<(), OperationError> {
    access::recheck(tenant).await?;
    if !access::can_manage_household(tenant) {
        return Err(OperationError::Forbidden);
    }
    Ok(())
}

pub async fn browse(
    tenant: &TenantTransaction,
    filter: &BrowseFilter,
) -> Result<BrowsePage, OperationError> {
    authorize_browse(tenant).await?;
    let mut query = security_audit_event::Entity::find()
        .select_only()
        .columns([
            security_audit_event::Column::Id,
            security_audit_event::Column::EventType,
            security_audit_event::Column::ActorAccountId,
            security_audit_event::Column::ActorMembershipId,
            security_audit_event::Column::CreatedAt,
        ])
        .filter(security_audit_event::Column::HouseholdId.eq(tenant.scope().household_id));
    if let Some(event_type) = &filter.event_type {
        query = query.filter(security_audit_event::Column::EventType.eq(event_type.0.clone()));
    }
    if let Some(from) = filter.from_utc {
        let lower = from
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| invalid("from", "is invalid"))?;
        query = query.filter(security_audit_event::Column::CreatedAt.gte(lower));
    }
    if let Some(through) = filter.through_utc {
        let upper = through
            .succ_opt()
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .ok_or_else(|| invalid("to", "is invalid"))?;
        query = query.filter(security_audit_event::Column::CreatedAt.lt(upper));
    }
    if let Some(before) = &filter.before {
        query = query.filter(
            Condition::any()
                .add(security_audit_event::Column::CreatedAt.lt(before.created_at))
                .add(
                    Condition::all()
                        .add(security_audit_event::Column::CreatedAt.eq(before.created_at))
                        .add(security_audit_event::Column::Id.lt(before.id)),
                ),
        );
    }
    let mut rows: Vec<SummaryRow> = query
        .order_by_desc(security_audit_event::Column::CreatedAt)
        .order_by_desc(security_audit_event::Column::Id)
        .limit(51)
        .into_model::<SummaryRow>()
        .all(tenant.transaction())
        .await?;
    let has_next = rows.len() > BROWSER_PAGE_SIZE;
    rows.truncate(BROWSER_PAGE_SIZE);
    let next_cursor = if has_next {
        rows.last().map(|row| filter.cursor_for(row)).transpose()?
    } else {
        None
    };
    Ok(BrowsePage {
        rows: rows.into_iter().map(summary).collect(),
        next_cursor,
    })
}

pub async fn recent(tenant: &TenantTransaction) -> Result<Vec<AuditSummary>, OperationError> {
    authorize_browse(tenant).await?;
    let rows: Vec<SummaryRow> = security_audit_event::Entity::find()
        .select_only()
        .columns([
            security_audit_event::Column::Id,
            security_audit_event::Column::EventType,
            security_audit_event::Column::ActorAccountId,
            security_audit_event::Column::ActorMembershipId,
            security_audit_event::Column::CreatedAt,
        ])
        .filter(security_audit_event::Column::HouseholdId.eq(tenant.scope().household_id))
        .order_by_desc(security_audit_event::Column::CreatedAt)
        .order_by_desc(security_audit_event::Column::Id)
        .limit(5)
        .into_model::<SummaryRow>()
        .all(tenant.transaction())
        .await?;
    Ok(rows.into_iter().map(summary).collect())
}

pub async fn detail(
    tenant: &TenantTransaction,
    id: AuditEventId,
) -> Result<AuditDetail, OperationError> {
    authorize_browse(tenant).await?;
    let row: DetailRow = security_audit_event::Entity::find()
        .select_only()
        .columns([
            security_audit_event::Column::Id,
            security_audit_event::Column::EventType,
            security_audit_event::Column::ActorAccountId,
            security_audit_event::Column::ActorMembershipId,
            security_audit_event::Column::CreatedAt,
            security_audit_event::Column::RequestId,
            security_audit_event::Column::Metadata,
        ])
        .filter(security_audit_event::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(security_audit_event::Column::Id.eq(id.0))
        .into_model::<DetailRow>()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    Ok(AuditDetail {
        summary: make_summary(
            row.id,
            row.event_type.clone(),
            row.actor_account_id,
            row.actor_membership_id,
            row.created_at,
        ),
        request_id: safe_request_id(row.request_id),
        fields: safe_fields(&row.event_type, &row.metadata),
    })
}

fn summary(row: SummaryRow) -> AuditSummary {
    make_summary(
        row.id,
        row.event_type,
        row.actor_account_id,
        row.actor_membership_id,
        row.created_at,
    )
}

fn safe_request_id(raw: Option<String>) -> Option<String> {
    raw.and_then(|value| uuid::Uuid::parse_str(&value).ok())
        .map(|value| value.hyphenated().to_string())
}

fn make_summary(
    id: i64,
    event_type: String,
    actor_account_id: Option<i64>,
    actor_membership_id: Option<i64>,
    created_at: NaiveDateTime,
) -> AuditSummary {
    let actor = match (actor_account_id, actor_membership_id) {
        (Some(account), Some(membership)) => {
            format!("Account #{account}, membership #{membership}")
        }
        (Some(account), None) => format!("Account #{account}"),
        (None, Some(membership)) => format!("Membership #{membership}"),
        (None, None) => "System".to_owned(),
    };
    let event_label = match event_type.as_str() {
        "api.request" => "API request",
        "household_membership.role_updated" => "Household role changed",
        "household_access.membership_changed" => "Household membership changed",
        "household_access.membership_created" => "Household membership created",
        _ => &event_type,
    };
    AuditSummary {
        id,
        event_label: event_label.to_owned(),
        event_type,
        occurred_at_utc: created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        actor,
        actor_account_id,
        actor_membership_id,
    }
}

fn safe_fields(event_type: &str, metadata: &Value) -> Vec<AuditField> {
    let Some(fields) = metadata.as_object() else {
        return Vec::new();
    };
    let mut safe = Vec::new();
    match event_type {
        "api.request" => {
            add_known(
                fields,
                &mut safe,
                "http_method",
                AuditFieldLabel::Method,
                &["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE"],
            );
            if let Some(controller) =
                fields
                    .get("controller")
                    .and_then(Value::as_str)
                    .filter(|controller| {
                        matches!(
                            *controller,
                            "api/v1/me"
                                | "api/v1/ui_capabilities"
                                | "api/v1/admin/settings"
                                | "api/v1/admin/memberships"
                                | "api/v1/admin/audit_logs"
                                | "api/v1/admin/app_tokens"
                                | "api/v1/admin/person_access_grants"
                                | "api/v1/admin/invitations"
                        )
                    })
            {
                safe.push(AuditField {
                    label: AuditFieldLabel::Controller,
                    value: controller.to_owned(),
                });
                add_known(
                    fields,
                    &mut safe,
                    "action",
                    AuditFieldLabel::Action,
                    &["index", "show", "create", "update", "destroy", "resend"],
                );
            }
            add_known(
                fields,
                &mut safe,
                "outcome",
                AuditFieldLabel::Outcome,
                &["success", "failure"],
            );
            if let Some(status) = fields
                .get("status")
                .and_then(Value::as_u64)
                .filter(|status| (100..=599).contains(status))
            {
                safe.push(AuditField {
                    label: AuditFieldLabel::HttpStatus,
                    value: status.to_string(),
                });
            }
        }
        "household_membership.role_updated"
        | "household_access.membership_changed"
        | "household_access.membership_created" => {
            add_id(
                fields,
                &mut safe,
                "target_account_id",
                AuditFieldLabel::TargetAccount,
            );
            add_id(
                fields,
                &mut safe,
                "target_membership_id",
                AuditFieldLabel::TargetMembership,
            );
            add_known(
                fields,
                &mut safe,
                "previous_role",
                AuditFieldLabel::PreviousRole,
                &["owner", "administrator", "member"],
            );
            add_known(
                fields,
                &mut safe,
                "new_role",
                AuditFieldLabel::NewRole,
                &["owner", "administrator", "member"],
            );
            for (key, role_label, status_label) in [
                (
                    "previous_state",
                    AuditFieldLabel::PreviousRole,
                    AuditFieldLabel::PreviousStatus,
                ),
                (
                    "new_state",
                    AuditFieldLabel::NewRole,
                    AuditFieldLabel::NewStatus,
                ),
            ] {
                if let Some(state) = fields.get(key).and_then(Value::as_object) {
                    if event_type == "household_access.membership_created" {
                        add_known(
                            state,
                            &mut safe,
                            "role",
                            role_label,
                            &["owner", "administrator", "member"],
                        );
                    }
                    add_known(
                        state,
                        &mut safe,
                        "status",
                        status_label,
                        &["active", "suspended", "revoked"],
                    );
                }
            }
            add_known(
                fields,
                &mut safe,
                "outcome",
                AuditFieldLabel::Outcome,
                &["success", "no_change", "rejected", "failure"],
            );
        }
        _ => {}
    }
    safe
}

fn add_id(
    fields: &serde_json::Map<String, Value>,
    safe: &mut Vec<AuditField>,
    key: &str,
    label: AuditFieldLabel,
) {
    if let Some(id) = fields.get(key).and_then(Value::as_i64).filter(|id| *id > 0) {
        safe.push(AuditField {
            label,
            value: id.to_string(),
        });
    }
}

fn add_known(
    fields: &serde_json::Map<String, Value>,
    safe: &mut Vec<AuditField>,
    key: &str,
    label: AuditFieldLabel,
    allowed: &[&str],
) {
    if let Some(value) = fields
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| allowed.contains(value))
    {
        safe.push(AuditField {
            label,
            value: value.to_owned(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_field_labels_keep_english_contract_and_have_all_locale_keys()
    -> Result<(), Box<dyn std::error::Error>> {
        for (field, english) in [
            (AuditFieldLabel::Method, "Method"),
            (AuditFieldLabel::Controller, "Controller"),
            (AuditFieldLabel::Action, "Action"),
            (AuditFieldLabel::Outcome, "Outcome"),
            (AuditFieldLabel::HttpStatus, "HTTP status"),
            (AuditFieldLabel::TargetAccount, "Target account"),
            (AuditFieldLabel::TargetMembership, "Target membership"),
            (AuditFieldLabel::PreviousRole, "Previous role"),
            (AuditFieldLabel::NewRole, "New role"),
            (AuditFieldLabel::PreviousStatus, "Previous status"),
            (AuditFieldLabel::NewStatus, "New status"),
        ] {
            assert_eq!(serde_json::to_string(&field)?, format!("\"{english}\""));
            for language in ["en", "cy", "ga", "pt", "es"] {
                let labels = crate::models::care::report_pdf::translations(language)
                    .map_err(|_| std::io::Error::other("bundled audit locale unavailable"))?;
                assert!(
                    labels["administration"]["audit"]["fields"][field.translation_key()]
                        .as_str()
                        .is_some_and(|label| !label.is_empty()),
                    "{language}: {english}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn browser_cursor_is_bounded_and_bound_to_exact_filters() -> Result<(), OperationError> {
        let filter = BrowseFilter::parse(
            Some("api.request"),
            Some("2030-01-01"),
            Some("2030-01-31"),
            None,
        )?;
        let created_at = NaiveDate::from_ymd_opt(2030, 1, 15)
            .and_then(|date| date.and_hms_opt(12, 0, 0))
            .ok_or(OperationError::Unavailable)?;
        let row = SummaryRow {
            id: 950001,
            event_type: "api.request".to_owned(),
            actor_account_id: Some(71001),
            actor_membership_id: Some(74001),
            created_at,
        };
        let cursor = filter.cursor_for(&row)?;
        assert!(
            BrowseFilter::parse(
                Some("api.request"),
                Some("2030-01-01"),
                Some("2030-01-31"),
                Some(&cursor)
            )
            .is_ok()
        );
        assert!(
            BrowseFilter::parse(
                Some("api.request"),
                Some("2030-01-02"),
                Some("2030-01-31"),
                Some(&cursor)
            )
            .is_err()
        );
        assert!(
            BrowseFilter::parse(
                Some("api.request"),
                Some("2030-01-01"),
                Some("2030-01-31"),
                Some("")
            )
            .is_err()
        );
        assert!(BrowseFilter::parse(None, None, None, Some(&"a".repeat(513))).is_err());
        assert!(BrowseFilter::parse(None, Some("2030-02-01"), Some("2030-01-31"), None).is_err());
        Ok(())
    }

    #[test]
    fn browser_detail_allows_only_known_fields_for_known_events() {
        assert_eq!(safe_request_id(Some("clinical-token-123".to_owned())), None);
        assert_eq!(
            safe_request_id(Some("550e8400-e29b-41d4-a716-446655440000".to_owned())),
            Some("550e8400-e29b-41d4-a716-446655440000".to_owned())
        );
        let metadata = json!({
            "target_membership_id": 74002,
            "previous_role": "member",
            "new_role": "administrator",
            "previous_state": {"status": "active", "clinical_note": "private previous note"},
            "new_state": {"status": "revoked", "clinical_note": "private new note"},
            "outcome": "success",
            "secret": "private token"
        });
        let fields = safe_fields("household_membership.role_updated", &metadata);
        let values = fields
            .iter()
            .map(|field| field.value.as_str())
            .collect::<Vec<_>>();
        assert!(values.contains(&"74002"));
        assert!(values.contains(&"administrator"));
        assert!(values.contains(&"revoked"));
        assert!(!values.iter().any(|value| value.contains("private")));
        assert!(safe_fields("unknown.event", &metadata).is_empty());

        let unsafe_request = json!({
            "http_method": "GET",
            "controller": "api/v1/admin/private-clinical-token",
            "action": "secret123",
            "outcome": "success",
            "status": 200
        });
        let fields = safe_fields("api.request", &unsafe_request);
        let values = fields
            .iter()
            .map(|field| field.value.as_str())
            .collect::<Vec<_>>();
        assert!(
            !values
                .iter()
                .any(|value| value.contains("private-clinical-token") || *value == "secret123")
        );
        assert!(values.contains(&"GET"));
        assert!(values.contains(&"200"));

        let known_request = json!({"controller":"api/v1/admin/audit_logs","action":"index"});
        let fields = safe_fields("api.request", &known_request);
        let values = fields
            .iter()
            .map(|field| field.value.as_str())
            .collect::<Vec<_>>();
        assert!(values.contains(&"api/v1/admin/audit_logs"));
        assert!(values.contains(&"index"));

        let created = json!({
            "target_membership_id": 74003,
            "new_state": {"role":"member","status":"active","private_note":"private clinical text"},
            "outcome": "success"
        });
        let fields = safe_fields("household_access.membership_created", &created);
        let values = fields
            .iter()
            .map(|field| field.value.as_str())
            .collect::<Vec<_>>();
        assert!(values.contains(&"74003"));
        assert!(values.contains(&"member"));
        assert!(values.contains(&"active"));
        assert!(!values.iter().any(|value| value.contains("private")));
    }
}
