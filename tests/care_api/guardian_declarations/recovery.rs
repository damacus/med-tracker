// SeaQuery has no PostgreSQL trigger/function DDL API. The exclusive fault cases retain
// narrow trigger DDL and restore it before closing the App; row operations use builders.

use super::{Application, require, storage_boundary::foreign_token, withdrawal::declaration};
use med_tracker::models::_entities::{
    account_lockouts, accounts, carer_relationships, household_memberships, households, people,
};
use med_tracker::models::{
    access::{self, Actor, HouseholdScope, PersonAccess},
    care::administration::recovery::{
        self, RecoveryAction, RecoveryCommand, RecoveryGuardianId, RecoveryOutcome,
        RecoverySnapshot, RecoverySubjectId,
    },
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, Set,
    TransactionTrait,
    sea_query::{Alias, Asterisk, Expr, ExprTrait, Func, Query},
};
use serde_json::{Value, json};

fn db_error(error: impl std::fmt::Display) -> DbErr {
    DbErr::Custom(error.to_string())
}

fn scope(account: i64, household: i64) -> HouseholdScope {
    HouseholdScope {
        actor: Actor {
            account_id: account,
        },
        household_id: household,
        request_id: "synthetic-recovery-command".into(),
    }
}

async fn backend_pid<C: ConnectionTrait>(connection: &C) -> Result<i32, DbErr> {
    let query = Query::select()
        .expr_as(Func::cust("pg_backend_pid"), "pid")
        .to_owned();
    connection
        .query_one_raw(connection.get_database_backend().build(&query))
        .await?
        .ok_or_else(|| DbErr::Custom("Missing participating backend PID".into()))?
        .try_get("", "pid")
}

fn blocked_activity(blocker_pid: i32) -> sea_orm::sea_query::SelectStatement {
    Query::select()
        .column("pid")
        .from("pg_stat_activity")
        .and_where(Expr::col("datname").eq(Func::cust("current_database")))
        .and_where(Expr::col("state").eq("active"))
        .and_where(Expr::col("wait_event_type").eq("Lock"))
        .and_where(
            Expr::expr(Func::cust("array_position").args([
                Expr::expr(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))),
                Expr::val(blocker_pid),
            ]))
            .is_not_null(),
        )
        .to_owned()
}

async fn is_blocked_by(
    app: &Application,
    waiter_pid: i32,
    blocker_pid: i32,
) -> Result<bool, DbErr> {
    let query = Query::select()
        .expr_as(
            Expr::exists(
                blocked_activity(blocker_pid)
                    .and_where(Expr::col("pid").eq(waiter_pid))
                    .to_owned(),
            ),
            "waiting",
        )
        .to_owned();
    app.fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await?
        .ok_or_else(|| DbErr::Custom("Missing attributed lock wait state".into()))?
        .try_get("", "waiting")
}

