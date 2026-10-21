use crate::models::{
    access::{self, TenantTransaction},
    authorization,
    entities::person,
    errors::OperationError,
};
use chrono::Utc;
use sea_orm::{ColumnTrait, ConnectionTrait, DbBackend, QueryFilter, Statement};
use serde_json::json;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoverySubjectId(i64);

impl RecoverySubjectId {
    pub fn parse(id: i64) -> Result<Self, OperationError> {
        (id > 0).then_some(Self(id)).ok_or(OperationError::NotFound)
    }

    pub const fn get(self) -> i64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveryGuardianId(i64);

impl RecoveryGuardianId {
    pub fn parse(id: i64) -> Result<Self, OperationError> {
        (id > 0).then_some(Self(id)).ok_or(OperationError::NotFound)
    }

    pub const fn get(self) -> i64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoverySnapshot {
    pub token: Uuid,
    pub revision: i64,
    pub link_token: Uuid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RestorableLink {
    pub declaration_id: i64,
    pub relationship_id: i64,
    pub guardian_id: RecoveryGuardianId,
    pub declaration_revision: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverySubject {
    pub id: RecoverySubjectId,
    pub name: String,
    pub snapshot: RecoverySnapshot,
    pub has_effective_guardian: bool,
    pub restorable: Option<RestorableLink>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryGuardian {
    pub id: RecoveryGuardianId,
    pub name: String,
}

pub struct RecoveryOptions {
    pub subjects: Vec<RecoverySubject>,
    pub subjects_has_more: bool,
    pub guardians: Vec<RecoveryGuardian>,
    pub guardians_has_more: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryAction {
    Restore {
        declaration_id: i64,
        relationship_id: i64,
    },
    Appoint,
}

pub struct RecoveryCommand {
    pub subject: RecoverySubjectId,
    pub guardian: RecoveryGuardianId,
    pub expected: RecoverySnapshot,
    pub action: RecoveryAction,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryOutcome {
    Restored,
    Appointed,
}

pub struct RecoveryResult {
    pub outcome: RecoveryOutcome,
    pub declaration_id: i64,
    pub relationship_id: i64,
}

fn conflict(code: &str) -> OperationError {
    OperationError::Conflict {
        code: code.to_owned(),
        details: json!({}),
    }
}

pub async fn options(tenant: &TenantTransaction) -> Result<RecoveryOptions, OperationError> {
    access::recheck(tenant).await?;
    if !access::can_manage_household(tenant) {
        return Err(OperationError::Forbidden);
    }
    let subject_rows = tenant.transaction().query_all_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "WITH candidates AS MATERIALIZED (
           SELECT p.id,p.name FROM public.people p
           JOIN public.household_memberships m ON m.person_id=p.id
           WHERE m.household_id=$1 AND m.status='active' AND m.revoked_at IS NULL
             AND p.person_type=1 AND NOT p.has_capacity
             AND p.date_of_birth IS NOT NULL
             AND p.date_of_birth<=med_tracker.guardian_recovery_earliest_date()
             AND p.date_of_birth>(med_tracker.guardian_recovery_latest_date()-interval '18 years')::date
           ORDER BY p.name,p.id LIMIT 101)
         SELECT c.id,c.name,s.snapshot_token,s.snapshot_revision,s.link_token,s.has_effective_guardian
         FROM candidates c CROSS JOIN LATERAL med_tracker.guardian_recovery_snapshot(c.id) s
         ORDER BY c.name,c.id",
        [tenant.scope().household_id.into()],
    )).await?;
    let mut subjects = subject_rows
        .iter()
        .map(|row| {
            Ok(RecoverySubject {
                id: RecoverySubjectId::parse(row.try_get("", "id")?)?,
                name: row.try_get("", "name")?,
                snapshot: RecoverySnapshot {
                    token: row.try_get("", "snapshot_token")?,
                    revision: row.try_get("", "snapshot_revision")?,
                    link_token: row.try_get("", "link_token")?,
                },
                has_effective_guardian: row.try_get("", "has_effective_guardian")?,
                restorable: None,
            })
        })
        .collect::<Result<Vec<_>, OperationError>>()?;
    let subjects_has_more = subjects.len() > 100;
    subjects.truncate(100);
    let guardian_rows = tenant
        .transaction()
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT p.id,p.name FROM public.people p
         JOIN public.household_memberships m ON m.person_id=p.id
         JOIN public.accounts a ON a.person_id=p.id
         JOIN public.users u ON u.person_id=p.id
         WHERE m.household_id=$1 AND m.status='active' AND m.revoked_at IS NULL
           AND p.person_type=0 AND p.has_capacity
           AND (p.date_of_birth IS NULL OR p.date_of_birth<=
             (med_tracker.guardian_recovery_earliest_date()-interval '18 years')::date)
           AND a.status=2 AND u.active
           AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id
             AND l.deadline>pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()))
         ORDER BY p.name,p.id LIMIT 101",
            [tenant.scope().household_id.into()],
        ))
        .await?;
    let mut guardians = guardian_rows
        .iter()
        .map(|row| {
            Ok(RecoveryGuardian {
                id: RecoveryGuardianId::parse(row.try_get("", "id")?)?,
                name: row.try_get("", "name")?,
            })
        })
        .collect::<Result<Vec<_>, OperationError>>()?;
    let guardians_has_more = guardians.len() > 100;
    guardians.truncate(100);
    if !subjects.is_empty() {
        let ids = subjects.iter().map(|row| row.id.get()).collect::<Vec<_>>();
        let restorable = tenant
            .transaction()
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT d.subject_person_id,d.id AS declaration_id,d.revision,
                    d.guardian_person_id,c.id AS relationship_id
             FROM public.guardian_declarations d
             JOIN public.carer_relationships c ON c.guardian_declaration_id=d.id
             JOIN public.household_memberships m
               ON m.person_id=d.guardian_person_id AND m.household_id=c.household_id
             JOIN public.people guardian ON guardian.id=d.guardian_person_id
             JOIN public.accounts a ON a.person_id=guardian.id
             JOIN public.users u ON u.person_id=guardian.id
             WHERE d.subject_person_id=ANY($1::bigint[])
               AND d.withdrawn_at IS NULL AND d.superseded_at IS NULL
               AND c.household_id=$2 AND c.patient_id=d.subject_person_id
               AND c.carer_id=d.guardian_person_id AND NOT c.active
               AND m.status='active' AND m.revoked_at IS NULL
               AND guardian.person_type=0 AND guardian.has_capacity
               AND (guardian.date_of_birth IS NULL OR guardian.date_of_birth<=
                 (med_tracker.guardian_recovery_earliest_date()-interval '18 years')::date)
               AND a.status=2 AND u.active
               AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id
                 AND l.deadline>pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()))
             ORDER BY d.subject_person_id,c.id",
                [ids.into(), tenant.scope().household_id.into()],
            ))
            .await?;
        for row in restorable {
            let subject_id: i64 = row.try_get("", "subject_person_id")?;
            if let Some(subject) = subjects.iter_mut().find(|item| item.id.get() == subject_id) {
                subject.restorable = Some(RestorableLink {
                    declaration_id: row.try_get("", "declaration_id")?,
                    relationship_id: row.try_get("", "relationship_id")?,
                    guardian_id: RecoveryGuardianId::parse(row.try_get("", "guardian_person_id")?)?,
                    declaration_revision: row.try_get("", "revision")?,
                });
            }
        }
    }
    Ok(RecoveryOptions {
        subjects,
        subjects_has_more,
        guardians,
        guardians_has_more,
    })
}

