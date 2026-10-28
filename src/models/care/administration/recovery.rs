use crate::models::{
    access::{self, TenantTransaction},
    authorization,
    entities::person,
    errors::OperationError,
};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, QueryFilter, Statement,
    sea_query::{
        Alias, CommonTableExpression, Condition, Expr, ExprTrait, Func, JoinType, Order, Query,
        SelectStatement,
    },
};
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestorableLink {
    pub declaration_id: i64,
    pub relationship_id: i64,
    pub guardian_id: RecoveryGuardianId,
    pub guardian_name: String,
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

// The database-owned snapshot is a guarded, volatile table function. SeaQuery builds
// the eligibility guard, joins, bounded result and command invocation around it.
fn recovery_statement(tenant: &TenantTransaction, query: &SelectStatement) -> Statement {
    tenant.transaction().get_database_backend().build(query)
}

fn birthday_cutoff(date_function: &'static str) -> Expr {
    Expr::from(Func::cust(date_function))
        .sub(Expr::from(
            Func::cust("pg_catalog.make_interval").arg(Expr::val(18)),
        ))
        .cast_as("date")
}

fn adult_cutoff() -> Expr {
    birthday_cutoff("med_tracker.guardian_recovery_earliest_date")
}

fn active_member(table: &'static str) -> Condition {
    Condition::all()
        .add(Expr::col((table, "status")).eq("active"))
        .add(Expr::col((table, "revoked_at")).is_null())
}

fn usable_account() -> Condition {
    let active_lockout = Query::select()
        .expr(1)
        .from_as((Alias::new("public"), Alias::new("account_lockouts")), "l")
        .and_where(Expr::col(("l", "account_id")).equals(("a", "id")))
        .and_where(Expr::col(("l", "deadline")).gt(Expr::from(
            Func::cust("pg_catalog.timezone").args(vec![
                Expr::val("UTC"),
                Expr::from(Func::cust("pg_catalog.clock_timestamp")),
            ]),
        )))
        .to_owned();
    Condition::all()
        .add(Expr::col(("a", "status")).eq(2))
        .add(Expr::col(("a", "active")).eq(true))
        .add(Expr::not_exists(active_lockout))
}

fn eligible_adult(table: &'static str) -> Condition {
    Condition::all()
        .add(Expr::col((table, "person_type")).eq(0))
        .add(Expr::col((table, "has_capacity")).eq(true))
        .add(
            Condition::any()
                .add(Expr::col((table, "date_of_birth")).is_null())
                .add(Expr::col((table, "date_of_birth")).lte(adult_cutoff())),
        )
}

fn guardian_options_query(household_id: i64) -> SelectStatement {
    Query::select()
        .columns([("p", "id"), ("p", "name")])
        .from_as((Alias::new("public"), Alias::new("people")), "p")
        .join_as(
            JoinType::InnerJoin,
            (Alias::new("public"), Alias::new("household_memberships")),
            "m",
            Expr::col(("m", "person_id")).equals(("p", "id")),
        )
        .join_as(
            JoinType::InnerJoin,
            (Alias::new("public"), Alias::new("accounts")),
            "a",
            Expr::col(("a", "person_id")).equals(("p", "id")),
        )
        .and_where(Expr::col(("m", "household_id")).eq(household_id))
        .cond_where(active_member("m"))
        .cond_where(eligible_adult("p"))
        .cond_where(usable_account())
        .order_by(("p", "name"), Order::Asc)
        .order_by(("p", "id"), Order::Asc)
        .limit(101)
        .to_owned()
}

fn restorable_query(household_id: i64, subject_ids: Vec<i64>) -> SelectStatement {
    Query::select()
        .columns([
            ("d", "subject_person_id"),
            ("d", "revision"),
            ("d", "guardian_person_id"),
        ])
        .expr_as(Expr::col(("d", "id")), "declaration_id")
        .expr_as(Expr::col(("guardian", "name")), "guardian_name")
        .expr_as(Expr::col(("c", "id")), "relationship_id")
        .from_as(
            (Alias::new("public"), Alias::new("guardian_declarations")),
            "d",
        )
        .join_as(
            JoinType::InnerJoin,
            (Alias::new("public"), Alias::new("carer_relationships")),
            "c",
            Expr::col(("c", "guardian_declaration_id")).equals(("d", "id")),
        )
        .join_as(
            JoinType::InnerJoin,
            (Alias::new("public"), Alias::new("household_memberships")),
            "m",
            Expr::col(("m", "person_id"))
                .equals(("d", "guardian_person_id"))
                .and(Expr::col(("m", "household_id")).equals(("c", "household_id"))),
        )
        .join_as(
            JoinType::InnerJoin,
            (Alias::new("public"), Alias::new("people")),
            "guardian",
            Expr::col(("guardian", "id")).equals(("d", "guardian_person_id")),
        )
        .join_as(
            JoinType::InnerJoin,
            (Alias::new("public"), Alias::new("accounts")),
            "a",
            Expr::col(("a", "person_id")).equals(("guardian", "id")),
        )
        .and_where(Expr::col(("d", "subject_person_id")).is_in(subject_ids))
        .and_where(Expr::col(("d", "withdrawn_at")).is_null())
        .and_where(Expr::col(("d", "superseded_at")).is_null())
        .and_where(Expr::col(("c", "household_id")).eq(household_id))
        .and_where(Expr::col(("c", "patient_id")).equals(("d", "subject_person_id")))
        .and_where(Expr::col(("c", "carer_id")).equals(("d", "guardian_person_id")))
        .and_where(Expr::col(("c", "active")).eq(false))
        .cond_where(active_member("m"))
        .cond_where(eligible_adult("guardian"))
        .cond_where(usable_account())
        .order_by(("d", "subject_person_id"), Order::Asc)
        .order_by(("c", "id"), Order::Asc)
        .to_owned()
}

fn eligible_subjects(household_id: i64) -> SelectStatement {
    Query::select()
        .columns([("p", "id"), ("p", "name")])
        .from_as((Alias::new("public"), Alias::new("people")), "p")
        .join_as(
            JoinType::InnerJoin,
            (Alias::new("public"), Alias::new("household_memberships")),
            "m",
            Expr::col(("m", "person_id")).equals(("p", "id")),
        )
        .and_where(Expr::col(("m", "household_id")).eq(household_id))
        .cond_where(active_member("m"))
        .and_where(Expr::col(("p", "person_type")).eq(1))
        .and_where(Expr::col(("p", "has_capacity")).eq(false))
        .and_where(Expr::col(("p", "date_of_birth")).is_not_null())
        .and_where(Expr::col(("p", "date_of_birth")).lte(Expr::from(Func::cust(
            "med_tracker.guardian_recovery_earliest_date",
        ))))
        .and_where(
            Expr::col(("p", "date_of_birth"))
                .gt(birthday_cutoff("med_tracker.guardian_recovery_latest_date")),
        )
        .to_owned()
}

fn subject_options_query(household_id: i64) -> SelectStatement {
    let candidates = CommonTableExpression::new()
        .table_name("candidates")
        .query(eligible_subjects(household_id))
        .materialized(true)
        .to_owned();
    Query::select()
        .columns([
            ("c", "id"),
            ("c", "name"),
            ("s", "snapshot_token"),
            ("s", "snapshot_revision"),
            ("s", "link_token"),
            ("s", "has_effective_guardian"),
        ])
        .from_as("candidates", "c")
        .from_function(
            Func::cust("med_tracker.guardian_recovery_snapshot").arg(Expr::col(("c", "id"))),
            "s",
        )
        .and_where(Expr::col(("s", "has_effective_guardian")).eq(false))
        .order_by(("c", "name"), Order::Asc)
        .order_by(("c", "id"), Order::Asc)
        .limit(101)
        .with_cte(candidates)
        .to_owned()
}

pub async fn options(tenant: &TenantTransaction) -> Result<RecoveryOptions, OperationError> {
    access::recheck(tenant).await?;
    if !access::can_manage_household(tenant) {
        return Err(OperationError::Forbidden);
    }
    let subject_rows = tenant
        .transaction()
        .query_all_raw(recovery_statement(
            tenant,
            &subject_options_query(tenant.scope().household_id),
        ))
        .await?;
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
        .query_all_raw(recovery_statement(
            tenant,
            &guardian_options_query(tenant.scope().household_id),
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
            .query_all_raw(recovery_statement(
                tenant,
                &restorable_query(tenant.scope().household_id, ids),
            ))
            .await?;
        for row in restorable {
            let subject_id: i64 = row.try_get("", "subject_person_id")?;
            if let Some(subject) = subjects.iter_mut().find(|item| item.id.get() == subject_id) {
                subject.restorable = Some(RestorableLink {
                    declaration_id: row.try_get("", "declaration_id")?,
                    relationship_id: row.try_get("", "relationship_id")?,
                    guardian_id: RecoveryGuardianId::parse(row.try_get("", "guardian_person_id")?)?,
                    guardian_name: row.try_get("", "guardian_name")?,
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
    access::recheck(tenant).await?;
    if !access::can_manage_household(tenant) {
        return Err(OperationError::Forbidden);
    }
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
    let invocation = Query::select()
        .columns([
            ("result", "outcome"),
            ("result", "declaration_id"),
            ("result", "relationship_id"),
        ])
        .from_function(
            Func::cust("med_tracker.guardian_recovery_command").args(vec![
                Expr::val(command.subject.get()),
                Expr::val(command.guardian.get()),
                Expr::val(command.expected.token),
                Expr::val(command.expected.revision),
                Expr::val(command.expected.link_token),
                Expr::val(restore_declaration),
                Expr::val(restore_link),
                Expr::val(reason),
                Expr::val(tenant.scope().request_id.clone()),
            ]),
            "result",
        )
        .to_owned();
    let row = tenant
        .transaction()
        .query_one_raw(recovery_statement(tenant, &invocation))
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