async fn wait_for_blocker(
    app: &Application,
    waiter_pid: i32,
    blocker_pid: i32,
) -> Result<bool, DbErr> {
    for _ in 0..40 {
        if is_blocked_by(app, waiter_pid, blocker_pid).await? {
            return Ok(true);
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    Ok(false)
}

async fn effective_snapshot(
    tenant: &access::TenantTransaction,
    subject: i64,
) -> Result<RecoverySnapshot, DbErr> {
    let query = sea_orm::sea_query::Query::select()
        .columns([
            ("snapshot", "snapshot_token"),
            ("snapshot", "snapshot_revision"),
            ("snapshot", "link_token"),
            ("snapshot", "has_effective_guardian"),
        ])
        .from_function(
            sea_orm::sea_query::Func::cust("med_tracker.guardian_recovery_snapshot")
                .arg(sea_orm::sea_query::Expr::val(subject)),
            "snapshot",
        )
        .to_owned();
    let row = tenant
        .transaction()
        .query_one_raw(tenant.transaction().get_database_backend().build(&query))
        .await?
        .ok_or_else(|| DbErr::Custom("Missing guarded recovery snapshot".into()))?;
    require(
        row.try_get::<bool>("", "has_effective_guardian")?,
        "Canonical snapshot must report effective guardian",
    )?;
    Ok(RecoverySnapshot {
        token: row.try_get("", "snapshot_token")?,
        revision: row.try_get("", "snapshot_revision")?,
        link_token: row.try_get("", "link_token")?,
    })
}

#[tokio::test]
async fn guardian_recovery_account_then_household_closure_order_rejects_without_effects()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let account = app.fixture.account.id;
    let household = app.fixture.household.id;
    let backend = app.fixture.admin.get_database_backend();
    let result = async {
        let child = orphan(&app, "Closure race minor").await?;
        let candidate = actor(&app, "Closing candidate", "member").await?;

        let tenant = access::begin(&app.fixture.runtime, &scope(account, household), None)
            .await
            .map_err(db_error)?;
        let snapshot = recovery::options(&tenant)
            .await
            .map_err(db_error)?
            .subjects
            .into_iter()
            .find(|item| item.id.get() == child.person.id)
            .ok_or_else(|| DbErr::Custom("Missing closure-race minor".into()))?
            .snapshot;
        tenant.rollback().await.map_err(db_error)?;
        let blocker = app.fixture.admin.begin().await?;
        let blocker_pid = backend_pid(&blocker).await?;
        blocker
            .execute_raw(
                backend.build(
                    &Query::select()
                        .expr(Expr::col("id"))
                        .from("accounts")
                        .and_where(Expr::col("id").eq(Expr::val(candidate.account.id)))
                        .lock_exclusive()
                        .to_owned(),
                ),
            )
            .await?;
        let tenant = access::begin(&app.fixture.runtime, &scope(account, household), None)
            .await
            .map_err(db_error)?;
        let recovery_pid = backend_pid(tenant.transaction()).await?;
        let attempt = tokio::spawn(async move {
            let outcome = recovery::execute(
                &tenant,
                RecoveryCommand {
                    subject: RecoverySubjectId::parse(child.person.id).unwrap(),
                    guardian: RecoveryGuardianId::parse(candidate.person.id).unwrap(),
                    expected: snapshot,
                    action: RecoveryAction::Appoint,
                    reason: "Candidate account closure must invalidate recovery.".into(),
                },
            )
            .await;
            if outcome.is_ok() {
                tenant.commit().await.unwrap()
            } else {
                tenant.rollback().await.unwrap()
            }
            outcome
                .map(|value| value.outcome)
                .map_err(|error| format!("{error:?}"))
        });
        let waiting = wait_for_blocker(&app, recovery_pid, blocker_pid).await?;
        if !waiting {
            blocker.rollback().await?;
            let _ = attempt.await;
            return Err(DbErr::Custom(
                "Recovery did not wait on candidate account".into(),
            ));
        }
        let household_lock = tokio::time::timeout(
            std::time::Duration::from_secs(8),
            blocker.execute_raw(
                backend.build(
                    &Query::select()
                        .expr(Expr::col("id"))
                        .from("households")
                        .and_where(Expr::col("id").eq(Expr::val(household)))
                        .lock_exclusive()
                        .to_owned(),
                ),
            ),
        )
        .await
        .map_err(db_error)?;
        if let Err(error) = household_lock {
            let _ = blocker.rollback().await;
            let _ = attempt.await;
            return Err(DbErr::Custom(format!(
                "Closure household lock failed: {error}"
            )));
        }
        (async {
            blocker
                .execute_raw(
                    backend.build(
                        &Query::update()
                            .table("household_memberships")
                            .value("status", Expr::val("revoked"))
                            .value("revoked_at", Expr::expr(Func::cust("now")))
                            .value(
                                "permissions_version",
                                Expr::col("permissions_version").add(Expr::val(1)),
                            )
                            .and_where(Expr::col("id").eq(Expr::val(candidate.membership.id)))
                            .to_owned(),
                    ),
                )
                .await?;
            blocker
                .execute_raw(
                    backend.build(
                        &Query::update()
                            .table("accounts")
                            .value("active", Expr::val(false))
                            .and_where(Expr::col("id").eq(Expr::val(candidate.account.id)))
                            .to_owned(),
                    ),
                )
                .await?;
            let result = blocker
                .execute_raw(
                    backend.build(
                        &Query::update()
                            .table("accounts")
                            .value("status", Expr::val(3))
                            .and_where(Expr::col("id").eq(Expr::val(candidate.account.id)))
                            .to_owned(),
                    ),
                )
                .await?;
            Ok::<_, DbErr>(result)
        })
        .await?;
        blocker.commit().await?;
        let outcome = tokio::time::timeout(std::time::Duration::from_secs(8), attempt)
            .await
            .map_err(db_error)?
            .map_err(db_error)?;
        require(
            outcome
                .as_ref()
                .is_err_and(|error| error.contains("is no longer eligible")),
            &format!("Recovery must reject closed candidate after account wait: {outcome:?}"),
        )?;
        let state = app
            .fixture
            .admin
            .query_one_raw(
                backend.build(
                    &Query::select()
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("guardian_declarations")
                                    .and_where(
                                        Expr::col("subject_person_id")
                                            .eq(Expr::val(child.person.id)),
                                    )
                                    .to_owned(),
                            ),
                            "declarations",
                        )
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("carer_relationships")
                                    .and_where(
                                        Expr::col("patient_id").eq(Expr::val(child.person.id)),
                                    )
                                    .to_owned(),
                            ),
                            "links",
                        )
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("security_audit_events")
                                    .and_where(Expr::col("event_type").like("guardian_recovery.%"))
                                    .and_where(Expr::col("household_id").eq(household))
                                    .to_owned(),
                            ),
                            "audits",
                        )
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("versions")
                                    .and_where(
                                        Expr::col("item_type")
                                            .eq(Expr::val("GuardianDeclaration"))
                                            .and(Expr::col("event").eq(Expr::val("appointed"))),
                                    )
                                    .and_where(Expr::col("household_id").eq(household))
                                    .to_owned(),
                            ),
                            "versions",
                        )
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(
                                        Expr::col("permissions_version")
                                            .cast_as(Alias::new("bigint")),
                                    )
                                    .from("household_memberships")
                                    .and_where(
                                        Expr::col("id").eq(Expr::val(candidate.membership.id)),
                                    )
                                    .to_owned(),
                            ),
                            "permissions_version",
                        )
                        .to_owned(),
                ),
            )
            .await?
            .ok_or_else(|| DbErr::Custom("Missing closure-race state".into()))?;
        require(
            state.try_get::<i64>("", "declarations")? == 0
                && state.try_get::<i64>("", "links")? == 0
                && state.try_get::<i64>("", "audits")? == 0
                && state.try_get::<i64>("", "versions")? == 0
                && state.try_get::<i64>("", "permissions_version")? == 2,
            "Rejected recovery must leave no authority, audit, version or extra permission effects",
        )?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_options_report_bounded_truncation() -> Result<(), DbErr> {
    let app = Application::new().await;
    let account = app.fixture.account.id;
    let household = app.fixture.household.id;
    let result = async {
        for index in 1..=101 {
            orphan(&app, &format!("Synthetic recovery list minor {index}")).await?;
        }

        let tenant = access::begin(&app.fixture.runtime, &scope(account, household), None)
            .await
            .map_err(db_error)?;
        let options = recovery::options(&tenant).await.map_err(db_error)?;
        require(
            options.subjects.len() == 100 && options.subjects_has_more,
            "Bounded subject list must truthfully report unseen eligible local minors",
        )?;
        require(
            !options.guardians_has_more,
            "Short guardian list must report completion",
        )?;
        tenant.rollback().await.map_err(db_error)?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_options_reach_orphan_after_101_guarded_minors() -> Result<(), DbErr> {
    let app = Application::new().await;
    let account = app.fixture.account.id;
    let household = app.fixture.household.id;
    let guardian = app.fixture.adult.id;
    let result = async {
        let fixture = app.fixture.admin.begin().await?;
        let mut guarded_ids = Vec::new();
        for index in 1..=101 {
            let child =
                orphan_on(&fixture, household, &format!("A guarded minor {index:03}")).await?;
            guarded_ids.push(child.person.id);
        }
        let orphan = orphan_on(&fixture, household, "Zoe late orphan").await?;

        fixture.commit().await?;
        let fixture = access::begin(&app.fixture.runtime, &scope(account, household), None)
            .await
            .map_err(db_error)?;
        for (index, subject) in guarded_ids.iter().enumerate() {
            initial_link(
                fixture.transaction(),
                household,
                guardian,
                account,
                *subject,
                &format!("A guarded minor {}", index + 1),
            )
            .await?;
        }
        fixture.commit().await.map_err(db_error)?;
        let tenant = access::begin(&app.fixture.runtime, &scope(account, household), None)
            .await
            .map_err(db_error)?;
        let mut guarded = 0;
        for subject in guarded_ids {
            effective_snapshot(&tenant, subject).await?;
            guarded += 1;
        }
        require(
            guarded == 101,
            "Fixture must contain 101 canonically effective guardian links",
        )?;
        let options = recovery::options(&tenant).await.map_err(db_error)?;
        require(
            options
                .subjects
                .iter()
                .any(|subject| subject.id.get() == orphan.person.id)
                && options
                    .subjects
                    .iter()
                    .all(|subject| !subject.has_effective_guardian)
                && !options.subjects_has_more,
            "Later orphan must be reachable and warning must count actual orphans",
        )?;
        tenant.rollback().await.map_err(db_error)?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_administrator_appoints_for_minor_without_history() -> Result<(), DbErr> {
    let app = Application::new().await;
    let household = app.fixture.household.id;
    let guardian = app.fixture.adult.id;
    let backend = app.fixture.admin.get_database_backend();
    let result = async {
        let child = orphan(&app, "No-history minor").await?;
        let manager = actor(&app, "No-history administrator", "administrator").await?;

        let manager_scope = HouseholdScope {
            actor: Actor {
                account_id: manager.account.id,
            },
            household_id: household,
            request_id: "synthetic-no-history-administrator".into(),
        };
        let tenant = access::begin(&app.fixture.runtime, &manager_scope, None)
            .await
            .map_err(db_error)?;
        let selected = recovery::options(&tenant)
            .await
            .map_err(db_error)?
            .subjects
            .into_iter()
            .find(|item| item.id.get() == child.person.id)
            .ok_or_else(|| DbErr::Custom("No-history minor missing from options".into()))?;
        require(
            !selected.has_effective_guardian
                && selected.restorable.is_none()
                && selected.snapshot.token.is_nil()
                && selected.snapshot.revision == 0,
            "No-history minor must have nil history confirmation and no authority",
        )?;
        let appointed = recovery::execute(
            &tenant,
            RecoveryCommand {
                subject: RecoverySubjectId::parse(child.person.id).map_err(db_error)?,
                guardian: RecoveryGuardianId::parse(guardian).map_err(db_error)?,
                expected: selected.snapshot,
                action: RecoveryAction::Appoint,
                reason: "Administrator confirmed adult guardian for this minor.".into(),
            },
        )
        .await
        .map_err(db_error)?;
        require(
            appointed.outcome == RecoveryOutcome::Appointed,
            "Administrator must be able to appoint for a no-history minor",
        )?;
        tenant.commit().await.map_err(db_error)?;
        let state = app
            .fixture
            .admin
            .query_one_raw(
                backend.build(
                    &Query::select()
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("guardian_declarations")
                                    .and_where(
                                        Expr::col("id")
                                            .eq(Expr::val(appointed.declaration_id))
                                            .and(
                                                Expr::col("subject_person_id")
                                                    .eq(Expr::val(child.person.id)),
                                            )
                                            .and(
                                                Expr::col("guardian_person_id")
                                                    .eq(Expr::val(guardian)),
                                            )
                                            .and(
                                                Expr::col("declared_by_account_id")
                                                    .eq(Expr::val(manager.account.id)),
                                            )
                                            .and(
                                                Expr::col("source")
                                                    .eq(Expr::val("manager_appointment")),
                                            ),
                                    )
                                    .to_owned(),
                            ),
                            "declarations",
                        )
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("security_audit_events")
                                    .and_where(
                                        Expr::col("event_type")
                                            .eq(Expr::val("guardian_recovery.appointed"))
                                            .and(
                                                Expr::col("actor_account_id")
                                                    .eq(Expr::val(manager.account.id)),
                                            )
                                            .and(
                                                Expr::expr(
                                                    Func::cust("jsonb_extract_path_text").args([
                                                        Expr::col("metadata"),
                                                        Expr::val("subject_person_id"),
                                                    ]),
                                                )
                                                .eq(Expr::val(child.person.id.to_string())),
                                            ),
                                    )
                                    .and_where(Expr::col("household_id").eq(household))
                                    .to_owned(),
                            ),
                            "audits",
                        )
                        .to_owned(),
                ),
            )
            .await?
            .ok_or_else(|| DbErr::Custom("Missing administrator recovery state".into()))?;
        require(
            state.try_get::<i64>("", "declarations")? == 1
                && state.try_get::<i64>("", "audits")? == 1,
            "No-history appointment must record the actual administrator and audit",
        )?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_latest_history_uses_declaration_time_not_high_old_id()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let account = app.fixture.account.id;
    let household = app.fixture.household.id;
    let guardian = app.fixture.adult.id;
    let backend = app.fixture.admin.get_database_backend();
    let sequence_before = next_declaration_id(&app).await?;
    let high = sequence_before.checked_add(1_000_000).unwrap();
    let result = async {
        set_declaration_id(&app, high).await?;
        let token = app.token().await;
        let (subject, old_id) = declaration(&app, &token).await?;
        require(
            old_id == high,
            "Fixture must give old initial declaration a high ID",
        )?;
        let withdrawn = app
            .client
            .patch(format!(
                "{}/api/v1/auth/guardian-declarations/{old_id}",
                app.origin
            ))
            .bearer_auth(&token)
            .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
            .send()
            .await
            .map_err(db_error)?;
        require(
            withdrawn.status().as_u16() == 200,
            "High-ID initial declaration withdrawal must succeed",
        )?;
        set_declaration_id(&app, sequence_before + 1).await?;
        let tenant = access::begin(&app.fixture.runtime, &scope(account, household), None)
            .await
            .map_err(db_error)?;
        let selected = recovery::options(&tenant)
            .await
            .map_err(db_error)?
            .subjects
            .into_iter()
            .find(|item| item.id.get() == subject)
            .ok_or_else(|| DbErr::Custom("Missing high-ID history subject".into()))?;
        let appointed = recovery::execute(
            &tenant,
            RecoveryCommand {
                subject: RecoverySubjectId::parse(subject).map_err(db_error)?,
                guardian: RecoveryGuardianId::parse(guardian).map_err(db_error)?,
                expected: selected.snapshot,
                action: RecoveryAction::Appoint,
                reason: "Appointed again after retained high-ID withdrawal.".into(),
            },
        )
        .await
        .map_err(db_error)?;
        require(
            appointed.declaration_id < old_id,
            "New appointment must have a lower generated ID",
        )?;
        tenant.commit().await.map_err(db_error)?;
        let tenant = access::begin(&app.fixture.runtime, &scope(account, household), None)
            .await
            .map_err(db_error)?;
        let latest = effective_snapshot(&tenant, subject).await?;
        require(
            !recovery::options(&tenant)
                .await
                .map_err(db_error)?
                .subjects
                .iter()
                .any(|item| item.id.get() == subject),
            "Guarded subject must leave recovery options after appointment",
        )?;
        tenant.rollback().await.map_err(db_error)?;
        let row = app
            .fixture
            .admin
            .query_one_raw(
                backend.build(
                    &Query::select()
                        .expr(Expr::col("confirmation_token"))
                        .expr(Expr::col("revision"))
                        .from("guardian_declarations")
                        .and_where(Expr::col("id").eq(Expr::val(appointed.declaration_id)))
                        .to_owned(),
                ),
            )
            .await?
            .ok_or_else(|| DbErr::Custom("Missing lower-ID new appointment".into()))?;
        require(
            latest.token == row.try_get::<uuid::Uuid>("", "confirmation_token")?
                && latest.revision == row.try_get::<i64>("", "revision")?
                && latest.token != selected.snapshot.token,
            "Latest confirmation must follow declaration time, including terminal high-ID history",
        )?;
        Ok(())
    }
    .await;
    let cleanup = restore_declaration_sequence(&app, high).await;
    app.close().await;
    cleanup?;
    result
}

#[tokio::test]
async fn guardian_recovery_after_personal_withdrawal_creates_fresh_appointment() -> Result<(), DbErr>
{
    let app = Application::new().await;
    let account = app.fixture.account.id;
    let household = app.fixture.household.id;
    let guardian = app.fixture.adult.id;
    let backend = app.fixture.admin.get_database_backend();
    let result = async {
        let token = app.token().await;
        let (subject, old_id) = declaration(&app, &token).await?;
        let response = app
            .client
            .patch(format!(
                "{}/api/v1/auth/guardian-declarations/{old_id}",
                app.origin
            ))
            .bearer_auth(&token)
            .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
            .send()
            .await
            .map_err(db_error)?;
        require(
            response.status().as_u16() == 200,
            "Withdrawal must succeed before recovery",
        )?;
        let tenant = access::begin(&app.fixture.runtime, &scope(account, household), None)
            .await
            .map_err(db_error)?;
        let options = recovery::options(&tenant)
            .await
            .map_err(|error| DbErr::Custom(format!("options: {error:?}")))?;
        let selected = options
            .subjects
            .iter()
            .find(|item| item.id.get() == subject)
            .ok_or_else(|| DbErr::Custom("Withdrawn minor missing from recovery options".into()))?;
        require(
            !selected.has_effective_guardian && selected.restorable.is_none(),
            "Withdrawn declaration must not be restored",
        )?;
        require(
            options
                .guardians
                .iter()
                .any(|item| item.id.get() == guardian),
            "Eligible same-person appointee must remain available",
        )?;
        let original_snapshot = selected.snapshot;
        let command = RecoveryCommand {
            subject: RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian: RecoveryGuardianId::parse(guardian).map_err(db_error)?,
            expected: original_snapshot,
            action: RecoveryAction::Appoint,
            reason: "Verified the guardian's current authority.".into(),
        };
        let outcome = recovery::execute(&tenant, command)
            .await
            .map_err(|error| DbErr::Custom(format!("execute: {error:?}")))?;
        require(
            outcome.outcome == RecoveryOutcome::Appointed && outcome.declaration_id != old_id,
            "Recovery must create a fresh declaration, never revive withdrawn authority",
        )?;
        tenant.commit().await.map_err(db_error)?;
        let state = app
            .fixture
            .admin
            .query_one_raw(
                backend.build(
                    &Query::select()
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("guardian_declarations")
                                    .and_where(
                                        Expr::col("subject_person_id")
                                            .eq(Expr::val(subject))
                                            .and(Expr::col("withdrawn_at").is_not_null())
                                            .and(
                                                Expr::col("source")
                                                    .eq(Expr::val("initial_attestation")),
                                            ),
                                    )
                                    .to_owned(),
                            ),
                            "withdrawn",
                        )
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("guardian_declarations")
                                    .and_where(
                                        Expr::col("subject_person_id")
                                            .eq(Expr::val(subject))
                                            .and(
                                                Expr::col("source")
                                                    .eq(Expr::val("manager_appointment")),
                                            )
                                            .and(Expr::col("withdrawn_at").is_null())
                                            .and(Expr::col("superseded_at").is_null()),
                                    )
                                    .to_owned(),
                            ),
                            "appointed",
                        )
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("security_audit_events")
                                    .and_where(
                                        Expr::col("event_type")
                                            .eq(Expr::val("guardian_recovery.appointed"))
                                            .and(
                                                Expr::expr(
                                                    Func::cust("jsonb_extract_path_text").args([
                                                        Expr::col("metadata"),
                                                        Expr::val("subject_person_id"),
                                                    ]),
                                                )
                                                .eq(Expr::val(subject.to_string())),
                                            ),
                                    )
                                    .and_where(Expr::col("household_id").eq(household))
                                    .to_owned(),
                            ),
                            "audits",
                        )
                        .to_owned(),
                ),
            )
            .await?
            .ok_or_else(|| DbErr::Custom("Missing recovery state".into()))?;
        require(
            state.try_get::<i64>("", "withdrawn")? == 1
                && state.try_get::<i64>("", "appointed")? == 1
                && state.try_get::<i64>("", "audits")? == 1,
            "Recovery must retain withdrawn history and audit the fresh appointment",
        )?;
        let own = app
            .client
            .get(format!("{}/api/v1/auth/guardian-declarations", app.origin))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(db_error)?;
        let body: Value = own.json().await.map_err(db_error)?;
        require(
            body["data"].as_array().is_some_and(|rows| {
                rows.iter().any(|row| {
                    row["source"] == "manager_appointment"
                        && serde_json::Value::is_null(&row["superseded_at"])
                })
            }),
            "Own API must distinguish the manager appointment",
        )?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