pub async fn execute(
    tenant: &TenantTransaction,
    command: RecoveryCommand,
) -> Result<RecoveryResult, OperationError> {
    super::authorize(tenant).await?;
    let subject = access::person_queries::household_participants(tenant.scope().household_id)
        .filter(person::Column::Id.eq(command.subject.get()))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    if !authorization::may_recover_minor_guardian(tenant.membership(), &subject)
        || subject.date_of_birth.is_none_or(|date| {
            let age = super::super::people::age(date, Utc::now().date_naive());
            !(0..18).contains(&age)
        })
    {
        return Err(OperationError::Forbidden);
    }
    let reason = command.reason.trim();
    if !(10..=500).contains(&reason.chars().count()) {
        return Err(OperationError::Validation {
            details: json!({"errors":{"reason":["must be 10 to 500 characters"]}}),
        });
    }
    let (restore_declaration, restore_link) = match command.action {
        RecoveryAction::Restore {
            declaration_id,
            relationship_id,
        } => (Some(declaration_id), Some(relationship_id)),
        RecoveryAction::Appoint => (None, None),
    };
    let row = tenant
        .transaction()
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT outcome,declaration_id,relationship_id
         FROM med_tracker.guardian_recovery_command($1,$2,$3::uuid,$4,$5::uuid,$6,$7,$8,$9)",
            [
                command.subject.get().into(),
                command.guardian.get().into(),
                command.expected.token.to_string().into(),
                command.expected.revision.into(),
                command.expected.link_token.to_string().into(),
                restore_declaration.into(),
                restore_link.into(),
                reason.into(),
                tenant.scope().request_id.clone().into(),
            ],
        ))
        .await?
        .ok_or(OperationError::Unavailable)?;
    let outcome: String = row.try_get("", "outcome")?;
    let outcome = match outcome.as_str() {
        "restored" => RecoveryOutcome::Restored,
        "appointed" => RecoveryOutcome::Appointed,
        "stale" => return Err(conflict("stale_guardian_recovery")),
        "already_guarded" => return Err(conflict("guardian_already_effective")),
        "restore_available" => return Err(conflict("guardian_restore_available")),
        "invalid_guardian" => {
            return Err(OperationError::Validation {
                details: json!({"errors":{"guardian":["is no longer eligible"]}}),
            });
        }
        "invalid_restore" => return Err(conflict("guardian_restore_unavailable")),
        "invalid" => {
            return Err(OperationError::Validation {
                details: json!({"errors":{"recovery":["is invalid"]}}),
            });
        }
        _ => return Err(OperationError::Unavailable),
    };
    Ok(RecoveryResult {
        outcome,
        declaration_id: row.try_get("", "declaration_id")?,
        relationship_id: row.try_get("", "relationship_id")?,
    })
}
