use super::*;
use chrono::{Timelike, Utc};
use med_tracker::models::_entities::notification_deliveries;
use med_tracker::models::{
    access::{Actor, HouseholdScope},
    care::browser_push::{self, Delivery, Service, Transport},
    entities::push_subscription,
};
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, Set};
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

use med_tracker::models::_entities::{accounts, people, person_access_grants, schedules};
use sea_orm::sea_query::{Alias, Expr, ExprTrait, Func, Query};
use sea_orm::{ColumnTrait, ConnectionTrait, QueryFilter, Statement, TransactionTrait};

async fn account_for(app: &Application, name: &str) -> accounts::Model {
    accounts::Entity::find()
        .filter(accounts::Column::PersonId.eq(participant_named(app, name).await.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}
async fn participant_named(app: &Application, name: &str) -> people::Model {
    people::Entity::find()
        .filter(people::Column::PortableId.eq(format!("{}-{name}", app.fixture.client_uuid)))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}
async fn schedule_for(app: &Application, person: i64) -> schedules::Model {
    schedules::Entity::find()
        .filter(schedules::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(schedules::Column::PersonId.eq(person))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}
async fn grant_for(app: &Application, person: i64) -> person_access_grants::Model {
    person_access_grants::Entity::find()
        .filter(person_access_grants::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(person_access_grants::Column::HouseholdMembershipId.eq(app.fixture.membership.id))
        .filter(person_access_grants::Column::PersonId.eq(person))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}
async fn take_for(app: &Application) -> medication_takes::Model {
    medication_takes::Entity::find()
        .filter(medication_takes::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(medication_takes::Column::PersonMedicationId.eq(app.fixture.assignment.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}
fn pending_take_id(app: &Application) -> i64 {
    i64::from_be_bytes(app.fixture.client_uuid.as_bytes()[..8].try_into().unwrap()).saturating_abs()
}

async fn own_recipients(
    app: &Application,
    now: chrono::DateTime<Utc>,
) -> Result<Vec<browser_push::scheduler::ReminderRun>, med_tracker::models::errors::OperationError>
{
    let runs = browser_push::scheduler::recipients(&app.context, now).await?;
    Ok(runs
        .into_iter()
        .filter(|run| {
            run.account_id == app.fixture.account.id && run.household_id == app.fixture.household.id
        })
        .collect())
}
fn scoped_count(table: &str, condition: Expr) -> Expr {
    Query::select()
        .expr(Expr::col("id").count())
        .from(Alias::new(table.to_owned()))
        .and_where(condition)
        .to_owned()
        .into()
}
fn delivery_query(app: &Application, id: Option<i64>) -> Statement {
    let mut query = Query::select();
    query
        .columns([
            ("d", "id"),
            ("d", "status"),
            ("d", "outcome"),
            ("d", "attempt_count"),
            ("d", "next_attempt_at"),
        ])
        .expr_as(
            Query::select()
                .column("sent_at")
                .from("notification_events")
                .and_where(Expr::col("id").eq(Expr::col(("d", "notification_event_id"))))
                .to_owned(),
            "sent_at",
        )
        .expr_as(
            Query::select()
                .column("actor_membership_id")
                .from("security_audit_events")
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .and_where(Expr::col("event_type").eq("browser_push.reminder.invalid"))
                .and_where(
                    Func::cust("jsonb_extract_path_text")
                        .args([Expr::col("metadata"), Expr::val("delivery_id")])
                        .eq(Expr::col(("d", "id")).cast_as(Alias::new("text"))),
                )
                .limit(1)
                .to_owned(),
            "actor_membership_id",
        )
        .expr_as(
            Query::select()
                .column("actor_account_id")
                .from("security_audit_events")
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .and_where(Expr::col("event_type").eq("browser_push.reminder.invalid"))
                .and_where(
                    Func::cust("jsonb_extract_path_text")
                        .args([Expr::col("metadata"), Expr::val("delivery_id")])
                        .eq(Expr::col(("d", "id")).cast_as(Alias::new("text"))),
                )
                .limit(1)
                .to_owned(),
            "actor_account_id",
        )
        .expr_as(
            scoped_count(
                "push_subscriptions",
                Expr::col("account_id").eq(Expr::col(("d", "account_id"))),
            )
            .gt(0),
            "subscribed",
        )
        .expr_as(
            scoped_count(
                "security_audit_events",
                Expr::col("household_id")
                    .eq(app.fixture.household.id)
                    .and(Expr::col("event_type").eq("browser_push.reminder.completed"))
                    .and(
                        Func::cust("jsonb_extract_path_text")
                            .args([Expr::col("metadata"), Expr::val("delivery_id")])
                            .eq(Expr::col(("d", "id")).cast_as(Alias::new("text"))),
                    ),
            )
            .gt(0),
            "audited",
        )
        .from_as("notification_deliveries", "d")
        .and_where(Expr::col(("d", "household_id")).eq(app.fixture.household.id))
        .and_where(Expr::col(("d", "account_id")).eq(app.fixture.account.id))
        .order_by(("d", "id"), sea_orm::Order::Asc);
    if let Some(id) = id {
        query.and_where(Expr::col(("d", "id")).eq(id));
    }
    app.fixture.admin.get_database_backend().build(&query)
}
fn delivery_counts(app: &Application) -> Statement {
    let own = Expr::col("household_id")
        .eq(app.fixture.household.id)
        .and(Expr::col("account_id").eq(app.fixture.account.id));
    let query = Query::select()
        .expr_as(
            scoped_count(
                "notification_deliveries",
                own.clone().and(Expr::col("status").eq("accepted")),
            ),
            "accepted",
        )
        .expr_as(
            scoped_count(
                "notification_deliveries",
                own.clone().and(Expr::col("status").eq("invalid")),
            ),
            "invalid",
        )
        .expr_as(
            scoped_count(
                "notification_deliveries",
                own.and(Expr::col("status").eq("pending"))
                    .and(Expr::col("attempt_count").eq(1)),
            ),
            "retryable",
        )
        .to_owned();
    app.fixture.admin.get_database_backend().build(&query)
}
fn event_query(
    app: &Application,
    kind: Option<&str>,
    when: Option<chrono::NaiveDateTime>,
    aggregate: bool,
    status: Option<&str>,
) -> Statement {
    let mut query = Query::select();
    query
        .from("notification_events")
        .and_where(Expr::col("household_id").eq(app.fixture.household.id));
    if aggregate {
        query
            .expr_as(Expr::col("id").count(), "count")
            .expr_as(Expr::col("id").count(), "total")
            .expr_as(Func::count_distinct(Expr::col("event_key")), "keys");
    } else {
        query
            .columns(["id", "person_id", "event_key", "intended_at", "expires_at"])
            .order_by("person_id", sea_orm::Order::Asc);
    }
    if let Some(kind) = kind {
        query.and_where(Expr::col("event_type").eq(kind));
    }
    if let Some(when) = when {
        query.and_where(Expr::col("intended_at").eq(when));
    }
    if let Some(status) = status {
        query.and_where(
            Func::cust("jsonb_extract_path_text")
                .args([Expr::col("metadata"), Expr::val("delivery_status")])
                .eq(status),
        );
        if status == "accepted" {
            query.and_where(Expr::col("sent_at").is_not_null());
        }
    }
    app.fixture.admin.get_database_backend().build(&query)
}
fn audit_query(app: &Application, kind: &str, delivery: Option<i64>) -> Statement {
    let mut query = Query::select();
    query
        .expr_as(Expr::col("id").count(), "count")
        .expr_as(Expr::col("id").count().gt(0), "audited")
        .from("security_audit_events")
        .and_where(Expr::col("household_id").eq(app.fixture.household.id))
        .and_where(Expr::col("event_type").eq(kind));
    if let Some(delivery) = delivery {
        query.and_where(
            Func::cust("jsonb_extract_path_text")
                .args([Expr::col("metadata"), Expr::val("delivery_id")])
                .eq(delivery.to_string()),
        );
    }
    app.fixture.admin.get_database_backend().build(&query)
}
fn update_deliveries(
    app: &Application,
    fields: Vec<(&str, Expr)>,
    status: Option<&str>,
) -> Statement {
    let mut query = Query::update();
    query
        .table("notification_deliveries")
        .and_where(Expr::col("household_id").eq(app.fixture.household.id))
        .and_where(Expr::col("account_id").eq(app.fixture.account.id));
    for (column, value) in fields {
        query.value(Alias::new(column.to_owned()), value);
    }
    if let Some(status) = status {
        query.and_where(Expr::col("status").eq(status));
    }
    app.fixture.admin.get_database_backend().build(&query)
}
fn update_events(app: &Application, expiry: chrono::NaiveDateTime) -> Statement {
    let query = Query::update()
        .table("notification_events")
        .value("expires_at", expiry)
        .and_where(Expr::col("household_id").eq(app.fixture.household.id))
        .to_owned();
    app.fixture.admin.get_database_backend().build(&query)
}
fn update_preference(app: &Application, column: &str, value: Expr) -> Statement {
    let query = Query::update()
        .table("notification_preferences")
        .value(Alias::new(column.to_owned()), value)
        .and_where(Expr::col("household_id").eq(app.fixture.household.id))
        .and_where(Expr::col("person_id").eq(app.fixture.adult.id))
        .to_owned();
    app.fixture.admin.get_database_backend().build(&query)
}
async fn backend_pid(db: &impl ConnectionTrait) -> i32 {
    let query = Query::select()
        .expr_as(Func::cust("pg_backend_pid"), "pid")
        .to_owned();
    db.query_one_raw(db.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap()
}
fn blocking_query(
    app: &Application,
    table: Option<&str>,
    holder: Option<i32>,
    waiter: Option<i32>,
) -> Statement {
    let mut inner = Query::select();
    inner
        .expr(Expr::col("pid").count())
        .from("pg_stat_activity")
        .and_where(Expr::col("datname").eq(Func::cust("current_database")))
        .and_where(Expr::col("wait_event_type").eq("Lock"));
    if let Some(pid) = holder {
        inner.and_where(
            Expr::val(pid).eq(sea_orm::sea_query::extension::postgres::PgFunc::any(
                Func::cust("pg_blocking_pids").arg(Expr::col("pid")),
            )),
        );
    }
    if let Some(pid) = waiter {
        inner.and_where(Expr::col("pid").eq(pid));
    }
    if let Some(table) = table {
        inner.and_where(Expr::col("query").like(format!("%{table}%")));
    }
    let query = Query::select()
        .expr_as(Expr::from(inner.to_owned()).gt(0), "waiting")
        .to_owned();
    app.fixture.admin.get_database_backend().build(&query)
}

#[tokio::test]
async fn browser_reminder_terminal_cleanup_audit_failure_is_atomic() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-cleanup-audit").await;
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(None)]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    browser_push::reminders::deliver_account(&app.context, &scope, now)
        .await
        .unwrap();
    let fault = format!(
        "reject_browser_cleanup_audit_{}",
        uuid::Uuid::new_v4().simple()
    );
    app.fixture
        .admin
        .execute_raw(authority_query(&app, AuthorityChange::Membership))
        .await
        .unwrap();
    // SeaORM has no trigger, PL/pgSQL function or GRANT/REVOKE builder.
    let household_id = app.fixture.household.id;
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION public.{fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.household_id={household_id} AND NEW.event_type='browser_push.reminder.invalid' THEN RAISE EXCEPTION 'synthetic cleanup audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER {fault} BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION public.{fault}()")).await.unwrap();
    assert!(own_recipients(&app, Utc::now()).await.is_err());
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "pending");
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {fault} ON security_audit_events; DROP FUNCTION public.{fault}()"
        ))
        .await
        .unwrap();
    own_recipients(&app, Utc::now()).await.unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "invalid");
    assert_eq!(
        row.try_get::<Option<i64>>("", "actor_membership_id")
            .unwrap(),
        None
    );
    assert_eq!(
        row.try_get::<Option<i64>>("", "actor_account_id").unwrap(),
        None
    );
    assert_eq!(*transport.1.lock().unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_profile_update_uses_same_lock_order() {
    use med_tracker::models::{access, profile};
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-profile-lock").await;
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: Arc::new(Reminders(Mutex::new(vec![]))),
    });
    let tenant = access::begin(&app.context.db, &scope, None).await.unwrap();
    let holder = backend_pid(tenant.transaction()).await;
    tenant
        .transaction()
        .execute_raw({
            let query = Query::select()
                .column("id")
                .from("accounts")
                .and_where(Expr::col("id").eq(app.fixture.account.id))
                .lock(sea_orm::sea_query::LockType::NoKeyUpdate)
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let ctx = app.context.clone();
    let sender =
        tokio::spawn(
            async move { browser_push::reminders::deliver_account(&ctx, &scope, now).await },
        );
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let row = app
                .fixture
                .admin
                .query_one_raw(blocking_query(&app, Some("accounts"), Some(holder), None))
                .await
                .unwrap()
                .unwrap();
            if row.try_get::<bool>("", "waiting").unwrap() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let updated = profile::update(
        &tenant,
        app.fixture.account.id,
        profile::Changes {
            date_of_birth: Some(chrono::NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
            ..Default::default()
        },
        now.date_naive(),
        None,
    )
    .await;
    if updated.is_ok() {
        tenant.commit().await.unwrap();
    } else {
        tenant.rollback().await.unwrap();
    }
    let sent = sender.await.unwrap();
    assert!(updated.is_ok(), "profile failed: {:?}", updated.err());
    assert!(sent.is_ok(), "reminder failed: {sent:?}");
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_lifecycle_revocation_recovers_without_resend() {
    for mutation in [
        AuthorityChange::Membership,
        AuthorityChange::Account,
        AuthorityChange::Household,
    ] {
        let app = Application::new().await;
        let (now, scope) = prepare_due_delivery(&app, "synthetic-lifecycle").await;
        let transport = Arc::new(BlockedAcceptance {
            started: Notify::new(),
            release: Notify::new(),
        });
        app.context.shared_store.insert(Service {
            public_key: String::new(),
            transport: transport.clone(),
        });
        let ctx = app.context.clone();
        let sending_scope = scope.clone();
        let sender = tokio::spawn(async move {
            browser_push::reminders::deliver_account(&ctx, &sending_scope, now).await
        });
        transport.started.notified().await;
        let mut waiter = None;
        let withdrawal = if matches!(mutation, AuthorityChange::Membership) {
            app.fixture
                .admin
                .execute_raw(authority_query(&app, mutation))
                .await
                .unwrap();
            None
        } else {
            let admin = app.fixture.admin.begin().await.unwrap();
            waiter = Some(backend_pid(&admin).await);
            let query = authority_query(&app, mutation);
            Some(tokio::spawn(async move {
                let result = admin.execute_raw(query).await;
                admin.commit().await?;
                result
            }))
        };
        if withdrawal.is_some() {
            let owner = owner_connection(&app).await;
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                loop {
                    let row = owner
                        .query_one_raw(blocking_query(&app, None, None, waiter))
                        .await
                        .unwrap()
                        .unwrap();
                    if row.try_get::<bool>("", "waiting").unwrap() {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
        }
        transport.release.notify_one();
        let _ = sender.await.unwrap();
        if let Some(withdrawal) = withdrawal {
            tokio::time::timeout(std::time::Duration::from_secs(10), withdrawal)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
        }
        app.fixture
            .admin
            .execute_raw({
                let query = Query::update()
                    .table("notification_deliveries")
                    .value(
                        "claimed_at",
                        Func::cust("now")
                            .sub(Expr::val("121 seconds").cast_as(Alias::new("interval"))),
                    )
                    .and_where(Expr::col("status").eq(Expr::val("sending")))
                    .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                    .and_where(Expr::col("account_id").eq(app.fixture.account.id))
                    .to_owned();
                app.fixture.admin.get_database_backend().build(&query)
            })
            .await
            .unwrap();
        let runs = own_recipients(&app, Utc::now()).await.unwrap();
        for run in runs {
            use loco_rs::bgworker::BackgroundWorker;
            browser_push::scheduler::ReminderWorker::build(&app.context)
                .perform(run)
                .await
                .unwrap();
        }
        let row = app
            .fixture
            .admin
            .query_one_raw(delivery_query(&app, None))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<String>("", "status").unwrap(), "uncertain");
        assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 1);
        async {
            let query = Query::update()
                .table("household_memberships")
                .value(
                    "revoked_at",
                    Expr::val(Option::<chrono::NaiveDateTime>::None),
                )
                .and_where(Expr::col("person_id").eq(app.fixture.adult.id))
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .to_owned();
            let _ = app
                .fixture
                .admin
                .execute_raw(app.fixture.admin.get_database_backend().build(&query))
                .await?;
            let query = Query::update()
                .table("accounts")
                .value("status", Expr::val(2))
                .and_where(Expr::col("id").eq(Expr::val(app.fixture.account.id)))
                .to_owned();
            let _ = app
                .fixture
                .admin
                .execute_raw(app.fixture.admin.get_database_backend().build(&query))
                .await?;
            let query = Query::update()
                .table("households")
                .value("status", Expr::val("active"))
                .value("lifecycle_state", Expr::val("active"))
                .and_where(Expr::col("id").eq(Expr::val(app.fixture.household.id)))
                .to_owned();
            app.fixture
                .admin
                .execute_raw(app.fixture.admin.get_database_backend().build(&query))
                .await
        }
        .await
        .unwrap();
        assert_eq!(
            browser_push::reminders::drain_pending(&app.context, &scope, Utc::now())
                .await
                .unwrap(),
            0
        );
        app.close().await;
    }
}

#[tokio::test]
async fn browser_reminder_lifecycle_revocation_invalidates_pending() {
    for mutation in [
        AuthorityChange::Membership,
        AuthorityChange::Account,
        AuthorityChange::Household,
    ] {
        let app = Application::new().await;
        let (now, scope) = prepare_due_delivery(&app, "synthetic-lifecycle-pending").await;
        let transport = Arc::new(ScriptedDelivery(
            Mutex::new(vec![Delivery::Temporary(None)]),
            Mutex::new(0),
        ));
        app.context.shared_store.insert(Service {
            public_key: String::new(),
            transport: transport.clone(),
        });
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap();
        app.fixture
            .admin
            .execute_raw(authority_query(&app, mutation))
            .await
            .unwrap();
        let foreign = app.fixture.foreign_household().await;
        let foreign_id = foreign.household.id;
        foreign.close().await;
        let other_scope = HouseholdScope {
            household_id: foreign_id,
            ..scope.clone()
        };
        let _ = browser_push::reminders::deliver_account(&app.context, &other_scope, now).await;
        let row = app
            .fixture
            .admin
            .query_one_raw(delivery_query(&app, None))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<String>("", "status").unwrap(), "pending");
        let runs = own_recipients(&app, Utc::now()).await.unwrap();
        for run in runs {
            use loco_rs::bgworker::BackgroundWorker;
            browser_push::scheduler::ReminderWorker::build(&app.context)
                .perform(run)
                .await
                .unwrap();
        }
        let row = app
            .fixture
            .admin
            .query_one_raw(delivery_query(&app, None))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<String>("", "status").unwrap(), "invalid");
        async {
            let query = Query::update()
                .table("household_memberships")
                .value(
                    "revoked_at",
                    Expr::val(Option::<chrono::NaiveDateTime>::None),
                )
                .and_where(Expr::col("person_id").eq(app.fixture.adult.id))
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .to_owned();
            let _ = app
                .fixture
                .admin
                .execute_raw(app.fixture.admin.get_database_backend().build(&query))
                .await?;
            let query = Query::update()
                .table("accounts")
                .value("status", Expr::val(2))
                .and_where(Expr::col("id").eq(Expr::val(app.fixture.account.id)))
                .to_owned();
            let _ = app
                .fixture
                .admin
                .execute_raw(app.fixture.admin.get_database_backend().build(&query))
                .await?;
            let query = Query::update()
                .table("households")
                .value("status", Expr::val("active"))
                .value("lifecycle_state", Expr::val("active"))
                .and_where(Expr::col("id").eq(Expr::val(app.fixture.household.id)))
                .to_owned();
            app.fixture
                .admin
                .execute_raw(app.fixture.admin.get_database_backend().build(&query))
                .await
        }
        .await
        .unwrap();
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap();
        assert_eq!(*transport.1.lock().unwrap(), 1);
        app.close().await;
    }
}

struct Reminders(Mutex<Vec<Value>>);

struct PartialDelivery(Mutex<Vec<i64>>);

struct ScriptedDelivery(Mutex<Vec<Delivery>>, Mutex<usize>);

#[async_trait::async_trait]
impl Transport for ScriptedDelivery {
    async fn send(&self, _: &push_subscription::Model, _: &[u8]) -> Delivery {
        *self.1.lock().unwrap() += 1;
        self.0.lock().unwrap().remove(0)
    }
}

struct BlockedAcceptance {
    started: Notify,
    release: Notify,
}

struct FirstBlockedDelivery {
    started: Notify,
    release: Notify,
    calls: Mutex<Vec<(i64, u32)>>,
}

struct TtlCapture(Mutex<Vec<(u32, chrono::DateTime<Utc>)>>);

#[async_trait::async_trait]
impl Transport for TtlCapture {
    async fn send(&self, _: &push_subscription::Model, _: &[u8]) -> Delivery {
        unreachable!()
    }

    async fn send_with_ttl(&self, _: &push_subscription::Model, _: &[u8], ttl: u32) -> Delivery {
        self.0.lock().unwrap().push((ttl, Utc::now()));
        Delivery::Accepted
    }
}

#[async_trait::async_trait]
impl Transport for FirstBlockedDelivery {
    async fn send(&self, _: &push_subscription::Model, _: &[u8]) -> Delivery {
        unreachable!()
    }

    async fn send_with_ttl(
        &self,
        subscription: &push_subscription::Model,
        _: &[u8],
        ttl: u32,
    ) -> Delivery {
        let first = {
            let mut calls = self.calls.lock().unwrap();
            calls.push((subscription.id, ttl));
            calls.len() == 1
        };
        if first {
            self.started.notify_one();
            self.release.notified().await;
        }
        Delivery::Accepted
    }
}

#[tokio::test]
async fn browser_reminder_stale_selected_candidate_cannot_bypass_retry_time() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-stale-candidate").await;
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![
            Delivery::Temporary(None),
            Delivery::Temporary(None),
            Delivery::Temporary(None),
        ]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, None))
        .await
        .unwrap()
        .unwrap();
    let id = row.try_get::<i64>("", "id").unwrap();
    let selected_by_both_drainers = [id];
    let retry_clock = now + chrono::Duration::minutes(2);
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("notification_deliveries")
                .value(
                    "next_attempt_at",
                    Func::cust("now").sub(Expr::val("1 second").cast_as(Alias::new("interval"))),
                )
                .and_where(Expr::col("status").eq(Expr::val("pending")))
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .and_where(Expr::col("account_id").eq(app.fixture.account.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::drain_candidates(
            &app.context,
            &scope,
            retry_clock,
            &selected_by_both_drainers
        )
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        browser_push::reminders::drain_candidates(
            &app.context,
            &scope,
            retry_clock,
            &selected_by_both_drainers
        )
        .await
        .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, Some(id)))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "pending");
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 2);
    assert!(
        row.try_get::<chrono::NaiveDateTime>("", "next_attempt_at")
            .unwrap()
            > retry_clock.naive_utc()
    );
    assert_eq!(*transport.1.lock().unwrap(), 2);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_refreshes_clock_between_device_sends() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-batch-expiry").await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("push_subscriptions")
                .columns([
                    "account_id",
                    "endpoint",
                    "p256dh",
                    "auth",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.account.id),
                    Expr::val(format!(
                        "https://fcm.googleapis.com/fcm/send/{}/second",
                        app.fixture.client_uuid
                    )),
                    Expr::val("synthetic"),
                    Expr::val("synthetic"),
                    Func::cust("now").into(),
                    Func::cust("now").into(),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let transport = Arc::new(FirstBlockedDelivery {
        started: Notify::new(),
        release: Notify::new(),
        calls: Mutex::new(vec![]),
    });
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let ctx = app.context.clone();
    let sending_scope = scope.clone();
    let sender = tokio::spawn(async move {
        browser_push::reminders::deliver_account(&ctx, &sending_scope, now).await
    });
    transport.started.notified().await;
    app.fixture
        .admin
        .execute_raw(update_events(
            &app,
            (now - chrono::Duration::seconds(1)).naive_utc(),
        ))
        .await
        .unwrap();
    transport.release.notify_one();
    assert_eq!(sender.await.unwrap().unwrap(), 1);
    assert_eq!(transport.calls.lock().unwrap().len(), 1);
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_counts(&app))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "accepted").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "invalid").unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_ttl_uses_remaining_lifetime_at_send() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-send-ttl").await;
    let first = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(None)]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: first,
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    async {
        let query = Query::update()
            .table("notification_events")
            .value(
                "expires_at",
                Func::cust("now").add(Expr::val("100 seconds").cast_as(Alias::new("interval"))),
            )
            .and_where(Expr::col("event_key").like("browser:%"))
            .and_where(Expr::col("household_id").eq(app.fixture.household.id))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::update()
            .table("notification_deliveries")
            .value(
                "next_attempt_at",
                Func::cust("now").sub(Expr::val("10 minutes").cast_as(Alias::new("interval"))),
            )
            .and_where(Expr::col("status").eq(Expr::val("pending")))
            .and_where(Expr::col("household_id").eq(app.fixture.household.id))
            .and_where(Expr::col("account_id").eq(app.fixture.account.id))
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
    }
    .await
    .unwrap();
    let transport = Arc::new(TtlCapture(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::drain_pending(
            &app.context,
            &scope,
            now - chrono::Duration::minutes(5)
        )
        .await
        .unwrap(),
        1
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(event_query(&app, None, None, false, None))
        .await
        .unwrap()
        .unwrap();
    let expires_at = row
        .try_get::<chrono::NaiveDateTime>("", "expires_at")
        .unwrap()
        .and_utc();
    {
        let recorded = transport.0.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        let (ttl, sent_at) = recorded[0];
        assert!(ttl <= 100);
        assert!(i64::from(ttl) <= (expires_at - sent_at).num_seconds() + 1);
    }
    app.close().await;
}

#[async_trait::async_trait]
impl Transport for BlockedAcceptance {
    async fn send(&self, _: &push_subscription::Model, _: &[u8]) -> Delivery {
        self.started.notify_one();
        self.release.notified().await;
        Delivery::Accepted
    }
}

async fn prepare_due_delivery(
    app: &Application,
    request_id: &str,
) -> (chrono::DateTime<Utc>, HouseholdScope) {
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    async {
        let query = Query::update()
            .table("accounts")
            .value(
                "preferences",
                Func::cust("jsonb_build_object").args([Expr::val("time_zone"), Expr::val("UTC")]),
            )
            .and_where(Expr::col("id").eq(Expr::val(app.fixture.account.id)))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::update()
            .table("person_medications")
            .value("administration_kind", Expr::val(0))
            .value("dose_cycle", Expr::val(0))
            .value("max_daily_doses", Expr::val(1))
            .value(
                "created_at",
                Func::cust("now").sub(Expr::val("2 days").cast_as(Alias::new("interval"))),
            )
            .and_where(Expr::col("id").eq(Expr::val(app.fixture.assignment.id)))
            .and_where(Expr::col("household_id").eq(app.fixture.household.id))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("push_subscriptions")
            .columns([
                "account_id",
                "endpoint",
                "p256dh",
                "auth",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.account.id),
                Expr::val(format!(
                    "https://fcm.googleapis.com/fcm/send/{}/failed",
                    app.fixture.client_uuid
                )),
                Expr::val("synthetic"),
                Expr::val("synthetic"),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
    }
    .await
    .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("notification_preferences")
                .columns([
                    "household_id",
                    "person_id",
                    "enabled",
                    "dose_due_enabled",
                    "missed_dose_enabled",
                    "low_stock_enabled",
                    "private_text_enabled",
                    "morning_time",
                    "afternoon_time",
                    "evening_time",
                    "night_time",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(app.fixture.adult.id),
                    Expr::val(true),
                    Expr::val(true),
                    Expr::val(false),
                    Expr::val(false),
                    Expr::val(true),
                    Expr::val(now.time()),
                    Expr::val(Option::<chrono::NaiveTime>::None),
                    Expr::val(Option::<chrono::NaiveTime>::None),
                    Expr::val(Option::<chrono::NaiveTime>::None),
                    Func::cust("now").into(),
                    Func::cust("now").into(),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    (
        now,
        HouseholdScope {
            actor: Actor {
                account_id: app.fixture.account.id,
            },
            household_id: app.fixture.household.id,
            request_id: request_id.into(),
        },
    )
}

async fn assert_pending_reminder_invalid_after_change(
    app: &Application,
    scope: &HouseholdScope,
    transport: &ScriptedDelivery,
) {
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("notification_deliveries")
                .value(
                    "next_attempt_at",
                    Func::cust("now").sub(Expr::val("1 second").cast_as(Alias::new("interval"))),
                )
                .and_where(Expr::col("status").eq(Expr::val("pending")))
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .and_where(Expr::col("account_id").eq(app.fixture.account.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::drain_pending(&app.context, scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(app, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "invalid");
    assert_eq!(
        row.try_get::<String>("", "outcome").unwrap(),
        "no_longer_eligible"
    );
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 1);
    assert_eq!(*transport.1.lock().unwrap(), 1);
}

#[tokio::test]
async fn browser_reminder_stops_pending_delivery_after_dose_outcome() {
    for outcome in ["taken", "not_taken"] {
        let app = Application::new().await;
        let (now, scope) = prepare_due_delivery(&app, "synthetic-dose-outcome").await;
        let transport = Arc::new(ScriptedDelivery(
            Mutex::new(vec![Delivery::Temporary(None)]),
            Mutex::new(0),
        ));
        app.context.shared_store.insert(Service {
            public_key: String::new(),
            transport: transport.clone(),
        });
        assert_eq!(
            browser_push::reminders::deliver_account(&app.context, &scope, now)
                .await
                .unwrap(),
            0
        );
        if outcome == "taken" {
            app.fixture
                .admin
                .execute_raw({
                    let query = Query::insert()
                        .into_table("medication_takes")
                        .columns([
                            "household_id",
                            "person_medication_id",
                            "dose_amount",
                            "dose_unit",
                            "taken_at",
                            "created_at",
                            "updated_at",
                        ])
                        .values_panic([
                            Expr::val(app.fixture.household.id),
                            Expr::val(app.fixture.assignment.id),
                            Expr::val(2),
                            Expr::val("tablet"),
                            Func::cust("now").into(),
                            Func::cust("now").into(),
                            Func::cust("now").into(),
                        ])
                        .to_owned();
                    app.fixture.admin.get_database_backend().build(&query)
                })
                .await
                .unwrap();
            app.fixture
                .admin
                .execute_raw({
                    let query = Query::insert()
                        .into_table("medication_dose_occurrences")
                        .columns([
                            "household_id",
                            "person_medication_id",
                            "medication_take_id",
                            "window_starts_on",
                            "window_ends_on",
                            "position",
                            "outcome",
                            "resolved_at",
                            "resolved_by_membership_id",
                            "created_at",
                            "updated_at",
                        ])
                        .values_panic([
                            Expr::val(app.fixture.household.id),
                            Expr::val(app.fixture.assignment.id),
                            Expr::val(take_for(&app).await.id),
                            Expr::val(now.date_naive()),
                            Expr::val(now.date_naive()),
                            Expr::val(1),
                            Expr::val("taken"),
                            Func::cust("now").into(),
                            Expr::val(app.fixture.membership.id),
                            Func::cust("now").into(),
                            Func::cust("now").into(),
                        ])
                        .to_owned();
                    app.fixture.admin.get_database_backend().build(&query)
                })
                .await
                .unwrap();
        } else {
            app.fixture
                .admin
                .execute_raw({
                    let query = Query::insert()
                        .into_table("medication_dose_occurrences")
                        .columns([
                            "household_id",
                            "person_medication_id",
                            "window_starts_on",
                            "window_ends_on",
                            "position",
                            "outcome",
                            "reason",
                            "resolved_at",
                            "resolved_by_membership_id",
                            "created_at",
                            "updated_at",
                        ])
                        .values_panic([
                            Expr::val(app.fixture.household.id),
                            Expr::val(app.fixture.assignment.id),
                            Expr::val(now.date_naive()),
                            Expr::val(now.date_naive()),
                            Expr::val(1),
                            Expr::val("not_taken"),
                            Expr::val("other"),
                            Func::cust("now").into(),
                            Expr::val(app.fixture.membership.id),
                            Func::cust("now").into(),
                            Func::cust("now").into(),
                        ])
                        .to_owned();
                    app.fixture.admin.get_database_backend().build(&query)
                })
                .await
                .unwrap();
        }
        assert_pending_reminder_invalid_after_change(&app, &scope, &transport).await;
        app.close().await;
    }
}

#[tokio::test]
async fn browser_reminder_stops_pending_delivery_after_schedule_pause() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-schedule-pause").await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("person_medications")
                .value("active", Expr::val(false))
                .and_where(Expr::col("id").eq(Expr::val(app.fixture.assignment.id)))
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("schedules")
                .columns([
                    "household_id",
                    "person_id",
                    "medication_id",
                    "active",
                    "start_date",
                    "end_date",
                    "dose_amount",
                    "dose_unit",
                    "schedule_type",
                    "schedule_config",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(app.fixture.adult.id),
                    Expr::val(app.fixture.medication.id),
                    Expr::val(true),
                    Expr::val(now.date_naive()),
                    Expr::val(now.date_naive()),
                    Expr::val(2),
                    Expr::val("tablet"),
                    Expr::val(0),
                    Func::cust("jsonb_build_object")
                        .args([
                            Expr::val("times"),
                            Func::cust("jsonb_build_array")
                                .args([Expr::val(now.format("%H:%M").to_string())])
                                .into(),
                        ])
                        .into(),
                    Func::cust("now").sub(Expr::val("2 days").cast_as(Alias::new("interval"))),
                    Func::cust("now").into(),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(None)]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("medication_pause_periods")
                .columns([
                    "household_id",
                    "schedule_id",
                    "reason",
                    "started_at",
                    "recorded_by_membership_id",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(schedule_for(&app, app.fixture.adult.id).await.id),
                    Expr::val("other"),
                    Func::cust("now").sub(Expr::val("5 minutes").cast_as(Alias::new("interval"))),
                    Expr::val(app.fixture.membership.id),
                    Func::cust("now").into(),
                    Func::cust("now").into(),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    assert_pending_reminder_invalid_after_change(&app, &scope, &transport).await;
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_stops_pending_missed_delivery_after_grant_withdrawal() {
    let app = Application::new().await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let scheduled = now - chrono::Duration::minutes(30);
    async {
        let query = Query::update()
            .table("accounts")
            .value(
                "preferences",
                Func::cust("jsonb_build_object").args([Expr::val("time_zone"), Expr::val("UTC")]),
            )
            .and_where(Expr::col("id").eq(Expr::val(app.fixture.account.id)))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("person_access_grants")
            .columns([
                "household_id",
                "household_membership_id",
                "person_id",
                "access_level",
                "relationship_type",
                "missed_dose_notifications_enabled",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.household.id),
                Expr::val(app.fixture.membership.id),
                Expr::val(app.fixture.minor.id),
                Expr::val("manage"),
                Expr::val("parent"),
                Expr::val(true),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("notification_preferences")
            .columns([
                "household_id",
                "person_id",
                "enabled",
                "dose_due_enabled",
                "missed_dose_enabled",
                "low_stock_enabled",
                "private_text_enabled",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.household.id),
                Expr::val(app.fixture.adult.id),
                Expr::val(true),
                Expr::val(false),
                Expr::val(true),
                Expr::val(false),
                Expr::val(true),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("push_subscriptions")
            .columns([
                "account_id",
                "endpoint",
                "p256dh",
                "auth",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.account.id),
                Expr::val(format!(
                    "https://fcm.googleapis.com/fcm/send/{}/grant-withdrawal",
                    app.fixture.client_uuid
                )),
                Expr::val("synthetic"),
                Expr::val("synthetic"),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
    }
    .await
    .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("schedules")
                .columns([
                    "household_id",
                    "person_id",
                    "medication_id",
                    "active",
                    "start_date",
                    "end_date",
                    "dose_amount",
                    "dose_unit",
                    "schedule_type",
                    "schedule_config",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(app.fixture.minor.id),
                    Expr::val(app.fixture.medication.id),
                    Expr::val(true),
                    Expr::val(now.date_naive() - chrono::Duration::days(1)),
                    Expr::val(now.date_naive() + chrono::Duration::days(1)),
                    Expr::val(2),
                    Expr::val("tablet"),
                    Expr::val(0),
                    Func::cust("jsonb_build_object")
                        .args([
                            Expr::val("times"),
                            Func::cust("jsonb_build_array")
                                .args([Expr::val(scheduled.format("%H:%M").to_string())])
                                .into(),
                        ])
                        .into(),
                    Func::cust("now").sub(Expr::val("2 days").cast_as(Alias::new("interval"))),
                    Func::cust("now").into(),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(None)]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-grant-withdrawal".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("person_access_grants")
                .value("revoked_at", Func::cust("now"))
                .and_where(
                    Expr::col("id").eq(Expr::val(grant_for(&app, app.fixture.minor.id).await.id)),
                )
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    assert_pending_reminder_invalid_after_change(&app, &scope, &transport).await;
    app.close().await;
}

async fn prepare_pending_low_stock(
    app: &Application,
    crossed_at: chrono::DateTime<Utc>,
) -> HouseholdScope {
    async {
        let query = Query::update()
            .table("medications")
            .value("current_supply", Expr::val(9))
            .value("reorder_threshold", Expr::val(10))
            .and_where(Expr::col("id").eq(Expr::val(app.fixture.medication.id)))
            .and_where(Expr::col("household_id").eq(app.fixture.household.id))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("notification_preferences")
            .columns([
                "household_id",
                "person_id",
                "enabled",
                "dose_due_enabled",
                "missed_dose_enabled",
                "low_stock_enabled",
                "private_text_enabled",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.household.id),
                Expr::val(app.fixture.adult.id),
                Expr::val(true),
                Expr::val(false),
                Expr::val(false),
                Expr::val(true),
                Expr::val(true),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("push_subscriptions")
            .columns([
                "account_id",
                "endpoint",
                "p256dh",
                "auth",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.account.id),
                Expr::val(format!(
                    "https://fcm.googleapis.com/fcm/send/{}/stock-catchup",
                    app.fixture.client_uuid
                )),
                Expr::val("synthetic"),
                Expr::val("synthetic"),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
    }
    .await
    .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("notification_events")
                .columns([
                    "household_id",
                    "person_id",
                    "event_type",
                    "event_key",
                    "metadata",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(app.fixture.adult.id),
                    Expr::val("browser_low_stock_pending"),
                    Expr::val(format!(
                        "low-stock:{}:{}:{}",
                        app.fixture.medication.id,
                        pending_take_id(app),
                        app.fixture.adult.id
                    )),
                    Func::cust("jsonb_build_object")
                        .args([
                            Expr::val("account_id"),
                            Expr::val(app.fixture.account.id),
                            Expr::val("medication_id"),
                            Expr::val(app.fixture.medication.id),
                            Expr::val("take_id"),
                            Expr::val(pending_take_id(app)),
                        ])
                        .into(),
                    Expr::val(crossed_at.naive_utc()),
                    Func::cust("now").into(),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-stock-catchup".into(),
    }
}

#[tokio::test]
async fn browser_low_stock_catches_up_and_stops_pending_delivery_after_refill() {
    let app = Application::new().await;
    let crossed_at = (Utc::now() - chrono::Duration::hours(2))
        .with_nanosecond(0)
        .unwrap();
    let scope = prepare_pending_low_stock(&app, crossed_at).await;
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(None)]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(event_query(&app, Some("low_stock"), None, false, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.try_get::<chrono::NaiveDateTime>("", "intended_at")
            .unwrap(),
        crossed_at.naive_utc()
    );
    assert_eq!(
        row.try_get::<chrono::NaiveDateTime>("", "expires_at")
            .unwrap(),
        (crossed_at + chrono::Duration::hours(24)).naive_utc()
    );
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("medications")
                .value("current_supply", Expr::val(11))
                .and_where(Expr::col("id").eq(Expr::val(app.fixture.medication.id)))
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    assert_pending_reminder_invalid_after_change(&app, &scope, &transport).await;
    app.close().await;
}

#[tokio::test]
async fn browser_low_stock_does_not_reserve_at_exact_24_hour_expiry() {
    let app = Application::new().await;
    let now = Utc::now();
    let scope = prepare_pending_low_stock(&app, now - chrono::Duration::hours(24)).await;
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(event_query(&app, Some("low_stock"), None, true, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 0);
    assert!(transport.0.lock().unwrap().is_empty());
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_catches_up_a_missed_due_tick_once() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-due-catchup").await;
    let intended = now - chrono::Duration::minutes(20);
    app.fixture
        .admin
        .execute_raw(update_preference(
            &app,
            "morning_time",
            Expr::val(intended.time()),
        ))
        .await
        .unwrap();
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    use loco_rs::bgworker::BackgroundWorker;
    browser_push::scheduler::ReminderWorker::build(&app.context)
        .perform(browser_push::scheduler::ReminderRun {
            account_id: scope.actor.account_id,
            household_id: scope.household_id,
            at: now - chrono::Duration::hours(2),
        })
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    assert_eq!(transport.0.lock().unwrap().len(), 1);
    let row = app
        .fixture
        .admin
        .query_one_raw(event_query(
            &app,
            Some("dose_due"),
            Some(intended.naive_utc()),
            false,
            None,
        ))
        .await
        .unwrap()
        .unwrap();
    let saved: chrono::NaiveDateTime = row.try_get("", "intended_at").unwrap();
    let expires: chrono::NaiveDateTime = row.try_get("", "expires_at").unwrap();
    assert_eq!(saved, intended.naive_utc());
    assert_eq!(expires, (intended + chrono::Duration::hours(1)).naive_utc());
    app.close().await;
}

#[tokio::test]
async fn browser_missed_reminder_catches_up_after_grace_and_stops_at_expiry() {
    let app = Application::new().await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let scheduled = now - chrono::Duration::minutes(45);
    async {
        let query = Query::update()
            .table("accounts")
            .value(
                "preferences",
                Func::cust("jsonb_build_object").args([Expr::val("time_zone"), Expr::val("UTC")]),
            )
            .and_where(Expr::col("id").eq(Expr::val(app.fixture.account.id)))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("notification_preferences")
            .columns([
                "household_id",
                "person_id",
                "enabled",
                "dose_due_enabled",
                "missed_dose_enabled",
                "low_stock_enabled",
                "private_text_enabled",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.household.id),
                Expr::val(app.fixture.adult.id),
                Expr::val(true),
                Expr::val(false),
                Expr::val(true),
                Expr::val(false),
                Expr::val(true),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("push_subscriptions")
            .columns([
                "account_id",
                "endpoint",
                "p256dh",
                "auth",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.account.id),
                Expr::val(format!(
                    "https://fcm.googleapis.com/fcm/send/{}/missed-catchup",
                    app.fixture.client_uuid
                )),
                Expr::val("synthetic"),
                Expr::val("synthetic"),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
    }
    .await
    .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("schedules")
                .columns([
                    "household_id",
                    "person_id",
                    "medication_id",
                    "active",
                    "start_date",
                    "end_date",
                    "dose_amount",
                    "dose_unit",
                    "schedule_type",
                    "schedule_config",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(app.fixture.adult.id),
                    Expr::val(app.fixture.medication.id),
                    Expr::val(true),
                    Expr::val(now.date_naive() - chrono::Duration::days(1)),
                    Expr::val(now.date_naive() + chrono::Duration::days(1)),
                    Expr::val(2),
                    Expr::val("tablet"),
                    Expr::val(0),
                    Func::cust("jsonb_build_object")
                        .args([
                            Expr::val("times"),
                            Func::cust("jsonb_build_array")
                                .args([Expr::val(scheduled.format("%H:%M").to_string())])
                                .into(),
                        ])
                        .into(),
                    Func::cust("now").sub(Expr::val("2 days").cast_as(Alias::new("interval"))),
                    Func::cust("now").into(),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(None)]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-missed-catchup".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(event_query(&app, Some("missed_dose"), None, false, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.try_get::<chrono::NaiveDateTime>("", "intended_at")
            .unwrap(),
        (scheduled + chrono::Duration::minutes(30)).naive_utc()
    );
    assert_eq!(
        row.try_get::<chrono::NaiveDateTime>("", "expires_at")
            .unwrap(),
        (scheduled + chrono::Duration::minutes(90)).naive_utc()
    );
    let expiry = scheduled + chrono::Duration::minutes(90);
    assert_eq!(
        browser_push::reminders::drain_pending(&app.context, &scope, expiry)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "invalid");
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 1);
    assert_eq!(*transport.1.lock().unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_does_not_reserve_at_exact_expiry() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-exact-expiry").await;
    let expired = now - chrono::Duration::hours(1);
    app.fixture
        .admin
        .execute_raw(update_preference(
            &app,
            "morning_time",
            Expr::val(expired.time()),
        ))
        .await
        .unwrap();
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(event_query(&app, Some("dose_due"), None, true, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 0);
    assert!(transport.0.lock().unwrap().is_empty());
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_worker_drains_retry_when_tick_timestamp_is_stale() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-worker-retry").await;
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(None), Delivery::Accepted]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("notification_deliveries")
                .value(
                    "next_attempt_at",
                    Func::cust("now").sub(Expr::val("1 second").cast_as(Alias::new("interval"))),
                )
                .and_where(Expr::col("status").eq(Expr::val("pending")))
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .and_where(Expr::col("account_id").eq(app.fixture.account.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    use loco_rs::bgworker::BackgroundWorker;
    browser_push::scheduler::ReminderWorker::build(&app.context)
        .perform(browser_push::scheduler::ReminderRun {
            account_id: scope.actor.account_id,
            household_id: scope.household_id,
            at: now - chrono::Duration::hours(2),
        })
        .await
        .unwrap();
    assert_eq!(*transport.1.lock().unwrap(), 2);
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "accepted");
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 2);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_obeys_retry_after_and_attempt_limit() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-backoff").await;
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![
            Delivery::Temporary(Some(std::time::Duration::from_secs(600))),
            Delivery::Temporary(None),
        ]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "pending");
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 1);
    assert!(
        row.try_get::<chrono::NaiveDateTime>("", "next_attempt_at")
            .unwrap()
            >= (now + chrono::Duration::minutes(10)).naive_utc()
    );
    assert_eq!(
        browser_push::reminders::drain_pending(
            &app.context,
            &scope,
            now + chrono::Duration::minutes(9)
        )
        .await
        .unwrap(),
        0
    );
    assert_eq!(*transport.1.lock().unwrap(), 1);
    let next_attempt_at = (now - chrono::Duration::seconds(1)).naive_utc();
    app.fixture
        .admin
        .execute_raw(update_deliveries(
            &app,
            vec![
                ("attempt_count", Expr::val(4)),
                ("next_attempt_at", Expr::val(next_attempt_at)),
            ],
            Some("pending"),
        ))
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.try_get::<chrono::NaiveDateTime>("", "next_attempt_at")
            .unwrap(),
        next_attempt_at
    );
    assert_eq!(
        browser_push::reminders::drain_pending(
            &app.context,
            &scope,
            now + chrono::Duration::minutes(1)
        )
        .await
        .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "invalid");
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 5);
    assert_eq!(*transport.1.lock().unwrap(), 2);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_deadline_and_revoked_preference_stop_retries() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-deadline").await;
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(Some(
            std::time::Duration::from_secs(3600),
        ))]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "invalid");
    assert_eq!(
        row.try_get::<String>("", "outcome").unwrap(),
        "retry_deadline"
    );
    assert_eq!(
        browser_push::reminders::drain_pending(
            &app.context,
            &scope,
            now + chrono::Duration::hours(2)
        )
        .await
        .unwrap(),
        0
    );
    assert_eq!(*transport.1.lock().unwrap(), 1);
    app.close().await;

    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-revoked-preference").await;
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Temporary(None)]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let delivery = notification_deliveries::Entity::find()
        .filter(notification_deliveries::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(notification_deliveries::Column::AccountId.eq(app.fixture.account.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(delivery.status, "pending");
    assert_eq!(delivery.attempt_count, 1);
    let mut delivery = delivery.into_active_model();
    delivery.next_attempt_at = Set((now + chrono::Duration::seconds(121)).naive_utc());
    let delivery = delivery.update(&app.fixture.admin).await.unwrap();
    let retry_due = delivery.next_attempt_at.and_utc();
    assert!(retry_due > now + chrono::Duration::minutes(2));
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("notification_preferences")
                .value("enabled", Expr::val(false))
                .and_where(Expr::col("person_id").eq(Expr::val(app.fixture.adult.id)))
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::drain_pending(
            &app.context,
            &scope,
            retry_due - chrono::Duration::milliseconds(1)
        )
        .await
        .unwrap(),
        0
    );
    let delivery = notification_deliveries::Entity::find_by_id(delivery.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(delivery.status, "pending");
    assert_eq!(delivery.attempt_count, 1);
    assert_eq!(*transport.1.lock().unwrap(), 1);
    assert_eq!(
        browser_push::reminders::drain_pending(&app.context, &scope, retry_due)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "invalid");
    assert_eq!(
        row.try_get::<String>("", "outcome").unwrap(),
        "no_longer_eligible"
    );
    assert_eq!(*transport.1.lock().unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_claim_audit_failure_rolls_back_before_transport() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-claim-audit").await;
    let fault = format!(
        "reject_browser_push_sending_audit_{}",
        uuid::Uuid::new_v4().simple()
    );
    // SeaORM has no trigger, PL/pgSQL function or GRANT/REVOKE builder.
    let household_id = app.fixture.household.id;
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION public.{fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.household_id={household_id} AND NEW.event_type='browser_push.reminder.sending' THEN RAISE EXCEPTION 'synthetic sending audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER {fault} BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION public.{fault}()")).await.unwrap();
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Accepted]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .is_err()
    );
    assert_eq!(*transport.1.lock().unwrap(), 0);
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "pending");
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 0);
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {fault} ON security_audit_events; DROP FUNCTION public.{fault}()"
        ))
        .await
        .unwrap();
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_uncertain_transport_result_is_never_retried() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-uncertain-send").await;
    let transport = Arc::new(ScriptedDelivery(
        Mutex::new(vec![Delivery::Uncertain]),
        Mutex::new(0),
    ));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        browser_push::reminders::drain_pending(
            &app.context,
            &scope,
            now + chrono::Duration::minutes(10)
        )
        .await
        .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "uncertain");
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 1);
    assert_eq!(*transport.1.lock().unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_stale_accepted_result_cannot_overwrite_uncertain_recovery() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-stale-result").await;
    let transport = Arc::new(BlockedAcceptance {
        started: Notify::new(),
        release: Notify::new(),
    });
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let ctx = app.context.clone();
    let sending_scope = scope.clone();
    let sender = tokio::spawn(async move {
        browser_push::reminders::deliver_account(&ctx, &sending_scope, now).await
    });
    transport.started.notified().await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("notification_deliveries")
                .value(
                    "claimed_at",
                    Func::cust("now").sub(Expr::val("121 seconds").cast_as(Alias::new("interval"))),
                )
                .and_where(Expr::col("status").eq(Expr::val("sending")))
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .and_where(Expr::col("account_id").eq(app.fixture.account.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::drain_pending(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    transport.release.notify_one();
    assert_eq!(sender.await.unwrap().unwrap(), 0);
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "uncertain");
    assert!(
        row.try_get::<Option<chrono::NaiveDateTime>>("", "sent_at")
            .unwrap()
            .is_none()
    );
    app.close().await;
}

#[async_trait::async_trait]
impl Transport for PartialDelivery {
    async fn send(&self, subscription: &push_subscription::Model, _: &[u8]) -> Delivery {
        self.0.lock().unwrap().push(subscription.id);
        if subscription.endpoint.ends_with("/accepted") {
            Delivery::Accepted
        } else {
            Delivery::Temporary(None)
        }
    }
}

#[tokio::test]
async fn browser_reminder_keeps_independent_device_outcomes() {
    let app = Application::new().await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap()
        - chrono::Duration::minutes(1);
    async {
        let query = Query::update()
            .table("accounts")
            .value(
                "preferences",
                Func::cust("jsonb_build_object").args([Expr::val("time_zone"), Expr::val("UTC")]),
            )
            .and_where(Expr::col("id").eq(Expr::val(app.fixture.account.id)))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::update()
            .table("person_medications")
            .value("administration_kind", Expr::val(0))
            .value("dose_cycle", Expr::val(0))
            .value("max_daily_doses", Expr::val(1))
            .value(
                "created_at",
                Func::cust("now").sub(Expr::val("2 days").cast_as(Alias::new("interval"))),
            )
            .and_where(Expr::col("id").eq(Expr::val(app.fixture.assignment.id)))
            .and_where(Expr::col("household_id").eq(app.fixture.household.id))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("push_subscriptions")
            .columns([
                "account_id",
                "endpoint",
                "p256dh",
                "auth",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.account.id),
                Expr::val(format!(
                    "https://fcm.googleapis.com/fcm/send/{}/accepted",
                    app.fixture.client_uuid
                )),
                Expr::val("synthetic"),
                Expr::val("synthetic"),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .values_panic([
                Expr::val(app.fixture.account.id),
                Expr::val(format!(
                    "https://fcm.googleapis.com/fcm/send/{}/failed",
                    app.fixture.client_uuid
                )),
                Expr::val("synthetic"),
                Expr::val("synthetic"),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
    }
    .await
    .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("notification_preferences")
                .columns([
                    "household_id",
                    "person_id",
                    "enabled",
                    "dose_due_enabled",
                    "missed_dose_enabled",
                    "low_stock_enabled",
                    "private_text_enabled",
                    "morning_time",
                    "afternoon_time",
                    "evening_time",
                    "night_time",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(app.fixture.adult.id),
                    Expr::val(true),
                    Expr::val(true),
                    Expr::val(false),
                    Expr::val(false),
                    Expr::val(true),
                    Expr::val(now.time()),
                    Expr::val(Option::<chrono::NaiveTime>::None),
                    Expr::val(Option::<chrono::NaiveTime>::None),
                    Expr::val(Option::<chrono::NaiveTime>::None),
                    Func::cust("now").into(),
                    Func::cust("now").into(),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let transport = Arc::new(PartialDelivery(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-two-devices".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        1
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_counts(&app))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "accepted").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "retryable").unwrap(), 1);
    assert_eq!(transport.0.lock().unwrap().len(), 2);
    let pending = notification_deliveries::Entity::find()
        .filter(notification_deliveries::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(notification_deliveries::Column::AccountId.eq(app.fixture.account.id))
        .filter(notification_deliveries::Column::Status.eq("pending"))
        .filter(notification_deliveries::Column::AttemptCount.eq(1))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let retry_at = pending.next_attempt_at.and_utc();
    assert!(retry_at > now + chrono::Duration::minutes(2));
    app.fixture
        .admin
        .execute_raw({
            let query = Query::delete()
                .from_table("push_subscriptions")
                .and_where(Expr::col("account_id").eq(Expr::val(app.fixture.account.id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::drain_pending(
            &app.context,
            &scope,
            retry_at + chrono::Duration::seconds(1)
        )
        .await
        .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_counts(&app))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "accepted").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "invalid").unwrap(), 1);
    assert_eq!(transport.0.lock().unwrap().len(), 2);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_recovers_stale_claim_as_uncertain_without_resending() {
    let app = Application::new().await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    async {
        let query = Query::update()
            .table("accounts")
            .value(
                "preferences",
                Func::cust("jsonb_build_object").args([Expr::val("time_zone"), Expr::val("UTC")]),
            )
            .and_where(Expr::col("id").eq(Expr::val(app.fixture.account.id)))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::update()
            .table("person_medications")
            .value("administration_kind", Expr::val(0))
            .value("dose_cycle", Expr::val(0))
            .value("max_daily_doses", Expr::val(1))
            .value(
                "created_at",
                Func::cust("now").sub(Expr::val("2 days").cast_as(Alias::new("interval"))),
            )
            .and_where(Expr::col("id").eq(Expr::val(app.fixture.assignment.id)))
            .and_where(Expr::col("household_id").eq(app.fixture.household.id))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("push_subscriptions")
            .columns([
                "account_id",
                "endpoint",
                "p256dh",
                "auth",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.account.id),
                Expr::val(format!(
                    "https://fcm.googleapis.com/fcm/send/{}/failed",
                    app.fixture.client_uuid
                )),
                Expr::val("synthetic"),
                Expr::val("synthetic"),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
    }
    .await
    .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("notification_preferences")
                .columns([
                    "household_id",
                    "person_id",
                    "enabled",
                    "dose_due_enabled",
                    "missed_dose_enabled",
                    "low_stock_enabled",
                    "private_text_enabled",
                    "morning_time",
                    "afternoon_time",
                    "evening_time",
                    "night_time",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(app.fixture.adult.id),
                    Expr::val(true),
                    Expr::val(true),
                    Expr::val(false),
                    Expr::val(false),
                    Expr::val(true),
                    Expr::val(now.time()),
                    Expr::val(Option::<chrono::NaiveTime>::None),
                    Expr::val(Option::<chrono::NaiveTime>::None),
                    Expr::val(Option::<chrono::NaiveTime>::None),
                    Func::cust("now").into(),
                    Func::cust("now").into(),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let transport = Arc::new(PartialDelivery(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-recover-claim".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("notification_deliveries")
                .value("status", Expr::val("sending"))
                .value(
                    "claimed_at",
                    Func::cust("now").sub(Expr::val("119 seconds").cast_as(Alias::new("interval"))),
                )
                .and_where(Expr::col("status").eq(Expr::val("pending")))
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .and_where(Expr::col("account_id").eq(app.fixture.account.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::drain_pending(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "sending");
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("notification_deliveries")
                .value(
                    "claimed_at",
                    Func::cust("now").sub(Expr::val("121 seconds").cast_as(Alias::new("interval"))),
                )
                .and_where(Expr::col("status").eq(Expr::val("sending")))
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .and_where(Expr::col("account_id").eq(app.fixture.account.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::drain_pending(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        browser_push::reminders::drain_pending(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "uncertain");
    assert_eq!(row.try_get::<i32>("", "attempt_count").unwrap(), 1);
    assert_eq!(transport.0.lock().unwrap().len(), 1);
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_retries_a_pending_reservation_once_after_database_failure() {
    let app = Application::new().await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    async {
        let query = Query::update()
            .table("accounts")
            .value(
                "preferences",
                Func::cust("jsonb_build_object").args([Expr::val("time_zone"), Expr::val("UTC")]),
            )
            .and_where(Expr::col("id").eq(Expr::val(app.fixture.account.id)))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::update()
            .table("person_medications")
            .value("administration_kind", Expr::val(0))
            .value("dose_cycle", Expr::val(0))
            .value("max_daily_doses", Expr::val(1))
            .value(
                "created_at",
                Func::cust("now").sub(Expr::val("2 days").cast_as(Alias::new("interval"))),
            )
            .and_where(Expr::col("id").eq(Expr::val(app.fixture.assignment.id)))
            .and_where(Expr::col("household_id").eq(app.fixture.household.id))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("push_subscriptions")
            .columns([
                "account_id",
                "endpoint",
                "p256dh",
                "auth",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.account.id),
                Expr::val(format!(
                    "https://fcm.googleapis.com/fcm/send/{}/retry",
                    app.fixture.client_uuid
                )),
                Expr::val("synthetic"),
                Expr::val("synthetic"),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
    }
    .await
    .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("notification_preferences")
                .columns([
                    "household_id",
                    "person_id",
                    "enabled",
                    "dose_due_enabled",
                    "missed_dose_enabled",
                    "low_stock_enabled",
                    "private_text_enabled",
                    "morning_time",
                    "afternoon_time",
                    "evening_time",
                    "night_time",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(app.fixture.adult.id),
                    Expr::val(true),
                    Expr::val(true),
                    Expr::val(false),
                    Expr::val(false),
                    Expr::val(true),
                    Expr::val(now.time()),
                    Expr::val(Option::<chrono::NaiveTime>::None),
                    Expr::val(Option::<chrono::NaiveTime>::None),
                    Expr::val(Option::<chrono::NaiveTime>::None),
                    Func::cust("now").into(),
                    Func::cust("now").into(),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let fault = format!(
        "fail_reminder_delivery_read_{}",
        uuid::Uuid::new_v4().simple()
    );
    // SeaORM has no trigger, PL/pgSQL function or GRANT/REVOKE builder.
    let household_id = app.fixture.household.id;
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION {fault}() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$ BEGIN REVOKE SELECT ON public.push_subscriptions FROM med_tracker_app; RETURN NEW; END $$; CREATE TRIGGER {fault} AFTER INSERT ON security_audit_events FOR EACH ROW WHEN (NEW.household_id={household_id} AND NEW.event_type='browser_push.reminder.requested') EXECUTE FUNCTION {fault}()")).await.unwrap();
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-reminder-retry".into(),
    };
    assert!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .is_err()
    );
    assert!(transport.0.lock().unwrap().is_empty());
    let row = app
        .fixture
        .admin
        .query_one_raw(event_query(
            &app,
            Some("dose_due"),
            None,
            true,
            Some("pending"),
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    app.fixture.admin.execute_unprepared(&format!("DROP TRIGGER {fault} ON security_audit_events; DROP FUNCTION {fault}(); GRANT SELECT ON public.push_subscriptions TO med_tracker_app")).await.unwrap();
    app.fixture
        .admin
        .execute_raw({
            let selection = Query::select()
                .columns(["household_id", "person_id", "event_type"])
                .expr(Func::cust("replace").args([
                    Expr::col("event_key"),
                    Expr::val(format!("browser:{}:", app.fixture.account.id)),
                    Expr::val(""),
                ]))
                .expr(Expr::val(json!({})))
                .expr(Func::cust("now"))
                .expr(Func::cust("now"))
                .from("notification_events")
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .and_where(Expr::col("event_type").eq("dose_due"))
                .to_owned();
            let query = Query::insert()
                .into_table("notification_events")
                .columns([
                    "household_id",
                    "person_id",
                    "event_type",
                    "event_key",
                    "metadata",
                    "created_at",
                    "updated_at",
                ])
                .select_from(selection)
                .unwrap()
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    assert!(transport.0.lock().unwrap().is_empty());
    app.fixture
        .admin
        .execute_raw({
            let query = Query::delete()
                .from_table("notification_events")
                .and_where(
                    Expr::col("event_type")
                        .eq(Expr::val("dose_due"))
                        .and(Expr::col("event_key").like("dose-due:%")),
                )
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let (first, second) = tokio::join!(
        browser_push::reminders::deliver_account(&app.context, &scope, now),
        browser_push::reminders::deliver_account(&app.context, &scope, now)
    );
    assert_eq!(first.unwrap() + second.unwrap(), 1);
    assert_eq!(transport.0.lock().unwrap().len(), 1);
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(event_query(
            &app,
            Some("dose_due"),
            None,
            true,
            Some("accepted"),
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    for event in [
        "browser_push.reminder.requested",
        "browser_push.reminder.completed",
    ] {
        let row = app
            .fixture
            .admin
            .query_one_raw(audit_query(&app, event, None))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    }
    app.close().await;
}

#[tokio::test]
async fn browser_low_stock_reminder_follows_committed_threshold_crossing_once() {
    let app = Application::new().await;
    let household_id = app.fixture.household.id;
    async {
        let query = Query::update()
            .table("medications")
            .value("current_supply", Expr::val(11))
            .value("reorder_threshold", Expr::val(10))
            .and_where(Expr::col("id").eq(Expr::val(app.fixture.medication.id)))
            .and_where(Expr::col("household_id").eq(app.fixture.household.id))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("notification_preferences")
            .columns([
                "household_id",
                "person_id",
                "enabled",
                "dose_due_enabled",
                "missed_dose_enabled",
                "low_stock_enabled",
                "private_text_enabled",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.household.id),
                Expr::val(app.fixture.adult.id),
                Expr::val(true),
                Expr::val(false),
                Expr::val(false),
                Expr::val(true),
                Expr::val(false),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("push_subscriptions")
            .columns([
                "account_id",
                "endpoint",
                "p256dh",
                "auth",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.account.id),
                Expr::val(format!(
                    "https://fcm.googleapis.com/fcm/send/{}/stock-reminder",
                    app.fixture.client_uuid
                )),
                Expr::val("synthetic"),
                Expr::val("synthetic"),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
    }
    .await
    .unwrap();
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-stock-reminder".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/medication_takes",
        app.origin
    );
    let fault = format!("reject_stock_take_audit_{}", uuid::Uuid::new_v4().simple());
    // SeaORM has no trigger, PL/pgSQL function or GRANT/REVOKE builder.
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION {fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.household_id={household_id} AND NEW.item_type='MedicationTake' THEN RAISE EXCEPTION 'synthetic take audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER {fault} BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION {fault}()")).await.unwrap();
    let rejected = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&shared_take_body(&app.fixture))
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status().as_u16(), 500);
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {fault} ON versions; DROP FUNCTION {fault}()"
        ))
        .await
        .unwrap();
    let response = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&shared_take_body(&app.fixture))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 201);
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("notification_preferences")
                .value("low_stock_enabled", Expr::val(false))
                .and_where(Expr::col("person_id").eq(Expr::val(app.fixture.adult.id)))
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("notification_preferences")
                .value("low_stock_enabled", Expr::val(true))
                .and_where(Expr::col("person_id").eq(Expr::val(app.fixture.adult.id)))
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, Utc::now())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        transport.0.lock().unwrap()[0],
        json!({"title":"Stock reminder","body":"A medication may be running low.","path":format!("/households/{}/dashboard",app.fixture.household.slug),"account_id":app.fixture.account.id.to_string()})
    );
    app.close().await;
}

#[test]
fn browser_reminder_scheduler_task_is_registered() {
    use loco_rs::app::Hooks;
    let mut tasks = loco_rs::task::Tasks::default();
    med_tracker::app::App::register_tasks(&mut tasks);
    assert!(tasks.names().contains(&"browser-reminders".to_owned()));
}

#[async_trait::async_trait]
impl Transport for Reminders {
    async fn send(&self, _: &push_subscription::Model, payload: &[u8]) -> Delivery {
        self.0
            .lock()
            .unwrap()
            .push(serde_json::from_slice(payload).unwrap());
        Delivery::Accepted
    }
}

#[tokio::test]
async fn browser_reminders_use_saved_times_private_content_and_deduplicate_delivery() {
    reminders_use_saved_zone("UTC", chrono_tz::UTC).await;
}

#[tokio::test]
async fn browser_reminders_accept_retained_rails_timezone_preferences() {
    reminders_use_saved_zone("Eastern Time (US & Canada)", chrono_tz::America::New_York).await;
}

async fn reminders_use_saved_zone(saved_zone: &str, zone: chrono_tz::Tz) {
    let app = Application::new().await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("accounts")
                .value("preferences", json!({"time_zone":saved_zone}))
                .and_where(Expr::col("id").eq(app.fixture.account.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("households")
                .value("timezone", Expr::val("Pacific/Auckland"))
                .and_where(Expr::col("id").eq(Expr::val(app.fixture.household.id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("notification_preferences")
                .columns([
                    "household_id",
                    "person_id",
                    "enabled",
                    "dose_due_enabled",
                    "missed_dose_enabled",
                    "low_stock_enabled",
                    "private_text_enabled",
                    "morning_time",
                    "afternoon_time",
                    "evening_time",
                    "night_time",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(app.fixture.adult.id),
                    Expr::val(true),
                    Expr::val(true),
                    Expr::val(false),
                    Expr::val(false),
                    Expr::val(true),
                    Expr::val(now.with_timezone(&zone).time()),
                    Expr::val(Option::<chrono::NaiveTime>::None),
                    Expr::val(Option::<chrono::NaiveTime>::None),
                    Expr::val(Option::<chrono::NaiveTime>::None),
                    Func::cust("now").into(),
                    Func::cust("now").into(),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    async {
        let query = Query::update()
            .table("person_medications")
            .value("administration_kind", Expr::val(0))
            .value("dose_cycle", Expr::val(0))
            .value("max_daily_doses", Expr::val(1))
            .value(
                "created_at",
                Func::cust("now").sub(Expr::val("2 days").cast_as(Alias::new("interval"))),
            )
            .and_where(Expr::col("id").eq(Expr::val(app.fixture.assignment.id)))
            .and_where(Expr::col("household_id").eq(app.fixture.household.id))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("push_subscriptions")
            .columns([
                "account_id",
                "endpoint",
                "p256dh",
                "auth",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.account.id),
                Expr::val(format!(
                    "https://fcm.googleapis.com/fcm/send/{}/reminder",
                    app.fixture.client_uuid
                )),
                Expr::val("synthetic"),
                Expr::val("synthetic"),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
    }
    .await
    .unwrap();
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-due-reminder".into(),
    };
    let runs = own_recipients(&app, now).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].account_id, scope.actor.account_id);
    assert_eq!(runs[0].household_id, scope.household_id);
    use loco_rs::bgworker::BackgroundWorker;
    browser_push::scheduler::ReminderWorker::build(&app.context)
        .perform(runs[0].clone())
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        transport.0.lock().unwrap().as_slice(),
        &[
            json!({"title":"Medication reminder","body":"A dose is due.","path":format!("/households/{}/dashboard",app.fixture.household.slug),"account_id":app.fixture.account.id.to_string()})
        ]
    );
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("notification_preferences")
                .value("enabled", Expr::val(false))
                .and_where(Expr::col("person_id").eq(Expr::val(app.fixture.adult.id)))
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::deliver_account(
            &app.context,
            &scope,
            now + chrono::Duration::days(1)
        )
        .await
        .unwrap(),
        0
    );
    assert_eq!(transport.0.lock().unwrap().len(), 1);
    app.close().await;
}

#[tokio::test]
async fn browser_missed_reminders_respect_grace_managed_opt_in_and_private_content() {
    let app = Application::new().await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let scheduled = (now - chrono::Duration::minutes(30))
        .format("%H:%M")
        .to_string();
    async {
        let query = Query::update()
            .table("accounts")
            .value(
                "preferences",
                Func::cust("jsonb_build_object").args([Expr::val("time_zone"), Expr::val("UTC")]),
            )
            .and_where(Expr::col("id").eq(Expr::val(app.fixture.account.id)))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("people")
            .columns([
                "name",
                "person_type",
                "has_capacity",
                "created_at",
                "updated_at",
                "portable_id",
            ])
            .values_panic([
                Expr::val("Managed adult"),
                Expr::val(0),
                Expr::val(true),
                Func::cust("now").into(),
                Func::cust("now").into(),
                Expr::val(format!("{}-{}", app.fixture.client_uuid, "Managed adult")),
            ])
            .values_panic([
                Expr::val("View-only adult"),
                Expr::val(0),
                Expr::val(true),
                Func::cust("now").into(),
                Func::cust("now").into(),
                Expr::val(format!("{}-{}", app.fixture.client_uuid, "View-only adult")),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
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
                Expr::val(app.fixture.household.id),
                Expr::val(participant_named(&app, "Managed adult").await.id),
                Expr::val("member"),
                Expr::val("active"),
                Func::cust("now").into(),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .values_panic([
                Expr::val(app.fixture.household.id),
                Expr::val(participant_named(&app, "View-only adult").await.id),
                Expr::val("member"),
                Expr::val("active"),
                Func::cust("now").into(),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("person_access_grants")
            .columns([
                "household_id",
                "household_membership_id",
                "person_id",
                "access_level",
                "relationship_type",
                "missed_dose_notifications_enabled",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.household.id),
                Expr::val(app.fixture.membership.id),
                Expr::val(app.fixture.minor.id),
                Expr::val("manage"),
                Expr::val("parent"),
                Expr::val(false),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .values_panic([
                Expr::val(app.fixture.household.id),
                Expr::val(app.fixture.membership.id),
                Expr::val(participant_named(&app, "Managed adult").await.id),
                Expr::val("manage"),
                Expr::val("carer"),
                Expr::val(false),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .values_panic([
                Expr::val(app.fixture.household.id),
                Expr::val(app.fixture.membership.id),
                Expr::val(participant_named(&app, "View-only adult").await.id),
                Expr::val("view"),
                Expr::val("carer"),
                Expr::val(true),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("notification_preferences")
            .columns([
                "household_id",
                "person_id",
                "enabled",
                "dose_due_enabled",
                "missed_dose_enabled",
                "low_stock_enabled",
                "private_text_enabled",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.household.id),
                Expr::val(app.fixture.adult.id),
                Expr::val(true),
                Expr::val(false),
                Expr::val(true),
                Expr::val(false),
                Expr::val(false),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("push_subscriptions")
            .columns([
                "account_id",
                "endpoint",
                "p256dh",
                "auth",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.account.id),
                Expr::val(format!(
                    "https://fcm.googleapis.com/fcm/send/{}/managed-reminder",
                    app.fixture.client_uuid
                )),
                Expr::val("synthetic"),
                Expr::val("synthetic"),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
    }
    .await
    .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let selection = Query::select()
                .expr(Expr::val(app.fixture.household.id))
                .column("id")
                .expr(Expr::val(app.fixture.medication.id))
                .expr(Expr::val(true))
                .expr(Expr::val(now.date_naive() - chrono::Duration::days(1)))
                .expr(Expr::val(now.date_naive() + chrono::Duration::days(2)))
                .expr(Expr::val(2))
                .expr(Expr::val("tablet"))
                .expr(Expr::val(0))
                .expr(Expr::val(json!({"times":[scheduled]})))
                .expr(Func::cust("now"))
                .expr(Func::cust("now"))
                .from("people")
                .and_where(Expr::col("id").is_in([
                    app.fixture.minor.id,
                    participant_named(&app, "Managed adult").await.id,
                    participant_named(&app, "View-only adult").await.id,
                ]))
                .to_owned();
            let query = Query::insert()
                .into_table("schedules")
                .columns([
                    "household_id",
                    "person_id",
                    "medication_id",
                    "active",
                    "start_date",
                    "end_date",
                    "dose_amount",
                    "dose_unit",
                    "schedule_type",
                    "schedule_config",
                    "created_at",
                    "updated_at",
                ])
                .select_from(selection)
                .unwrap()
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-missed-reminder".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(
            &app.context,
            &scope,
            now - chrono::Duration::minutes(1)
        )
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        transport.0.lock().unwrap()[0]["body"],
        "Synthetic minor may have missed a dose."
    );
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("person_access_grants")
                .value("missed_dose_notifications_enabled", Expr::val(true))
                .and_where(
                    Expr::col("id").eq(Expr::val(
                        grant_for(&app, participant_named(&app, "Managed adult").await.id)
                            .await
                            .id,
                    )),
                )
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        transport.0.lock().unwrap()[1]["body"],
        "Managed adult may have missed a dose."
    );
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("person_access_grants")
                .value("revoked_at", Func::cust("now"))
                .and_where(
                    Expr::col("id").is_in([
                        Expr::val(grant_for(&app, app.fixture.minor.id).await.id),
                        Expr::val(
                            grant_for(&app, participant_named(&app, "Managed adult").await.id)
                                .await
                                .id,
                        ),
                    ]),
                )
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    assert_eq!(
        browser_push::reminders::deliver_account(
            &app.context,
            &scope,
            now + chrono::Duration::days(2)
        )
        .await
        .unwrap(),
        0
    );
    app.close().await;
}

async fn prepare_managed_dependent_dose(
    app: &Application,
    request_id: &str,
) -> (chrono::DateTime<Utc>, HouseholdScope) {
    let (now, scope) = prepare_due_delivery(app, request_id).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("person_access_grants")
                .columns([
                    "household_id",
                    "household_membership_id",
                    "person_id",
                    "access_level",
                    "relationship_type",
                    "missed_dose_notifications_enabled",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(app.fixture.membership.id),
                    Expr::val(app.fixture.minor.id),
                    Expr::val("manage"),
                    Expr::val("parent"),
                    Expr::val(false),
                    Func::cust("now").into(),
                    Func::cust("now").into(),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("schedules")
                .columns([
                    "household_id",
                    "person_id",
                    "medication_id",
                    "active",
                    "start_date",
                    "end_date",
                    "dose_amount",
                    "dose_unit",
                    "schedule_type",
                    "schedule_config",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(app.fixture.minor.id),
                    Expr::val(app.fixture.medication.id),
                    Expr::val(true),
                    Expr::val(now.date_naive() - chrono::Duration::days(1)),
                    Expr::val(now.date_naive() + chrono::Duration::days(2)),
                    Expr::val(2),
                    Expr::val("tablet"),
                    Expr::val(0),
                    Func::cust("jsonb_build_object")
                        .args([
                            Expr::val("times"),
                            Func::cust("jsonb_build_array")
                                .args([Expr::val(now.format("%H:%M").to_string())])
                                .into(),
                        ])
                        .into(),
                    Func::cust("now").into(),
                    Func::cust("now").into(),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    (now, scope)
}

#[tokio::test]
async fn browser_dose_due_reaches_managed_dependent_subjects() {
    let app = Application::new().await;
    let (now, scope) = prepare_managed_dependent_dose(&app, "synthetic-due-managed").await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("notification_preferences")
                .value("private_text_enabled", Expr::val(false))
                .and_where(
                    Expr::col("household_id")
                        .eq(Expr::val(app.fixture.household.id))
                        .and(Expr::col("person_id").eq(Expr::val(app.fixture.adult.id))),
                )
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        2
    );
    {
        let bodies = transport.0.lock().unwrap();
        let bodies = bodies
            .iter()
            .map(|value| value["body"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        let expected_self = format!("{} medications: Synthetic tablets", now.format("%H:%M"));
        assert!(
            bodies.contains(&expected_self),
            "self body missing: {bodies:?}"
        );
        assert!(
            bodies
                .iter()
                .any(|body| body.starts_with("Synthetic minor")),
            "dependent body missing: {bodies:?}"
        );
    }
    let events = app
        .fixture
        .admin
        .query_all_raw(event_query(&app, Some("dose_due"), None, false, None))
        .await
        .unwrap();
    let person_ids = events
        .iter()
        .map(|row| row.try_get::<i64>("", "person_id").unwrap())
        .collect::<Vec<_>>();
    assert_eq!(person_ids, vec![app.fixture.adult.id, app.fixture.minor.id]);
    let keys = events
        .iter()
        .map(|row| row.try_get::<String>("", "event_key").unwrap())
        .collect::<Vec<_>>();
    assert!(keys[0].contains(&format!("dose-due:{}:", app.fixture.adult.id)));
    assert!(keys[1].contains(&format!("dose-due:{}:", app.fixture.minor.id)));
    app.close().await;
}

#[tokio::test]
async fn browser_dose_due_keeps_managed_dependent_names_private() {
    let app = Application::new().await;
    let (now, scope) = prepare_managed_dependent_dose(&app, "synthetic-due-private").await;
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        2
    );
    {
        let bodies = transport.0.lock().unwrap();
        assert_eq!(bodies.len(), 2);
        assert!(
            bodies
                .iter()
                .all(|value| value["body"].as_str().unwrap() == "A dose is due."),
            "private bodies leaked names: {bodies:?}"
        );
    }
    app.close().await;
}

#[tokio::test]
async fn browser_dose_due_skips_decided_and_unmanaged_dependents() {
    let app = Application::new().await;
    let (now, scope) = prepare_managed_dependent_dose(&app, "synthetic-due-decided").await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("medication_dose_occurrences")
                .columns([
                    "household_id",
                    "schedule_id",
                    "window_starts_on",
                    "window_ends_on",
                    "position",
                    "scheduled_at",
                    "outcome",
                    "resolved_at",
                    "resolved_by_membership_id",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(schedule_for(&app, app.fixture.minor.id).await.id),
                    Expr::val(now.date_naive()),
                    Expr::val(now.date_naive()),
                    Expr::val(1),
                    Expr::val(now.naive_utc()),
                    Expr::val("not_taken"),
                    Func::cust("now").into(),
                    Expr::val(app.fixture.membership.id),
                    Func::cust("now").into(),
                    Func::cust("now").into(),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    async {
        let query = Query::insert()
            .into_table("people")
            .columns([
                "name",
                "person_type",
                "has_capacity",
                "created_at",
                "updated_at",
                "portable_id",
            ])
            .values_panic([
                Expr::val("Synthetic unmanaged adult"),
                Expr::val(0),
                Expr::val(true),
                Func::cust("now").into(),
                Func::cust("now").into(),
                Expr::val(format!(
                    "{}-{}",
                    app.fixture.client_uuid, "Synthetic unmanaged adult"
                )),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
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
                Expr::val(app.fixture.household.id),
                Expr::val(
                    participant_named(&app, "Synthetic unmanaged adult")
                        .await
                        .id,
                ),
                Expr::val("member"),
                Expr::val("active"),
                Func::cust("now").into(),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("person_access_grants")
            .columns([
                "household_id",
                "household_membership_id",
                "person_id",
                "access_level",
                "relationship_type",
                "missed_dose_notifications_enabled",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.household.id),
                Expr::val(app.fixture.membership.id),
                Expr::val(
                    participant_named(&app, "Synthetic unmanaged adult")
                        .await
                        .id,
                ),
                Expr::val("manage"),
                Expr::val("carer"),
                Expr::val(false),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
    }
    .await
    .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("schedules")
                .columns([
                    "household_id",
                    "person_id",
                    "medication_id",
                    "active",
                    "start_date",
                    "end_date",
                    "dose_amount",
                    "dose_unit",
                    "schedule_type",
                    "schedule_config",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(
                        participant_named(&app, "Synthetic unmanaged adult")
                            .await
                            .id,
                    ),
                    Expr::val(app.fixture.medication.id),
                    Expr::val(true),
                    Expr::val(now.date_naive() - chrono::Duration::days(1)),
                    Expr::val(now.date_naive() + chrono::Duration::days(2)),
                    Expr::val(2),
                    Expr::val("tablet"),
                    Expr::val(0),
                    Func::cust("jsonb_build_object")
                        .args([
                            Expr::val("times"),
                            Func::cust("jsonb_build_array")
                                .args([Expr::val(now.format("%H:%M").to_string())])
                                .into(),
                        ])
                        .into(),
                    Func::cust("now").into(),
                    Func::cust("now").into(),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        1
    );
    let events = app
        .fixture
        .admin
        .query_all_raw(event_query(&app, Some("dose_due"), None, false, None))
        .await
        .unwrap();
    let person_ids = events
        .iter()
        .map(|row| row.try_get::<i64>("", "person_id").unwrap())
        .collect::<Vec<_>>();
    assert_eq!(person_ids, vec![app.fixture.adult.id]);
    app.close().await;
}

#[tokio::test]
async fn browser_dose_due_dependents_dedupe_across_reruns() {
    let app = Application::new().await;
    let (now, scope) = prepare_managed_dependent_dose(&app, "synthetic-due-dedupe").await;
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(event_query(&app, Some("dose_due"), None, true, None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "total").unwrap(), 2);
    assert_eq!(row.try_get::<i64>("", "keys").unwrap(), 2);
    app.close().await;
}

async fn owner_connection(app: &Application) -> sea_orm::DatabaseConnection {
    app.fixture.admin.clone()
}

async fn seed_claimable_delivery(
    app: &Application,
    scope: &HouseholdScope,
    now: chrono::DateTime<Utc>,
) -> i64 {
    let key = format!(
        "browser:{}:dose-due:{}:{}",
        scope.actor.account_id,
        app.fixture.adult.id,
        now.format("%Y-%m-%dT%H:%M:%SZ")
    );
    let event = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::insert()
                .into_table("notification_events")
                .columns([
                    "household_id",
                    "person_id",
                    "event_type",
                    "event_key",
                    "metadata",
                    "intended_at",
                    "expires_at",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(app.fixture.adult.id),
                    Expr::val("dose_due"),
                    Expr::val(key),
                    Func::cust("jsonb_build_object")
                        .args([
                            Expr::val("account_id"),
                            Expr::val(scope.actor.account_id),
                            Expr::val("delivery_status"),
                            Expr::val("pending"),
                        ])
                        .into(),
                    Expr::val(now.naive_utc()),
                    Expr::val((now + chrono::Duration::hours(1)).naive_utc()),
                    Func::cust("now").into(),
                    Func::cust("now").into(),
                ])
                .returning(Query::returning().column("id").to_owned())
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let event_id: i64 = event.try_get("", "id").unwrap();
    let delivery = app
        .fixture
        .admin
        .query_one_raw({
            let selection = Query::select()
                .expr(Expr::val(event_id))
                .column("id")
                .expr(Expr::val(app.fixture.household.id))
                .expr(Expr::val(scope.actor.account_id))
                .expr(Expr::val(now.naive_utc()))
                .from("push_subscriptions")
                .and_where(Expr::col("account_id").eq(scope.actor.account_id))
                .limit(1)
                .to_owned();
            let query = Query::insert()
                .into_table("notification_deliveries")
                .columns([
                    "notification_event_id",
                    "subscription_id",
                    "household_id",
                    "account_id",
                    "next_attempt_at",
                ])
                .select_from(selection)
                .unwrap()
                .returning(Query::returning().column("id").to_owned())
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    delivery.try_get("", "id").unwrap()
}

#[tokio::test]
async fn browser_reminder_authority_revoked_before_send_is_suppressed() {
    use med_tracker::models::access;
    for (mutation, expected) in [
        (AuthorityChange::Preference, "no_longer_eligible"),
        (AuthorityChange::Subscription, "no_longer_eligible"),
        (AuthorityChange::Membership, "recipient_revoked"),
    ] {
        let app = Application::new().await;
        let (now, scope) = prepare_due_delivery(&app, "synthetic-presend-revoke").await;
        let transport = Arc::new(ScriptedDelivery(
            Mutex::new(vec![Delivery::Accepted]),
            Mutex::new(0),
        ));
        app.context.shared_store.insert(Service {
            public_key: String::new(),
            transport: transport.clone(),
        });
        let delivery_id = seed_claimable_delivery(&app, &scope, now).await;
        let tenant = access::begin(&app.context.db, &scope, None).await.unwrap();
        let holder = backend_pid(tenant.transaction()).await;
        tenant
            .transaction()
            .execute_raw({
                let query = Query::select()
                    .column("id")
                    .from("push_subscriptions")
                    .and_where(Expr::col("account_id").eq(app.fixture.account.id))
                    .lock(sea_orm::sea_query::LockType::Update)
                    .to_owned();
                app.fixture.admin.get_database_backend().build(&query)
            })
            .await
            .unwrap();
        let ctx = app.context.clone();
        let sending = scope.clone();
        let sender = tokio::spawn(async move {
            browser_push::reminders::drain_candidates(&ctx, &sending, now, &[delivery_id]).await
        });
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let row = app
                    .fixture
                    .admin
                    .query_one_raw(blocking_query(
                        &app,
                        Some("push_subscriptions"),
                        Some(holder),
                        None,
                    ))
                    .await
                    .unwrap()
                    .unwrap();
                if row.try_get::<bool>("", "waiting").unwrap() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        tenant
            .transaction()
            .execute_raw(authority_query(&app, mutation))
            .await
            .unwrap();
        tenant.commit().await.unwrap();
        let sent = tokio::time::timeout(std::time::Duration::from_secs(10), sender)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(sent.unwrap(), 0);
        let row = app
            .fixture
            .admin
            .query_one_raw(delivery_query(&app, Some(delivery_id)))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<String>("", "status").unwrap(), "invalid");
        assert_eq!(
            row.try_get::<Option<String>>("", "outcome")
                .unwrap()
                .as_deref(),
            Some(expected)
        );
        let audited = app
            .fixture
            .admin
            .query_one_raw(audit_query(
                &app,
                "browser_push.reminder.invalid",
                Some(delivery_id),
            ))
            .await
            .unwrap()
            .unwrap();
        assert!(audited.try_get::<bool>("", "audited").unwrap());
        assert_eq!(*transport.1.lock().unwrap(), 0);
        app.close().await;
    }
}

#[tokio::test]
async fn browser_reminder_submicrosecond_claim_clock_keeps_send_ownership() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-claim-precision").await;
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let delivery_id = seed_claimable_delivery(&app, &scope, now).await;
    let future = (Utc::now() + chrono::Duration::seconds(2))
        .with_nanosecond(123_456_789)
        .unwrap();
    assert_eq!(
        browser_push::reminders::drain_candidates(&app.context, &scope, future, &[delivery_id])
            .await
            .unwrap(),
        1
    );
    assert_eq!(transport.0.lock().unwrap().len(), 1);
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, Some(delivery_id)))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "accepted");
    app.close().await;
}

#[tokio::test]
async fn browser_reminder_in_flight_send_finishes_before_revocation_lands() {
    let app = Application::new().await;
    let (now, scope) = prepare_due_delivery(&app, "synthetic-presend-order").await;
    let transport = Arc::new(FirstBlockedDelivery {
        started: Notify::new(),
        release: Notify::new(),
        calls: Mutex::new(Vec::new()),
    });
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let delivery_id = seed_claimable_delivery(&app, &scope, now).await;
    let ctx = app.context.clone();
    let sending = scope.clone();
    let sender = tokio::spawn(async move {
        browser_push::reminders::drain_candidates(&ctx, &sending, now, &[delivery_id]).await
    });
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        transport.started.notified(),
    )
    .await
    .unwrap();
    let owner = owner_connection(&app).await;
    let (waiter, withdrawal) = {
        let owner = owner.begin().await.unwrap();
        let waiter = backend_pid(&owner).await;
        let query = Query::update()
            .table("push_subscriptions")
            .value("updated_at", Utc::now().naive_utc())
            .and_where(Expr::col("account_id").eq(app.fixture.account.id))
            .to_owned();
        let statement = owner.get_database_backend().build(&query);
        (
            Some(waiter),
            tokio::spawn(async move {
                let result = owner.execute_raw(statement).await;
                owner.commit().await?;
                result
            }),
        )
    };
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let row = app
                .fixture
                .admin
                .query_one_raw(blocking_query(&app, None, None, waiter))
                .await
                .unwrap()
                .unwrap();
            if row.try_get::<bool>("", "waiting").unwrap() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    transport.release.notify_one();
    let sent = tokio::time::timeout(std::time::Duration::from_secs(10), sender)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(sent.unwrap(), 1);
    tokio::time::timeout(std::time::Duration::from_secs(10), withdrawal)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    assert_eq!(transport.calls.lock().unwrap().len(), 1);
    let row = app
        .fixture
        .admin
        .query_one_raw(delivery_query(&app, Some(delivery_id)))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "status").unwrap(), "accepted");
    assert_eq!(
        row.try_get::<Option<String>>("", "outcome")
            .unwrap()
            .as_deref(),
        Some("accepted")
    );
    assert!(row.try_get::<bool>("", "subscribed").unwrap());
    assert!(row.try_get::<bool>("", "audited").unwrap());
    app.close().await;
}

async fn create_declared_dependent(
    app: &Application,
    token: &str,
    key: &str,
    name: &str,
    kind: &str,
    birth: &str,
) -> i64 {
    let household_id = app.fixture.household.id;
    let response = app
        .client
        .post(format!("{}/api/v1/households/{household_id}/people", app.origin))
        .bearer_auth(token)
        .header("idempotency-key", key)
        .json(&json!({"person":{"name":name,"date_of_birth":birth,"person_type":kind,"has_capacity":false,"initial_guardian_declaration":true}}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 201);
    response.json::<Value>().await.unwrap()["data"]["id"]
        .as_i64()
        .unwrap()
}

async fn seed_daily_schedule(
    app: &Application,
    person_id: i64,
    now: chrono::DateTime<Utc>,
    time: &str,
) -> i64 {
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::insert()
                .into_table("schedules")
                .columns([
                    "household_id",
                    "person_id",
                    "medication_id",
                    "active",
                    "start_date",
                    "end_date",
                    "dose_amount",
                    "dose_unit",
                    "schedule_type",
                    "schedule_config",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(person_id),
                    Expr::val(app.fixture.medication.id),
                    Expr::val(true),
                    Expr::val(now.date_naive() - chrono::Duration::days(1)),
                    Expr::val(now.date_naive() + chrono::Duration::days(2)),
                    Expr::val(2),
                    Expr::val("tablet"),
                    Expr::val(0),
                    Func::cust("jsonb_build_object")
                        .args([
                            Expr::val("times"),
                            Func::cust("jsonb_build_array")
                                .args([Expr::val(time)])
                                .into(),
                        ])
                        .into(),
                    Func::cust("now").into(),
                    Func::cust("now").into(),
                ])
                .returning(Query::returning().column("id").to_owned())
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    row.try_get("", "id").unwrap()
}

async fn declared_reminder_account(app: &Application) -> String {
    let token = app.token().await;
    async {
        let query = Query::update()
            .table("accounts")
            .value(
                "preferences",
                Func::cust("jsonb_build_object").args([Expr::val("time_zone"), Expr::val("UTC")]),
            )
            .and_where(Expr::col("id").eq(Expr::val(app.fixture.account.id)))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("notification_preferences")
            .columns([
                "household_id",
                "person_id",
                "enabled",
                "dose_due_enabled",
                "missed_dose_enabled",
                "low_stock_enabled",
                "private_text_enabled",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.household.id),
                Expr::val(app.fixture.adult.id),
                Expr::val(true),
                Expr::val(false),
                Expr::val(true),
                Expr::val(false),
                Expr::val(false),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("push_subscriptions")
            .columns([
                "account_id",
                "endpoint",
                "p256dh",
                "auth",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.account.id),
                Expr::val(format!(
                    "https://fcm.googleapis.com/fcm/send/{}/declared-reminder",
                    app.fixture.client_uuid
                )),
                Expr::val("synthetic"),
                Expr::val("synthetic"),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
    }
    .await
    .unwrap();
    token
}

#[tokio::test]
async fn browser_missed_reminder_reaches_declared_dependents_without_grants() {
    let app = Application::new().await;
    let token = declared_reminder_account(&app).await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let scheduled = (now - chrono::Duration::minutes(30))
        .format("%H:%M")
        .to_string();
    let mut declared = Vec::new();
    for (key, name, kind, birth) in [
        (
            "synthetic-declared-minor",
            "Synthetic declared minor",
            "minor",
            "2020-01-01",
        ),
        (
            "synthetic-declared-adult",
            "Synthetic declared adult",
            "dependent_adult",
            "1980-01-01",
        ),
    ] {
        let person_id = create_declared_dependent(&app, &token, key, name, kind, birth).await;
        declared.push(person_id);
        seed_daily_schedule(&app, person_id, now, &scheduled).await;
    }
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-declared-reminder".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        2
    );
    let mut bodies = transport
        .0
        .lock()
        .unwrap()
        .iter()
        .map(|payload| payload["body"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    bodies.sort();
    assert_eq!(
        bodies,
        [
            "Synthetic declared adult may have missed a dose.",
            "Synthetic declared minor may have missed a dose."
        ]
    );
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(Expr::col("id").count(), "grants")
                .from("person_access_grants")
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .and_where(Expr::col("person_id").is_in(declared))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "grants").unwrap(), 0);
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    app.close().await;
}

#[tokio::test]
async fn browser_missed_reminder_stops_declared_dependents_when_authority_breaks() {
    let app = Application::new().await;
    let token = declared_reminder_account(&app).await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let scheduled = (now - chrono::Duration::minutes(30))
        .format("%H:%M")
        .to_string();
    let mut people = Vec::new();
    for (key, name) in [
        (
            "synthetic-withdrawn-declaration",
            "Synthetic withdrawn child",
        ),
        ("synthetic-inactive-link", "Synthetic inactive link child"),
        ("synthetic-revoked-membership", "Synthetic revoked child"),
        ("synthetic-current-declaration", "Synthetic current child"),
    ] {
        let person_id =
            create_declared_dependent(&app, &token, key, name, "minor", "2020-01-01").await;
        people.push(person_id);
        seed_daily_schedule(&app, person_id, now, &scheduled).await;
    }
    let declaration = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .column("id")
                .from("guardian_declarations")
                .and_where(Expr::col("subject_person_id").eq(people[0]))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let declaration_id: i64 = declaration.try_get("", "id").unwrap();
    let withdrawn = app
        .client
        .patch(format!(
            "{}/api/v1/auth/guardian-declarations/{declaration_id}",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(withdrawn.status().as_u16(), 200);
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("carer_relationships")
                .value("active", false)
                .value("updated_at", Utc::now().naive_utc())
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .and_where(Expr::col("patient_id").eq(people[1]))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("household_memberships")
                .value("status", "revoked")
                .value("revoked_at", Utc::now().naive_utc())
                .value("updated_at", Utc::now().naive_utc())
                .value(
                    "permissions_version",
                    Expr::col("permissions_version").add(1),
                )
                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                .and_where(Expr::col("person_id").eq(people[2]))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-declared-revocation".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        1
    );
    {
        let bodies = transport.0.lock().unwrap();
        assert_eq!(bodies.len(), 1);
        assert_eq!(
            bodies[0]["body"].as_str().unwrap(),
            "Synthetic current child may have missed a dose."
        );
    }
    app.close().await;
}

#[tokio::test]
async fn browser_missed_reminder_skips_decided_dependent_doses_and_undeclared_managers() {
    let app = Application::new().await;
    let token = declared_reminder_account(&app).await;
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let scheduled_at = now - chrono::Duration::minutes(30);
    let scheduled = scheduled_at.format("%H:%M").to_string();
    let decided = create_declared_dependent(
        &app,
        &token,
        "synthetic-decided-dependent",
        "Synthetic decided child",
        "minor",
        "2020-01-01",
    )
    .await;
    let schedule = seed_daily_schedule(&app, decided, now, &scheduled).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("medication_dose_occurrences")
                .columns([
                    "household_id",
                    "schedule_id",
                    "window_starts_on",
                    "window_ends_on",
                    "position",
                    "scheduled_at",
                    "outcome",
                    "resolved_at",
                    "resolved_by_membership_id",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(app.fixture.household.id),
                    Expr::val(schedule),
                    Expr::val(scheduled_at.date_naive()),
                    Expr::val(scheduled_at.date_naive()),
                    Expr::val(1),
                    Expr::val(scheduled_at.naive_utc()),
                    Expr::val("not_taken"),
                    Func::cust("now").into(),
                    Expr::val(app.fixture.membership.id),
                    Func::cust("now").into(),
                    Func::cust("now").into(),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-declared-decided".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        0
    );
    async {
        let query = Query::insert()
            .into_table("people")
            .columns([
                "name",
                "person_type",
                "has_capacity",
                "created_at",
                "updated_at",
                "portable_id",
            ])
            .values_panic([
                Expr::val("Synthetic unrelated manager"),
                Expr::val(0),
                Expr::val(true),
                Func::cust("now").into(),
                Func::cust("now").into(),
                Expr::val(format!(
                    "{}-{}",
                    app.fixture.client_uuid, "Synthetic unrelated manager"
                )),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("accounts")
            .columns(["person_id", "email", "status", "created_at", "updated_at"])
            .values_panic([
                Expr::val(
                    participant_named(&app, "Synthetic unrelated manager")
                        .await
                        .id,
                ),
                Expr::val(format!(
                    "{}-unrelated-manager@example.test",
                    app.fixture.client_uuid
                )),
                Expr::val(2),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
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
                Expr::val(app.fixture.household.id),
                Expr::val(
                    participant_named(&app, "Synthetic unrelated manager")
                        .await
                        .id,
                ),
                Expr::val("administrator"),
                Expr::val("active"),
                Func::cust("now").into(),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("notification_preferences")
            .columns([
                "household_id",
                "person_id",
                "enabled",
                "dose_due_enabled",
                "missed_dose_enabled",
                "low_stock_enabled",
                "private_text_enabled",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.household.id),
                Expr::val(
                    participant_named(&app, "Synthetic unrelated manager")
                        .await
                        .id,
                ),
                Expr::val(true),
                Expr::val(false),
                Expr::val(true),
                Expr::val(false),
                Expr::val(false),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("push_subscriptions")
            .columns([
                "account_id",
                "endpoint",
                "p256dh",
                "auth",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(account_for(&app, "Synthetic unrelated manager").await.id),
                Expr::val(format!(
                    "https://fcm.googleapis.com/fcm/send/{}/unrelated-manager",
                    app.fixture.client_uuid
                )),
                Expr::val("synthetic"),
                Expr::val("synthetic"),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
    }
    .await
    .unwrap();
    let unrelated = HouseholdScope {
        actor: Actor {
            account_id: account_for(&app, "Synthetic unrelated manager").await.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-undeclared-manager".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &unrelated, now)
            .await
            .unwrap(),
        0
    );
    assert!(transport.0.lock().unwrap().is_empty());
    app.close().await;
}

#[tokio::test]
async fn browser_missed_reminder_keeps_declared_dependent_names_private() {
    let app = Application::new().await;
    let token = app.token().await;
    async {
        let query = Query::update()
            .table("accounts")
            .value(
                "preferences",
                Func::cust("jsonb_build_object").args([Expr::val("time_zone"), Expr::val("UTC")]),
            )
            .and_where(Expr::col("id").eq(Expr::val(app.fixture.account.id)))
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("notification_preferences")
            .columns([
                "household_id",
                "person_id",
                "enabled",
                "dose_due_enabled",
                "missed_dose_enabled",
                "low_stock_enabled",
                "private_text_enabled",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.household.id),
                Expr::val(app.fixture.adult.id),
                Expr::val(true),
                Expr::val(false),
                Expr::val(true),
                Expr::val(false),
                Expr::val(true),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        let _ = app
            .fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?;
        let query = Query::insert()
            .into_table("push_subscriptions")
            .columns([
                "account_id",
                "endpoint",
                "p256dh",
                "auth",
                "created_at",
                "updated_at",
            ])
            .values_panic([
                Expr::val(app.fixture.account.id),
                Expr::val(format!(
                    "https://fcm.googleapis.com/fcm/send/{}/declared-private",
                    app.fixture.client_uuid
                )),
                Expr::val("synthetic"),
                Expr::val("synthetic"),
                Func::cust("now").into(),
                Func::cust("now").into(),
            ])
            .to_owned();
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
    }
    .await
    .unwrap();
    let now = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let scheduled = (now - chrono::Duration::minutes(30))
        .format("%H:%M")
        .to_string();
    let dependent = create_declared_dependent(
        &app,
        &token,
        "synthetic-private-dependent",
        "Synthetic private child",
        "minor",
        "2020-01-01",
    )
    .await;
    seed_daily_schedule(&app, dependent, now, &scheduled).await;
    let transport = Arc::new(Reminders(Mutex::new(vec![])));
    app.context.shared_store.insert(Service {
        public_key: String::new(),
        transport: transport.clone(),
    });
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-declared-private".into(),
    };
    assert_eq!(
        browser_push::reminders::deliver_account(&app.context, &scope, now)
            .await
            .unwrap(),
        1
    );
    {
        let bodies = transport.0.lock().unwrap();
        assert_eq!(bodies.len(), 1);
        assert_eq!(
            bodies[0]["body"].as_str().unwrap(),
            "A dose may have been missed."
        );
    }
    app.close().await;
}

#[derive(Clone, Copy, Debug)]
enum AuthorityChange {
    Membership,
    Account,
    Household,
    Preference,
    Subscription,
}
fn authority_query(app: &Application, change: AuthorityChange) -> Statement {
    match change {
        AuthorityChange::Membership => {
            let query = Query::update()
                .table("household_memberships")
                .value("revoked_at", Utc::now().naive_utc())
                .and_where(Expr::col("person_id").eq(app.fixture.adult.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        }
        AuthorityChange::Account => {
            let query = Query::update()
                .table("accounts")
                .value("status", 3)
                .and_where(Expr::col("id").eq(app.fixture.account.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        }
        AuthorityChange::Household => {
            let query = Query::update()
                .table("households")
                .value("status", "archived")
                .value("lifecycle_state", "offboarded")
                .and_where(Expr::col("id").eq(app.fixture.household.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        }
        AuthorityChange::Preference => update_preference(app, "enabled", Expr::val(false)),
        AuthorityChange::Subscription => {
            let query = Query::delete()
                .from_table("push_subscriptions")
                .and_where(Expr::col("account_id").eq(app.fixture.account.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        }
    }
}