async fn concurrent_appointment(
    app: &Application,
    household_id: i64,
    subject: i64,
    snapshot: RecoverySnapshot,
) -> Result<Result<RecoveryOutcome, String>, DbErr> {
    let account = app.fixture.account.id;
    let guardian = app.fixture.adult.id;
    let scope = HouseholdScope {
        actor: Actor {
            account_id: account,
        },
        household_id,
        request_id: format!("synthetic-concurrent-recovery-{household_id}"),
    };
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .map_err(db_error)?;
    let result = recovery::execute(
        &tenant,
        RecoveryCommand {
            subject: RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian: RecoveryGuardianId::parse(guardian).map_err(db_error)?,
            expected: snapshot,
            action: RecoveryAction::Appoint,
            reason: "Confirmed the guardian for this household.".into(),
        },
    )
    .await;
    match result {
        Ok(value) => {
            tenant.commit().await.map_err(db_error)?;
            Ok(Ok(value.outcome))
        }
        Err(error) => {
            tenant.rollback().await.map_err(db_error)?;
            Ok(Err(format!("{error:?}")))
        }
    }
}

#[tokio::test]
async fn guardian_recovery_competing_households_serialise_by_subject() -> Result<(), DbErr> {
    let app = Application::new().await;
    let account = app.fixture.account.id;
    let household = app.fixture.household.id;
    let backend = app.fixture.admin.get_database_backend();
    let result = async {
        let token = app.token().await;
        let (subject, old_id) = declaration(&app, &token).await?;
        let withdrawn = app
            .client
            .patch(format!(
                "{}/api/v1/auth/guardian-declarations/{old_id}",
                app.origin
            ))
            .bearer_auth(&token)
            .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
            .send()
            .await
            .map_err(db_error)?;
        require(
            withdrawn.status().as_u16() == 200,
            "Concurrent test requires terminal old authority",
        )?;
        let foreign_household =
            second_household(&app, "Second recovery household", "owner").await?;

        app.fixture
            .admin
            .execute_raw(
                backend.build(
                    &Query::insert()
                        .into_table("household_memberships")
                        .columns([
                            "household_id",
                            "person_id",
                            "role",
                            "status",
                            "joined_at",
                            "created_at",
                            "updated_at",
                        ])
                        .values_panic([
                            Expr::val(foreign_household.id),
                            Expr::val(subject),
                            Expr::val("member"),
                            Expr::val("active"),
                            Expr::expr(Func::cust("now")),
                            Expr::expr(Func::cust("now")),
                            Expr::expr(Func::cust("now")),
                        ])
                        .to_owned(),
                ),
            )
            .await?;
        let tenant = access::begin(&app.fixture.runtime, &scope(account, household), None)
            .await
            .map_err(db_error)?;
        let snapshot = recovery::options(&tenant)
            .await
            .map_err(db_error)?
            .subjects
            .into_iter()
            .find(|item| item.id.get() == subject)
            .ok_or_else(|| DbErr::Custom("Missing concurrent subject".into()))?
            .snapshot;
        tenant.rollback().await.map_err(db_error)?;
        let (first, second) = tokio::join!(
            concurrent_appointment(&app, household, subject, snapshot),
            concurrent_appointment(&app, foreign_household.id, subject, snapshot),
        );
        let first = first?;
        let second = second?;
        require(
            (first == Ok(RecoveryOutcome::Appointed)
                && second
                    .as_ref()
                    .is_err_and(|text| text.contains("stale_guardian_recovery")))
                || (second == Ok(RecoveryOutcome::Appointed)
                    && first
                        .as_ref()
                        .is_err_and(|text| text.contains("stale_guardian_recovery"))),
            &format!(
                "Exactly one competing household must appoint; outcomes {first:?}, {second:?}"
            ),
        )?;
        let state = app
            .fixture
            .admin
            .query_one_raw(
                backend.build(
                    &Query::select()
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("guardian_declarations")
                                    .and_where(
                                        Expr::col("subject_person_id").eq(Expr::val(subject)).and(
                                            Expr::col("source")
                                                .eq(Expr::val("manager_appointment")),
                                        ),
                                    )
                                    .to_owned(),
                            ),
                            "appointments",
                        )
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("guardian_declarations")
                                    .and_where(
                                        Expr::col("subject_person_id")
                                            .eq(Expr::val(subject))
                                            .and(Expr::col("withdrawn_at").is_null())
                                            .and(Expr::col("superseded_at").is_null()),
                                    )
                                    .to_owned(),
                            ),
                            "open",
                        )
                        .to_owned(),
                ),
            )
            .await?
            .ok_or_else(|| DbErr::Custom("Missing concurrent state".into()))?;
        require(
            state.try_get::<i64>("", "appointments")? == 1
                && state.try_get::<i64>("", "open")? == 1,
            "Subject serialization must leave one new open authority",
        )?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_restore_and_personal_withdrawal_serialize_without_deadlock()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let household = app.fixture.household.id;
    let backend = app.fixture.admin.get_database_backend();
    let result=async {
        let token=app.token().await;
        let (subject,declaration_id)=declaration(&app,&token).await?;
        let manager=actor(&app,"Restore race administrator","administrator").await?;

        app.fixture.admin.execute_raw(backend.build(&Query::update().table("carer_relationships").value("active",Expr::val(false)).and_where(Expr::col("guardian_declaration_id").eq(Expr::val(declaration_id))).to_owned())).await?;
        let manager_scope=HouseholdScope {actor:Actor{account_id:manager.account.id},household_id:household,
            request_id:"synthetic-restore-withdrawal-race".into()};
        let tenant=access::begin(&app.fixture.runtime,&manager_scope,None).await.map_err(db_error)?;
        let selected=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Missing restore-race minor".into()))?;
        let link=selected.restorable.ok_or_else(||DbErr::Custom("Missing restore-race link".into()))?;
        tenant.rollback().await.map_err(db_error)?;
        let blocker=app.fixture.admin.begin().await?;
        let blocker_pid = backend_pid(&blocker).await?;
        blocker.query_one_raw(backend.build(&Query::select().expr(Expr::col("id")).from("guardian_declarations").and_where(Expr::col("id").eq(Expr::val(declaration_id))).lock_exclusive().to_owned())).await?
            .ok_or_else(||DbErr::Custom("Missing declaration lock blocker".into()))?;
        let tenant = access::begin(&app.fixture.runtime, &manager_scope, None).await.map_err(db_error)?;
        let recovery_pid = backend_pid(tenant.transaction()).await?;
        let restore=tokio::spawn(async move {

            let result=recovery::execute(&tenant,RecoveryCommand {
                subject:RecoverySubjectId::parse(subject).unwrap(),
                guardian:link.guardian_id,expected:selected.snapshot,
                action:RecoveryAction::Restore {declaration_id:link.declaration_id,
                    relationship_id:link.relationship_id},
                reason:"Confirm existing guardian link before withdrawal.".into(),
            }).await;
            if result.is_ok() {tenant.commit().await.unwrap()} else {tenant.rollback().await.unwrap()}
            result.map(|value|value.outcome).map_err(|error|format!("{error:?}"))
        });
        let recovery_waiting=wait_for_blocker(&app,recovery_pid,blocker_pid).await?;
        if !recovery_waiting {
            blocker.commit().await?;
            let _=restore.await;
            return Err(DbErr::Custom("Restore did not wait on declaration blocker".into()));
        }
        let client=app.client.clone();
        let origin=app.origin.clone();
        let withdrawal=tokio::spawn(async move {
            let response=client.patch(format!("{origin}/api/v1/auth/guardian-declarations/{declaration_id}"))
                .bearer_auth(&token)
                .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
                .send().await.map_err(|error|error.to_string())?;
            Ok::<u16,String>(response.status().as_u16())
        });
        let mut two_waiters=false;
        let mut withdrawal_pids=Vec::new();
        for _ in 0..40 {
            let query=blocked_activity(recovery_pid)
                .and_where(Expr::col("pid").ne(recovery_pid))
                .and_where(Expr::col("query").like("%\"accounts\"%FOR NO KEY UPDATE%"))
                .to_owned();
            withdrawal_pids=app.fixture.admin.query_all_raw(backend.build(&query)).await?
                .into_iter().map(|row|row.try_get::<i32>("","pid")).collect::<Result<Vec<_>,_>>()?;
            two_waiters=withdrawal_pids.len()==1 && withdrawal_pids[0]!=blocker_pid
                && is_blocked_by(&app,recovery_pid,blocker_pid).await?;
            if two_waiters {break}
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        blocker.commit().await?;
        let restore_outcome=tokio::time::timeout(std::time::Duration::from_secs(8),restore)
            .await.map_err(db_error)?.map_err(db_error)?;
        let withdrawal_status=tokio::time::timeout(std::time::Duration::from_secs(8),withdrawal)
            .await.map_err(db_error)?.map_err(db_error)?;
        require(two_waiters,&format!("Both restore and personal withdrawal must enter lock waits: declaration blocker {blocker_pid}, restore {recovery_pid}, account withdrawal waiters {withdrawal_pids:?}"))?;
        require(matches!(restore_outcome,Ok(RecoveryOutcome::Restored))
            && matches!(withdrawal_status,Ok(200)),
            &format!("Restore and withdrawal must serialize without lock error: {restore_outcome:?}, {withdrawal_status:?}"))?;
        let state=app.fixture.admin.query_one_raw(backend.build(&Query::select().expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("guardian_declarations").and_where(Expr::col("id").eq(Expr::val(declaration_id)).and(Expr::col("withdrawn_at").is_not_null()).and(Expr::col("superseded_at").is_null())).to_owned()),"terminal").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("security_audit_events").and_where(Expr::col("event_type").eq(Expr::val("guardian_recovery.restored")).and(Expr::expr(Func::cust("jsonb_extract_path_text").args([Expr::col("metadata"),Expr::val("subject_person_id")])).eq(Expr::val(subject.to_string())))).and_where(Expr::col("household_id").eq(household)).to_owned()),"restores").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("versions").and_where(Expr::col("item_type").eq(Expr::val("GuardianDeclaration")).and(Expr::col("item_id").eq(Expr::val(declaration_id))).and(Expr::col("event").eq(Expr::val("withdraw")))).to_owned()),"withdrawals").to_owned())).await?
            .ok_or_else(||DbErr::Custom("Missing serialized restore/withdrawal state".into()))?;
        require(state.try_get::<i64>("","terminal")?==1 && state.try_get::<i64>("","restores")?==1
            && state.try_get::<i64>("","withdrawals")?==1,
            "Final authority must be terminal with one restore and one personal withdrawal audit")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_candidate_lockout_after_account_wait_rejects_without_effects()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let account = app.fixture.account.id;
    let household = app.fixture.household.id;
    let backend = app.fixture.admin.get_database_backend();
    let result=async {
        let token=app.token().await;
        let (subject,_)=declaration(&app,&token).await?;
        let candidate=actor(&app,"Lockout race candidate","member").await?;
        let manager=actor(&app,"Lockout race manager","administrator").await?;
        accounts::Entity::update_many().col_expr(accounts::Column::Status,Expr::val(1)).filter(accounts::Column::Id.eq(account)).exec(&app.fixture.admin).await?;

        let manager_scope=HouseholdScope {actor:Actor{account_id:manager.account.id},household_id:household,
            request_id:"synthetic-candidate-lockout-race".into()};
        let tenant=access::begin(&app.fixture.runtime,&manager_scope,None).await.map_err(db_error)?;
        let selected=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Missing lockout-race minor".into()))?;
        require(!selected.has_effective_guardian,"Disabled old guardian must make the minor orphaned")?;
        tenant.rollback().await.map_err(db_error)?;
        let blocker=app.fixture.admin.begin().await?;
        let blocker_pid = backend_pid(&blocker).await?;
        blocker.execute_raw(backend.build(&Query::update().table("accounts").value("status",Expr::val(2)).and_where(Expr::col("id").eq(Expr::val(candidate.account.id))).to_owned())).await?;
        let tenant = access::begin(&app.fixture.runtime, &manager_scope, None).await.map_err(db_error)?;
        let recovery_pid = backend_pid(tenant.transaction()).await?;
        let snapshot=selected.snapshot;
        let attempt=tokio::spawn(async move {

            let result=recovery::execute(&tenant,RecoveryCommand {
                subject:RecoverySubjectId::parse(subject).unwrap(),
                guardian:RecoveryGuardianId::parse(candidate.person.id).unwrap(),
                expected:snapshot,action:RecoveryAction::Appoint,
                reason:"Lockout race must reject appointment.".into(),
            }).await;
            if result.is_ok() {tenant.commit().await.unwrap()} else {tenant.rollback().await.unwrap()}
            result.map(|value|value.outcome).map_err(|error|format!("{error:?}"))
        });
        let waiting=wait_for_blocker(&app,recovery_pid,blocker_pid).await?;
        if !waiting {
            blocker.commit().await?;
            let _=attempt.await;
            return Err(DbErr::Custom("Recovery did not wait on candidate account".into()));
        }
        blocker.execute_raw(backend.build(&Query::insert().into_table("account_lockouts").columns(["account_id","key","deadline"]).values_panic([Expr::val(candidate.account.id),Expr::val("synthetic-recovery-race"),Expr::expr(Func::cust("now")).add(Expr::expr(Func::cust("make_interval").args([Expr::val(0),Expr::val(0),Expr::val(0),Expr::val(0),Expr::val(1)])))]).to_owned())).await?;
        blocker.commit().await?;
        let outcome=attempt.await.map_err(db_error)?;
        require(outcome.as_ref().is_err_and(|error|error.contains("is no longer eligible")),
            &format!("New candidate lockout must reject after wait: {outcome:?}"))?;
        let state=app.fixture.admin.query_one_raw(backend.build(&Query::select().expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("guardian_declarations").and_where(Expr::col("subject_person_id").eq(Expr::val(subject)).and(Expr::col("source").eq(Expr::val("manager_appointment")))).to_owned()),"appointments").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("carer_relationships").and_where(Expr::col("patient_id").eq(Expr::val(subject)).and(Expr::col("carer_id").eq(Expr::val(candidate.person.id)))).to_owned()),"links").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("security_audit_events").and_where(Expr::col("event_type").like("guardian_recovery.%").and(Expr::expr(Func::cust("jsonb_extract_path_text").args([Expr::col("metadata"),Expr::val("subject_person_id")])).eq(Expr::val(subject.to_string())))).and_where(Expr::col("household_id").eq(household)).to_owned()),"audits").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("versions").and_where(Expr::col("item_type").eq(Expr::val("GuardianDeclaration")).and(Expr::col("event").eq(Expr::val("appointed"))).and(Expr::col("item_id").in_subquery(Query::select().expr(Expr::col("id")).from("guardian_declarations").and_where(Expr::col("subject_person_id").eq(Expr::val(subject))).to_owned()))).to_owned()),"versions").expr_as(Expr::from(Query::select().expr(Expr::col("permissions_version").cast_as(Alias::new("bigint"))).from("household_memberships").and_where(Expr::col("id").eq(Expr::val(candidate.membership.id))).to_owned()),"permissions_version").to_owned())).await?
            .ok_or_else(||DbErr::Custom("Missing rejected lockout-race state".into()))?;
        require(state.try_get::<i64>("","appointments")?==0 && state.try_get::<i64>("","links")?==0
            && state.try_get::<i64>("","audits")?==0 && state.try_get::<i64>("","versions")?==0
            && state.try_get::<i64>("","permissions_version")?==1,
            "Rejected lockout-race command must have zero authority, audit and permission side effects")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_waiting_on_candidate_rechecks_restored_old_authority()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let account = app.fixture.account.id;
    let household = app.fixture.household.id;
    let backend = app.fixture.admin.get_database_backend();
    let result = async {
        let token = app.token().await;
        let (subject, _) = declaration(&app, &token).await?;
        let candidate = actor(&app, "Candidate lock adult", "member").await?;
        let manager = actor(&app, "Recovery administrator", "administrator").await?;
        accounts::Entity::update_many()
            .col_expr(accounts::Column::Status, Expr::val(1))
            .filter(accounts::Column::Id.eq(account))
            .exec(&app.fixture.admin)
            .await?;

        let manager_scope = HouseholdScope {
            actor: Actor {
                account_id: manager.account.id,
            },
            household_id: household,
            request_id: "synthetic-candidate-lock-race".into(),
        };
        let tenant = access::begin(&app.fixture.runtime, &manager_scope, None)
            .await
            .map_err(db_error)?;
        let selected = recovery::options(&tenant)
            .await
            .map_err(db_error)?
            .subjects
            .into_iter()
            .find(|item| item.id.get() == subject)
            .ok_or_else(|| DbErr::Custom("Missing lock-race subject".into()))?;
        require(
            !selected.has_effective_guardian,
            "Disabled old account must make the minor orphaned",
        )?;
        tenant.rollback().await.map_err(db_error)?;
        let blocker = app.fixture.admin.begin().await?;
        let blocker_pid = backend_pid(&blocker).await?;
        blocker
            .execute_raw(
                backend.build(
                    &Query::update()
                        .table("accounts")
                        .value("status", Expr::val(2))
                        .and_where(Expr::col("id").eq(Expr::val(candidate.account.id)))
                        .to_owned(),
                ),
            )
            .await?;
        let tenant = access::begin(&app.fixture.runtime, &manager_scope, None)
            .await
            .map_err(db_error)?;
        let recovery_pid = backend_pid(tenant.transaction()).await?;
        let snapshot = selected.snapshot;
        let attempt = tokio::spawn(async move {
            let result = recovery::execute(
                &tenant,
                RecoveryCommand {
                    subject: RecoverySubjectId::parse(subject).unwrap(),
                    guardian: RecoveryGuardianId::parse(candidate.person.id).unwrap(),
                    expected: snapshot,
                    action: RecoveryAction::Appoint,
                    reason: "Candidate is eligible after account lock.".into(),
                },
            )
            .await;
            if result.is_ok() {
                tenant.commit().await.unwrap()
            } else {
                tenant.rollback().await.unwrap()
            }
            result
                .map(|value| value.outcome)
                .map_err(|error| format!("{error:?}"))
        });
        let waiting = wait_for_blocker(&app, recovery_pid, blocker_pid).await?;
        if !waiting {
            blocker.commit().await?;
            let _ = attempt.await;
            return Err(DbErr::Custom(
                "Recovery was not observed waiting on the candidate lock".into(),
            ));
        }
        blocker
            .execute_raw(
                backend.build(
                    &Query::update()
                        .table("accounts")
                        .value("status", Expr::val(2))
                        .and_where(Expr::col("id").eq(Expr::val(account)))
                        .to_owned(),
                ),
            )
            .await?;
        blocker.commit().await?;
        let outcome = attempt.await.map_err(db_error)?;
        require(
            outcome
                .as_ref()
                .is_err_and(|error| error.contains("guardian_already_effective")),
            &format!("Restored old authority must block appointment after lock wait: {outcome:?}"),
        )?;
        let row = app
            .fixture
            .admin
            .query_one_raw(
                backend.build(
                    &Query::select()
                        .expr_as(Expr::col(Asterisk).count(), "appointments")
                        .from("guardian_declarations")
                        .and_where(
                            Expr::col("subject_person_id")
                                .eq(Expr::val(subject))
                                .and(Expr::col("source").eq(Expr::val("manager_appointment"))),
                        )
                        .to_owned(),
                ),
            )
            .await?
            .ok_or_else(|| DbErr::Custom("Missing race outcome".into()))?;
        require(
            row.try_get::<i64>("", "appointments")? == 0,
            "Lock-wait race must leave no new authority",
        )?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_birthday_bounds_fail_closed() -> Result<(), DbErr> {
    let app = Application::new().await;
    let account = app.fixture.account.id;
    let household = app.fixture.household.id;
    let guardian = app.fixture.adult.id;
    let result = async {
        let token = app.token().await;
        let (subject, _) = declaration(&app, &token).await?;
        let birthday = Expr::expr(Func::cust("med_tracker.guardian_recovery_latest_date"))
            .sub(Func::cust("make_interval").arg(Expr::val(18)))
            .cast_as(Alias::new("date"));
        people::Entity::update_many()
            .col_expr(people::Column::DateOfBirth, birthday.clone())
            .filter(people::Column::Id.eq(guardian))
            .exec(&app.fixture.admin)
            .await?;
        let boundary = orphan(&app, "Birthday-boundary minor").await?;
        people::Entity::update_many()
            .col_expr(people::Column::DateOfBirth, birthday)
            .filter(people::Column::Id.eq(boundary.person.id))
            .exec(&app.fixture.admin)
            .await?;

        let tenant = access::begin(&app.fixture.runtime, &scope(account, household), None)
            .await
            .map_err(db_error)?;
        let options = recovery::options(&tenant).await.map_err(db_error)?;
        effective_snapshot(&tenant, subject).await?;
        require(
            !options.subjects.iter().any(|item| item.id.get() == subject),
            "Guardian adult in latest worldwide zone must prevent false orphanhood",
        )?;
        require(
            !options
                .guardians
                .iter()
                .any(|item| item.id.get() == guardian),
            "New appointee must be adult in earliest worldwide zone",
        )?;
        require(
            !options
                .subjects
                .iter()
                .any(|item| item.id.get() == boundary.person.id),
            "Subject approaching 18 in latest zone must be excluded",
        )?;
        tenant.rollback().await.map_err(db_error)?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_definer_rejects_ordinary_member_and_foreign_subject() -> Result<(), DbErr>
{
    let app = Application::new().await;
    let account = app.fixture.account.id;
    let household = app.fixture.household.id;
    let guardian = app.fixture.adult.id;
    let backend = app.fixture.admin.get_database_backend();
    let result = async {
        let token = app.token().await;
        let (subject, declaration_id) = declaration(&app, &token).await?;
        let ordinary = actor(&app, "Ordinary recovery member", "member").await?;
        let now = chrono::Utc::now().naive_utc();
        let foreign = people::ActiveModel {
            name: Set("Foreign recovery minor".into()),
            date_of_birth: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1)),
            person_type: Set(1),
            has_capacity: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await?;

        let member_scope = HouseholdScope {
            actor: Actor {
                account_id: ordinary.account.id,
            },
            household_id: household,
            request_id: "synthetic-ordinary-recovery-denial".into(),
        };
        let tenant = access::begin(&app.fixture.runtime, &member_scope, None)
            .await
            .map_err(db_error)?;
        require(
            matches!(
                recovery::options(&tenant).await,
                Err(med_tracker::models::errors::OperationError::Forbidden)
            ),
            "Ordinary member cannot enumerate recovery options",
        )?;
        let raw = tenant
            .transaction()
            .query_one_raw(
                backend.build(
                    &Query::select()
                        .expr(Expr::col("outcome"))
                        .from_function(
                            Func::cust("med_tracker.guardian_recovery_command").args([
                                Expr::val(subject),
                                Expr::val(guardian),
                                Expr::val("00000000-0000-0000-0000-000000000000")
                                    .cast_as(Alias::new("uuid")),
                                Expr::val(0),
                                Expr::val("00000000-0000-0000-0000-000000000000")
                                    .cast_as(Alias::new("uuid")),
                                Expr::val(Option::<i64>::None),
                                Expr::val(Option::<i64>::None),
                                Expr::val("Unauthorised recovery attempt"),
                                Expr::val("synthetic-ordinary"),
                            ]),
                            "command",
                        )
                        .to_owned(),
                ),
            )
            .await;
        require(
            raw.is_err(),
            "Ordinary member cannot invoke the guarded database command",
        )?;
        tenant.rollback().await.map_err(db_error)?;
        let tenant = access::begin(&app.fixture.runtime, &scope(account, household), None)
            .await
            .map_err(db_error)?;
        let raw = tenant
            .transaction()
            .query_one_raw(
                backend.build(
                    &Query::select()
                        .expr(Expr::col("outcome"))
                        .from_function(
                            Func::cust("med_tracker.guardian_recovery_command").args([
                                Expr::val(foreign.id),
                                Expr::val(guardian),
                                Expr::val("00000000-0000-0000-0000-000000000000")
                                    .cast_as(Alias::new("uuid")),
                                Expr::val(0),
                                Expr::val("00000000-0000-0000-0000-000000000000")
                                    .cast_as(Alias::new("uuid")),
                                Expr::val(Option::<i64>::None),
                                Expr::val(Option::<i64>::None),
                                Expr::val("Foreign subject attempt"),
                                Expr::val("synthetic-foreign"),
                            ]),
                            "command",
                        )
                        .to_owned(),
                ),
            )
            .await;
        require(
            raw.is_err(),
            "Manager cannot recover a nonparticipant minor through raw SQL",
        )?;
        tenant.rollback().await.map_err(db_error)?;
        let tenant = access::begin(&app.fixture.runtime, &scope(account, household), None)
            .await
            .map_err(db_error)?;
        let forged = tenant
            .transaction()
            .execute_raw(
                backend.build(
                    &Query::update()
                        .table("carer_relationships")
                        .value(
                            "confirmation_token",
                            Expr::expr(Func::cust("pg_catalog.gen_random_uuid")),
                        )
                        .and_where(
                            Expr::col("guardian_declaration_id").eq(Expr::val(declaration_id)),
                        )
                        .to_owned(),
                ),
            )
            .await;
        require(
            forged.is_err(),
            "Caller cannot forge the opaque relationship revision",
        )?;
        tenant.rollback().await.map_err(db_error)?;
        let tenant = access::begin(&app.fixture.runtime, &scope(account, household), None)
            .await
            .map_err(db_error)?;
        let forged = tenant
            .transaction()
            .execute_raw(
                backend.build(
                    &Query::insert()
                        .into_table("guardian_declarations")
                        .columns([
                            "subject_person_id",
                            "guardian_person_id",
                            "origin_household_id",
                            "declared_by_account_id",
                            "subject_name_at_declaration",
                            "source",
                            "statement_version",
                        ])
                        .values_panic([
                            Expr::val(subject),
                            Expr::val(guardian),
                            Expr::val(household),
                            Expr::val(account),
                            Expr::val("Forged manager source"),
                            Expr::val("manager_appointment"),
                            Expr::val(2),
                        ])
                        .to_owned(),
                ),
            )
            .await;
        require(
            forged.is_err(),
            "Caller cannot insert administrative authority outside the guarded command",
        )?;
        tenant.rollback().await.map_err(db_error)?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_restores_only_the_exact_open_local_link() -> Result<(), DbErr> {
    let app = Application::new().await;
    let account = app.fixture.account.id;
    let household = app.fixture.household.id;
    let guardian = app.fixture.adult.id;
    let backend = app.fixture.admin.get_database_backend();
    let result=async {
        let token=app.token().await;
        let (subject,declaration_id)=declaration(&app,&token).await?;
        let link=app.fixture.admin.query_one_raw(backend.build(&Query::select().expr(Expr::col("id")).from("carer_relationships").and_where(Expr::col("guardian_declaration_id").eq(Expr::val(declaration_id)).and(Expr::col("household_id").eq(Expr::val(household)))).to_owned())).await?.ok_or_else(||DbErr::Custom("Missing initial link".into()))?;
        let link_id:i64=link.try_get("","id")?;
        app.fixture.admin.execute_raw(backend.build(&Query::update().table("carer_relationships").value("active",Expr::val(false)).and_where(Expr::col("id").eq(Expr::val(link_id))).to_owned())).await?;
        let tenant=access::begin(&app.fixture.runtime,&scope(account,household),None).await.map_err(db_error)?;
        let options=recovery::options(&tenant).await.map_err(db_error)?;
        let selected=options.subjects.iter().find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Inactive link minor missing".into()))?;
        require(!selected.has_effective_guardian
            && selected.restorable.as_ref().is_some_and(|link|link.declaration_id==declaration_id && link.relationship_id==link_id),
            "Only exact open local declaration/link may be restored")?;
        let restored=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(guardian).map_err(db_error)?,
            expected:selected.snapshot,
            action:RecoveryAction::Restore{declaration_id,relationship_id:link_id},
            reason:"Verified the existing link and current guardian.".into(),
        }).await.map_err(db_error)?;
        require(restored.outcome==RecoveryOutcome::Restored && restored.declaration_id==declaration_id
            && restored.relationship_id==link_id,"Restore must preserve exact provenance IDs")?;
        tenant.commit().await.map_err(db_error)?;
        let count=app.fixture.admin.query_one_raw(backend.build(&Query::select().expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("guardian_declarations").and_where(Expr::col("subject_person_id").eq(Expr::val(subject))).to_owned()),"declarations").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("carer_relationships").and_where(Expr::col("id").eq(Expr::val(link_id)).and(Expr::col("active"))).to_owned()),"active").to_owned())).await?.ok_or_else(||DbErr::Custom("Missing restore state".into()))?;
        require(count.try_get::<i64>("","declarations")?==1 && count.try_get::<i64>("","active")?==1,
            "Restore must not create new declaration history")?;
        app.fixture.admin.execute_raw(backend.build(&Query::update().table("carer_relationships").value("active",Expr::val(false)).and_where(Expr::col("id").eq(Expr::val(link_id))).to_owned())).await?;
        let tenant=access::begin(&app.fixture.runtime,&scope(account,household),None).await.map_err(db_error)?;
        let after_cycle=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Missing subject after link cycle".into()))?;
        require(after_cycle.restorable.is_some() && after_cycle.snapshot.token==selected.snapshot.token
            && after_cycle.snapshot.revision==selected.snapshot.revision
            && after_cycle.snapshot.link_token!=selected.snapshot.link_token,
            "Restore/deactivate must rotate only opaque link confirmation")?;
        let stale=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(guardian).map_err(db_error)?,
            expected:selected.snapshot,
            action:RecoveryAction::Restore{declaration_id,relationship_id:link_id},
            reason:"Old restore confirmation must be stale.".into(),
        }).await;
        require(matches!(stale,Err(med_tracker::models::errors::OperationError::Conflict{ref code,..}) if code=="stale_guardian_recovery"),
            "Previously rendered restore must fail after restore/deactivate ABA")?;
        tenant.rollback().await.map_err(db_error)?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_foreign_active_link_blocks_local_takeover() -> Result<(), DbErr> {
    let app = Application::new().await;
    let account = app.fixture.account.id;
    let household = app.fixture.household.id;
    let guardian = app.fixture.adult.id;
    let backend = app.fixture.admin.get_database_backend();
    let result=async {
        let token=app.token().await;
        let (subject,declaration_id)=declaration(&app,&token).await?;
        let foreign_household=second_household(&app,"Recovery foreign household","member").await?;

        app.fixture.admin.execute_raw(backend.build(&Query::insert().into_table("household_memberships").columns(["household_id","person_id","role","status","joined_at","created_at","updated_at"]).values_panic([Expr::val(foreign_household.id),Expr::val(subject),Expr::val("member"),Expr::val("active"),Expr::expr(Func::cust("now")),Expr::expr(Func::cust("now")),Expr::expr(Func::cust("now"))]).to_owned())).await?;
        app.fixture.admin.execute_raw(backend.build(&Query::update().table("carer_relationships").value("active",Expr::val(false)).and_where(Expr::col("guardian_declaration_id").eq(Expr::val(declaration_id)).and(Expr::col("household_id").eq(Expr::val(household)))).to_owned())).await?;
        app.fixture.admin.execute_unprepared("ALTER TABLE public.carer_relationships DISABLE TRIGGER guardian_recovery_link_guard").await?;
        let seeded=app.fixture.admin.execute_raw(backend.build(&Query::insert().into_table("carer_relationships").columns(["household_id","carer_id","patient_id","guardian_declaration_id","relationship_type","active","created_at","updated_at"]).values_panic([Expr::val(foreign_household.id),Expr::val(guardian),Expr::val(subject),Expr::val(declaration_id),Expr::val("family_member"),Expr::val(true),Expr::expr(Func::cust("now")),Expr::expr(Func::cust("now"))]).to_owned())).await;
        app.fixture.admin.execute_unprepared("ALTER TABLE public.carer_relationships ENABLE TRIGGER guardian_recovery_link_guard").await?;
        seeded?;
        let tenant=access::begin(&app.fixture.runtime,&scope(account,household),None).await.map_err(db_error)?;
        let options=recovery::options(&tenant).await.map_err(db_error)?;
        let snapshot=effective_snapshot(&tenant,subject).await?;
        require(!options.subjects.iter().any(|item|item.id.get()==subject),
            "A healthy foreign relationship must count despite local inactivity")?;
        let attempted=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(guardian).map_err(db_error)?,
            expected:snapshot,
            action:RecoveryAction::Appoint,
            reason:"Attempted appointment with foreign authority.".into(),
        }).await;
        require(matches!(attempted,Err(med_tracker::models::errors::OperationError::Conflict{ref code,..}) if code=="guardian_already_effective"),
            "A globally effective foreign guardian must block local appointment")?;
        tenant.rollback().await.map_err(db_error)?;
        let candidate=actor(&app,"Synthetic lockout appointee","member").await?;
        let manager=actor(&app,"Synthetic lockout manager","administrator").await?;
        lockout(&app.fixture.admin,account).await?;

        let manager_scope=HouseholdScope {
            actor:Actor {account_id:manager.account.id},
            household_id:household,
            request_id:"synthetic-foreign-lockout-recovery".into(),
        };
        let tenant=access::begin(&app.fixture.runtime,&manager_scope,None).await.map_err(db_error)?;
        let options=recovery::options(&tenant).await.map_err(db_error)?;
        let snapshot=effective_snapshot(&tenant,subject).await?;
        require(!options.subjects.iter().any(|item|item.id.get()==subject),
            "Temporary foreign guardian login lockout must not create orphanhood")?;
        let attempted=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(candidate.person.id).map_err(db_error)?,
            expected:snapshot,
            action:RecoveryAction::Appoint,
            reason:"Lockout must not permit a replacement appointment.".into(),
        }).await;
        require(matches!(attempted,Err(med_tracker::models::errors::OperationError::Conflict{ref code,..}) if code=="guardian_already_effective"),
            "Temporary foreign login lockout must block a distinct adult appointment")?;
        tenant.rollback().await.map_err(db_error)?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_new_guardian_records_manager_and_remains_withdrawable()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let account = app.fixture.account.id;
    let household = app.fixture.household.id;
    let backend = app.fixture.admin.get_database_backend();
    let result = async {
        let token = app.token().await;
        let (subject, old_declaration) = declaration(&app, &token).await?;
        let candidate = actor(&app, "Synthetic new guardian", "member").await?;

        app.fixture
            .admin
            .execute_raw(
                backend.build(
                    &Query::update()
                        .table("carer_relationships")
                        .value("active", Expr::val(false))
                        .and_where(
                            Expr::col("guardian_declaration_id")
                                .eq(Expr::val(old_declaration))
                                .and(Expr::col("household_id").eq(Expr::val(household))),
                        )
                        .to_owned(),
                ),
            )
            .await?;
        let tenant = access::begin(&app.fixture.runtime, &scope(account, household), None)
            .await
            .map_err(db_error)?;
        let options = recovery::options(&tenant).await.map_err(db_error)?;
        let selected = options
            .subjects
            .iter()
            .find(|item| item.id.get() == subject)
            .ok_or_else(|| DbErr::Custom("Minor missing for new appointee".into()))?;
        require(
            selected.restorable.is_some()
                && options
                    .guardians
                    .iter()
                    .any(|item| item.id.get() == candidate.person.id),
            "Old link can restore, while a distinct adult may be deliberately appointed",
        )?;
        let appointed = recovery::execute(
            &tenant,
            RecoveryCommand {
                subject: RecoverySubjectId::parse(subject).map_err(db_error)?,
                guardian: RecoveryGuardianId::parse(candidate.person.id).map_err(db_error)?,
                expected: selected.snapshot,
                action: RecoveryAction::Appoint,
                reason: "Confirmed the new adult accepts this guardianship.".into(),
            },
        )
        .await
        .map_err(|error| DbErr::Custom(format!("new appointment: {error:?}")))?;
        require(
            appointed.outcome == RecoveryOutcome::Appointed,
            "Distinct adult must be appointed",
        )?;
        tenant.commit().await.map_err(db_error)?;
        let state = app
            .fixture
            .admin
            .query_one_raw(
                backend.build(
                    &Query::select()
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("guardian_declarations")
                                    .and_where(
                                        Expr::col("id")
                                            .eq(Expr::val(appointed.declaration_id))
                                            .and(
                                                Expr::col("source")
                                                    .eq(Expr::val("manager_appointment")),
                                            )
                                            .and(
                                                Expr::col("declared_by_account_id")
                                                    .eq(Expr::val(account)),
                                            )
                                            .and(
                                                Expr::col("guardian_person_id")
                                                    .eq(Expr::val(candidate.person.id)),
                                            )
                                            .and(Expr::col("statement_version").eq(Expr::val(2))),
                                    )
                                    .to_owned(),
                            ),
                            "appointed",
                        )
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("guardian_declarations")
                                    .and_where(
                                        Expr::col("id")
                                            .eq(Expr::val(old_declaration))
                                            .and(Expr::col("superseded_at").is_not_null())
                                            .and(
                                                Expr::col("superseded_by_account_id")
                                                    .eq(Expr::val(account)),
                                            ),
                                    )
                                    .to_owned(),
                            ),
                            "superseded",
                        )
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("security_audit_events")
                                    .and_where(
                                        Expr::col("event_type")
                                            .eq(Expr::val("guardian_recovery.appointed"))
                                            .and(
                                                Expr::col("actor_account_id")
                                                    .eq(Expr::val(account)),
                                            )
                                            .and(
                                                Expr::expr(
                                                    Func::cust("jsonb_extract_path_text").args([
                                                        Expr::col("metadata"),
                                                        Expr::val("guardian_person_id"),
                                                    ]),
                                                )
                                                .eq(Expr::val(candidate.person.id.to_string())),
                                            ),
                                    )
                                    .and_where(Expr::col("household_id").eq(household))
                                    .to_owned(),
                            ),
                            "audited",
                        )
                        .to_owned(),
                ),
            )
            .await?
            .ok_or_else(|| DbErr::Custom("Missing manager provenance".into()))?;
        require(
            state.try_get::<i64>("", "appointed")? == 1
                && state.try_get::<i64>("", "superseded")? == 1
                && state.try_get::<i64>("", "audited")? == 1,
            "New authority must credit manager, supersede old source and audit actual appointee",
        )?;
        crate::fixtures::seed_native_password(&app.fixture.admin, candidate.account.id).await;
        let guardian_token = foreign_token(&app, candidate.account.id).await?;
        let own = app
            .client
            .get(format!("{}/api/v1/auth/guardian-declarations", app.origin))
            .bearer_auth(&guardian_token)
            .send()
            .await
            .map_err(db_error)?;
        require(
            own.status().as_u16() == 200,
            "New guardian can read personal history",
        )?;
        let body: Value = own.json().await.map_err(db_error)?;
        let expected_id = appointed.declaration_id.to_string();
        require(
            body["data"].as_array().is_some_and(|rows| {
                rows.iter()
                    .any(|row| row["id"] == expected_id && row["source"] == "manager_appointment")
            }),
            "Own history must expose manager appointment source",
        )?;
        let withdraw = app
            .client
            .patch(format!(
                "{}/api/v1/auth/guardian-declarations/{}",
                app.origin, appointed.declaration_id
            ))
            .bearer_auth(&guardian_token)
            .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
            .send()
            .await
            .map_err(db_error)?;
        require(
            withdraw.status().as_u16() == 200,
            "New guardian must retain personal withdrawal",
        )?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_superseded_active_link_remains_history_without_authority()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let account = app.fixture.account.id;
    let household = app.fixture.household.id;
    let result = async {
use sea_orm::sea_query::{Alias, Expr, ExprTrait, JoinType, Query};
        let now=chrono::Utc::now().naive_utc();
        let child=orphan(&app,"Historical guardian subject").await?;
        let old_actor=actor(&app,"Historical old guardian","member").await?;
        let new_actor=actor(&app,"Historical new guardian","member").await?;
        let old_scope = HouseholdScope { actor: Actor { account_id: old_actor.account.id },
            household_id: household, request_id: "synthetic-old-guardian-authority".into() };
        let old = access::begin(&app.fixture.runtime,&old_scope,None).await.map_err(db_error)?;
        let declaration=Query::insert()
            .into_table((Alias::new("public"),Alias::new("guardian_declarations")))
            .columns(["subject_person_id","guardian_person_id","origin_household_id",
                "declared_by_account_id","subject_name_at_declaration"])
            .values_panic([Expr::val(child.person.id),Expr::val(old_actor.person.id),Expr::val(household),
                Expr::val(old_actor.account.id),Expr::val("Historical guardian subject")])
            .returning_col("id")
            .to_owned();
        let row=old.transaction().query_one_raw(
            old.transaction().get_database_backend().build(&declaration)).await?
            .ok_or_else(||DbErr::Custom("Missing historical declaration".into()))?;
        let old_declaration:i64=row.try_get("","id")?;
        carer_relationships::Entity::insert(carer_relationships::ActiveModel {
            household_id:Set(household),carer_id:Set(old_actor.person.id),patient_id:Set(child.person.id),
            guardian_declaration_id:Set(Some(old_declaration)),
            relationship_type:Set(Some("family_member".into())),active:Set(true),
            created_at:Set(now),updated_at:Set(now),..Default::default()
        }).exec(old.transaction()).await?;
        require(access::require_person_access(&old,child.person.id,PersonAccess::Manage).await.is_ok(),
            "Initial active link must grant old guardian effective authority before recovery")?;
        old.commit().await.map_err(db_error)?;
        let disable=Query::update().table(accounts::Entity)
            .value(accounts::Column::Status,1)
            .and_where(Expr::col(accounts::Column::Id).eq(old_actor.account.id)).to_owned();
        app.fixture.admin.execute_raw(
            app.fixture.admin.get_database_backend().build(&disable)).await?;
        let tenant=access::begin(&app.fixture.runtime,&scope(account,household),None).await.map_err(db_error)?;
        let selected=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==child.person.id)
            .ok_or_else(||DbErr::Custom("Historical orphan missing from options".into()))?;
        require(!selected.has_effective_guardian && selected.restorable.is_none(),
            "Unusable former guardian must leave an orphan but no restorable local link")?;
        let appointed=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(child.person.id).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(new_actor.person.id).map_err(db_error)?,
            expected:selected.snapshot,action:RecoveryAction::Appoint,
            reason:"Manager appointed another adult while old guardian unusable.".into(),
        }).await.map_err(db_error)?;
        tenant.commit().await.map_err(db_error)?;
        let historical=Query::select()
            .expr_as(Expr::col(("d","superseded_at")).is_not_null(),"terminal")
            .expr_as(Expr::col(("c","active")),"historical_link_active")
            .from_as((Alias::new("public"),Alias::new("guardian_declarations")),"d")
            .join_as(JoinType::InnerJoin,
                (Alias::new("public"),Alias::new("carer_relationships")),"c",
                Expr::col(("c","guardian_declaration_id")).equals(("d","id")))
            .and_where(Expr::col(("d","subject_person_id")).eq(child.person.id))
            .and_where(Expr::col(("d","guardian_person_id")).eq(old_actor.person.id))
            .to_owned();
        let row=app.fixture.admin.query_one_raw(
            app.fixture.admin.get_database_backend().build(&historical)).await?
            .ok_or_else(||DbErr::Custom("Missing historical link".into()))?;
        require(row.try_get::<bool>("","terminal")? && row.try_get::<bool>("","historical_link_active")?
            && appointed.outcome==RecoveryOutcome::Appointed,
            "Superseded declaration and its retained active relationship must remain truthful history")?;
        let enable=Query::update().table(accounts::Entity)
            .value(accounts::Column::Status,2)
            .and_where(Expr::col(accounts::Column::Id).eq(old_actor.account.id)).to_owned();
        app.fixture.admin.execute_raw(
            app.fixture.admin.get_database_backend().build(&enable)).await?;
        let old=access::begin(&app.fixture.runtime,&old_scope,None).await.map_err(db_error)?;
        require(matches!(access::require_person_access(&old,child.person.id,PersonAccess::Manage).await,
            Err(med_tracker::models::errors::OperationError::Forbidden)),
            "Restored old account must not regain authority from retained superseded link")?;
        old.rollback().await.map_err(db_error)?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_stale_confirmation_and_audit_failure_leave_history_unchanged()
-> Result<(), DbErr> {
    let app = Application::new().await;
    let account = app.fixture.account.id;
    let household = app.fixture.household.id;
    let guardian = app.fixture.adult.id;
    let backend = app.fixture.admin.get_database_backend();
    let result=async {
        let token=app.token().await;
        let (subject,old_id)=declaration(&app,&token).await?;
        app.fixture.admin.execute_raw(backend.build(&Query::update().table("carer_relationships").value("active",Expr::val(false)).and_where(Expr::col("guardian_declaration_id").eq(Expr::val(old_id))).to_owned())).await?;
        let tenant=access::begin(&app.fixture.runtime,&scope(account,household),None).await.map_err(db_error)?;
        let selected=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Missing stale-test subject".into()))?;
        tenant.rollback().await.map_err(db_error)?;
        let withdrawn=app.client.patch(format!("{}/api/v1/auth/guardian-declarations/{old_id}",app.origin))
            .bearer_auth(&token)
            .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
            .send().await.map_err(db_error)?;
        require(withdrawn.status().as_u16()==200,"Personal withdrawal changes confirmation snapshot")?;
        let tenant=access::begin(&app.fixture.runtime,&scope(account,household),None).await.map_err(db_error)?;
        let stale=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(guardian).map_err(db_error)?,
            expected:selected.snapshot,
            action:RecoveryAction::Appoint,
            reason:"A stale confirmation must be rejected.".into(),
        }).await;
        require(matches!(stale,Err(med_tracker::models::errors::OperationError::Conflict{ref code,..}) if code=="stale_guardian_recovery"),
            "Terminal transition must invalidate prior confirmation")?;
        tenant.rollback().await.map_err(db_error)?;
        app.fixture.admin.execute_unprepared(
            "CREATE FUNCTION public.synthetic_reject_recovery_audit() RETURNS trigger LANGUAGE plpgsql AS $body$ BEGIN IF NEW.event_type LIKE 'guardian_recovery.%' THEN RAISE EXCEPTION USING ERRCODE='23514',MESSAGE='synthetic audit outage'; END IF; RETURN NEW; END $body$;
             CREATE TRIGGER synthetic_reject_recovery_audit BEFORE INSERT ON public.security_audit_events FOR EACH ROW EXECUTE FUNCTION public.synthetic_reject_recovery_audit()"
        ).await?;
        let tenant=access::begin(&app.fixture.runtime,&scope(account,household),None).await.map_err(db_error)?;
        let latest=recovery::options(&tenant).await.map_err(db_error)?.subjects.into_iter()
            .find(|item|item.id.get()==subject)
            .ok_or_else(||DbErr::Custom("Missing latest subject".into()))?;
        require(latest.snapshot!=selected.snapshot,"Withdrawn row must rotate opaque confirmation")?;
        let failed=recovery::execute(&tenant,RecoveryCommand {
            subject:RecoverySubjectId::parse(subject).map_err(db_error)?,
            guardian:RecoveryGuardianId::parse(guardian).map_err(db_error)?,
            expected:latest.snapshot,
            action:RecoveryAction::Appoint,
            reason:"Auditing must commit with the appointment.".into(),
        }).await;
        require(failed.is_err(),"Audit failure must reject recovery")?;
        tenant.rollback().await.map_err(db_error)?;
        let state=app.fixture.admin.query_one_raw(backend.build(&Query::select().expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("guardian_declarations").and_where(Expr::col("subject_person_id").eq(Expr::val(subject))).to_owned()),"history").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("guardian_declarations").and_where(Expr::col("id").eq(Expr::val(old_id)).and(Expr::col("withdrawn_at").is_not_null()).and(Expr::col("superseded_at").is_null())).to_owned()),"terminal").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("carer_relationships").and_where(Expr::col("patient_id").eq(Expr::val(subject)).and(Expr::col("active"))).to_owned()),"active").expr_as(Expr::from(Query::select().expr(Expr::col(Asterisk).count()).from("versions").and_where(Expr::col("item_type").eq(Expr::val("GuardianDeclaration")).and(Expr::col("event").eq(Expr::val("appointed"))).and(Expr::col("item_id").in_subquery(Query::select().expr(Expr::col("id")).from("guardian_declarations").and_where(Expr::col("subject_person_id").eq(Expr::val(subject))).to_owned()))).to_owned()),"versions").to_owned())).await?.ok_or_else(||DbErr::Custom("Missing rollback state".into()))?;
        require(state.try_get::<i64>("","history")?==1 && state.try_get::<i64>("","terminal")?==1
            && state.try_get::<i64>("","active")?==0 && state.try_get::<i64>("","versions")?==0,
            "Audit rejection must rollback declaration, link and versions")?;
        Ok(())
    }.await;
    let cleanup=app.fixture.admin.execute_unprepared(
        "DROP TRIGGER IF EXISTS synthetic_reject_recovery_audit ON public.security_audit_events;
         DROP FUNCTION IF EXISTS public.synthetic_reject_recovery_audit()").await;
    app.close().await;
    cleanup?;
    result
}

struct SubjectRows {
    person: people::Model,
}
struct ActorRows {
    person: people::Model,
    account: accounts::Model,
    membership: household_memberships::Model,
}

async fn orphan(app: &Application, name: &str) -> Result<SubjectRows, DbErr> {
    orphan_on(&app.fixture.admin, app.fixture.household.id, name).await
}
async fn orphan_on<C: ConnectionTrait>(
    connection: &C,
    household: i64,
    name: &str,
) -> Result<SubjectRows, DbErr> {
    let now = chrono::Utc::now().naive_utc();
    let person = people::ActiveModel {
        name: Set(name.into()),
        date_of_birth: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1)),
        person_type: Set(1),
        has_capacity: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(connection)
    .await?;
    participate(connection, household, person.id, "member").await?;
    Ok(SubjectRows { person })
}
async fn participate<C: ConnectionTrait>(
    connection: &C,
    household: i64,
    person: i64,
    role: &str,
) -> Result<household_memberships::Model, DbErr> {
    let now = chrono::Utc::now().naive_utc();
    household_memberships::ActiveModel {
        household_id: Set(household),
        person_id: Set(person),
        role: Set(role.into()),
        status: Set("active".into()),
        permissions_version: Set(1),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(connection)
    .await
}
async fn actor(app: &Application, name: &str, role: &str) -> Result<ActorRows, DbErr> {
    let now = chrono::Utc::now().naive_utc();
    let person = people::ActiveModel {
        name: Set(name.into()),
        date_of_birth: Set(chrono::NaiveDate::from_ymd_opt(1980, 1, 1)),
        person_type: Set(0),
        has_capacity: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await?;
    let email = format!("recovery-{}@example.test", uuid::Uuid::new_v4());
    let account = accounts::ActiveModel {
        person_id: Set(person.id),
        email: Set(email.clone()),
        status: Set(2),
        created_at: Set(now.and_utc()),
        updated_at: Set(now.and_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await?;
    let membership = participate(
        &app.fixture.admin,
        app.fixture.household.id,
        person.id,
        role,
    )
    .await?;
    Ok(ActorRows {
        person,
        account,
        membership,
    })
}
async fn initial_link<C: ConnectionTrait>(
    connection: &C,
    household: i64,
    guardian: i64,
    account: i64,
    subject: i64,
    name: &str,
) -> Result<i64, DbErr> {
    let query = Query::insert()
        .into_table("guardian_declarations")
        .columns([
            "subject_person_id",
            "guardian_person_id",
            "origin_household_id",
            "declared_by_account_id",
            "subject_name_at_declaration",
        ])
        .values_panic([
            Expr::val(subject),
            Expr::val(guardian),
            Expr::val(household),
            Expr::val(account),
            Expr::val(name),
        ])
        .returning_col("id")
        .to_owned();
    let id = connection
        .query_one_raw(connection.get_database_backend().build(&query))
        .await?
        .ok_or_else(|| DbErr::Custom("Missing initial guardian declaration".into()))?
        .try_get::<i64>("", "id")?;
    let now = chrono::Utc::now().naive_utc();
    carer_relationships::ActiveModel {
        household_id: Set(household),
        carer_id: Set(guardian),
        patient_id: Set(subject),
        guardian_declaration_id: Set(Some(id)),
        relationship_type: Set(Some("family_member".into())),
        active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(connection)
    .await?;
    Ok(id)
}
async fn second_household(
    app: &Application,
    name: &str,
    role: &str,
) -> Result<households::Model, DbErr> {
    let now = chrono::Utc::now().naive_utc();
    let household = households::ActiveModel {
        created_by_account_id: Set(Some(app.fixture.account.id)),
        name: Set(name.into()),
        slug: Set(format!("recovery-{}", uuid::Uuid::new_v4())),
        timezone: Set("UTC".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await?;
    participate(&app.fixture.admin, household.id, app.fixture.adult.id, role).await?;
    Ok(household)
}
async fn lockout<C: ConnectionTrait>(connection: &C, account: i64) -> Result<(), DbErr> {
    account_lockouts::ActiveModel {
        account_id: Set(account),
        key: Set(uuid::Uuid::new_v4().to_string()),
        deadline: Set(chrono::Utc::now().naive_utc() + chrono::Duration::hours(1)),
        ..Default::default()
    }
    .insert(connection)
    .await?;
    Ok(())
}

async fn next_declaration_id(app: &Application) -> Result<i64, DbErr> {
    let query = Query::select()
        .expr_as(
            Func::cust("nextval").arg(
                Func::cust("pg_get_serial_sequence")
                    .args([Expr::val("public.guardian_declarations"), Expr::val("id")]),
            ),
            "id",
        )
        .to_owned();
    app.fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await?
        .ok_or_else(|| DbErr::Custom("Missing declaration sequence".into()))?
        .try_get("", "id")
}
async fn set_declaration_id(app: &Application, id: i64) -> Result<(), DbErr> {
    let query = Query::select()
        .expr(
            Func::cust("setval").args([
                Expr::expr(
                    Func::cust("pg_get_serial_sequence")
                        .args([Expr::val("public.guardian_declarations"), Expr::val("id")]),
                ),
                Expr::val(id),
                Expr::val(false),
            ]),
        )
        .to_owned();
    app.fixture
        .admin
        .execute_raw(app.fixture.admin.get_database_backend().build(&query))
        .await?;
    Ok(())
}
async fn restore_declaration_sequence(app: &Application, high: i64) -> Result<(), DbErr> {
    let query = Query::select()
        .expr_as(
            Func::cust("coalesce").args([Expr::col("id").max(), Expr::val(0_i64)]),
            "id",
        )
        .from("guardian_declarations")
        .to_owned();
    let maximum = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await?
        .ok_or_else(|| DbErr::Custom("Missing retained declaration maximum".into()))?
        .try_get::<i64>("", "id")?;
    set_declaration_id(app, std::cmp::max(maximum, high).checked_add(1).unwrap()).await
}
