use super::{Application, fixture::Fixture};
use chrono::Timelike;
use loco_rs::{
    app::Hooks,
    task::{Tasks, Vars},
};
use med_tracker::models::_entities::audit_ledger_entries;
use med_tracker::models::_entities::{
    household_memberships, platform_admins, security_audit_events, support_access_sessions,
    versions,
};
use sea_orm::Condition;
use sea_orm::sea_query::{Alias, Expr, ExprTrait, Func, JoinType, Query};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbErr,
    EntityTrait, IntoActiveModel, QueryFilter, QueryResult, QuerySelect, Statement,
    TransactionTrait,
};

fn missing() -> DbErr {
    DbErr::Custom("Synthetic support expiry assertion row missing".into())
}

async fn seed(fixture: &Fixture, count: i32) -> Result<Vec<support_access_sessions::Model>, DbErr> {
    let now = chrono::Utc::now().naive_utc();
    let admin = platform_admins::Entity::insert(platform_admins::ActiveModel {
        account_id: Set(fixture.account.id),
        status: Set("active".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .on_conflict(
        sea_orm::sea_query::OnConflict::column(platform_admins::Column::AccountId)
            .update_column(platform_admins::Column::Status)
            .to_owned(),
    )
    .exec_with_returning(&fixture.admin)
    .await?;
    let mut sessions = Vec::new();
    for _ in 0..count {
        sessions.push(
            support_access_sessions::ActiveModel {
                household_id: Set(fixture.household.id),
                platform_admin_id: Set(admin.id),
                reason: Set("Synthetic worker expiry".into()),
                starts_at: Set(now - chrono::Duration::hours(2)),
                expires_at: Set(now - chrono::Duration::hours(1)),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&fixture.admin)
            .await?,
        );
    }
    Ok(sessions)
}

async fn runtime(fixture: &Fixture) -> Result<DatabaseTransaction, DbErr> {
    fixture.runtime.begin().await
}

async fn page(
    transaction: &DatabaseTransaction,
    after: i64,
    limit: i32,
) -> Result<Vec<i64>, DbErr> {
    // SeaQuery 1.0.2 Func::cust accepts one identifier, with no schema-qualified
    // function API. Keep this fixed capability call qualified under hostile search paths.
    transaction
        .query_all_raw(Statement::from_sql_and_values(
            transaction.get_database_backend(),
            "SELECT id FROM med_tracker.due_support_access_sessions($1,$2)",
            [after.into(), limit.into()],
        ))
        .await?
        .into_iter()
        .map(|row| row.try_get("", "id"))
        .collect()
}

async fn expire(transaction: &DatabaseTransaction, id: i64) -> Result<bool, DbErr> {
    // Func::cust has the same schema-qualified function limitation as page.
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            transaction.get_database_backend(),
            "SELECT med_tracker.expire_support_access_session($1) AS changed",
            [id.into()],
        ))
        .await?
        .ok_or_else(missing)?
        .try_get("", "changed")
}

fn count(table: &str, condition: Expr) -> Expr {
    Query::select()
        .expr(Expr::col("id").count())
        .from(Alias::new(table.to_owned()))
        .and_where(condition)
        .to_owned()
        .into()
}

fn audit_condition(id: i64) -> Expr {
    Expr::col("event_type")
        .eq("support_access_session.expired")
        .and(
            Func::cust("jsonb_extract_path_text")
                .args([
                    Expr::col("metadata"),
                    Expr::val("support_access_session_id"),
                ])
                .eq(id.to_string()),
        )
}

async fn state(fixture: &Fixture, id: i64) -> Result<QueryResult, DbErr> {
    let query = Query::select()
        .expr_as(Expr::col("expired_at").is_not_null(), "expired")
        .expr_as(
            Expr::col("expired_at").cast_as(Alias::new("text")),
            "expiry",
        )
        .expr_as(
            count(
                "versions",
                Expr::col("item_type")
                    .eq("SupportAccessSession")
                    .and(Expr::col("item_id").eq(id))
                    .and(Expr::col("event").eq("platform/support/expired")),
            ),
            "versions",
        )
        .expr_as(
            count("security_audit_events", audit_condition(id)),
            "audits",
        )
        .from("support_access_sessions")
        .and_where(Expr::col("id").eq(id))
        .to_owned();
    fixture
        .admin
        .query_one_raw(fixture.admin.get_database_backend().build(&query))
        .await?
        .ok_or_else(missing)
}

async fn change(fixture: &Fixture, id: i64, kind: u8) -> Result<(), DbErr> {
    let mut row = support_access_sessions::Entity::find_by_id(id)
        .one(&fixture.admin)
        .await?
        .ok_or_else(missing)?
        .into_active_model();
    let now = chrono::Utc::now().naive_utc();
    match kind {
        0 => row.expires_at = Set(now + chrono::Duration::hours(1)),
        1 => row.ended_at = Set(Some(now)),
        _ => row.expired_at = Set(Some(now)),
    }
    row.update(&fixture.admin).await?;
    Ok(())
}

async fn context(transaction: &DatabaseTransaction, key: &str, value: &str) -> Result<(), DbErr> {
    let query = Query::select()
        .expr(Func::cust("set_config").args([Expr::val(key), Expr::val(value), Expr::val(true)]))
        .to_owned();
    transaction
        .execute_raw(transaction.get_database_backend().build(&query))
        .await?;
    Ok(())
}
#[tokio::test]
async fn restricted_expiry_pages_past_500_and_excludes_ineligible_rows() -> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        let seeded = seed(&fixture, 504).await?;
        change(&fixture, seeded[501].id, 1).await?;
        change(&fixture, seeded[502].id, 2).await?;
        change(&fixture, seeded[503].id, 0).await?;
        let transaction = runtime(&fixture).await?;
        let first = page(&transaction, seeded[0].id - 1, 500).await?;
        let second = page(
            &transaction,
            first.last().copied().ok_or_else(missing)?,
            500,
        )
        .await?;
        let invalid = page(&transaction, seeded[0].id - 1, 501).await;
        transaction.rollback().await?;
        assert_eq!(first.len(), 500);
        assert_eq!(second, vec![seeded[500].id]);
        assert!(
            invalid.is_err(),
            "Paging must not accept an unbounded limit"
        );
        Ok(())
    }
    .await;
    release_admin(&fixture).await?;
    fixture.close().await;
    result
}

#[tokio::test]
async fn restricted_expiry_records_both_audits_once_after_requester_revocation() -> Result<(), DbErr>
{
    let fixture = Fixture::new().await;
    let result = async {
        let seeded = seed(&fixture, 1).await?;
        platform_admins::Entity::update_many()
            .col_expr(platform_admins::Column::Status, Expr::value("revoked"))
            .filter(platform_admins::Column::AccountId.eq(fixture.account.id))
            .exec(&fixture.admin)
            .await?;
        household_memberships::Entity::update_many()
            .col_expr(
                household_memberships::Column::Status,
                Expr::value("revoked"),
            )
            .col_expr(
                household_memberships::Column::RevokedAt,
                Expr::value(chrono::Utc::now().naive_utc()),
            )
            .filter(household_memberships::Column::PersonId.eq(fixture.adult.id))
            .exec(&fixture.admin)
            .await?;
        let transaction = runtime(&fixture).await?;
        let changed = expire(&transaction, seeded[0].id).await?;
        transaction.commit().await?;
        let original_expiry = state(&fixture, seeded[0].id)
            .await?
            .try_get::<String>("", "expiry")?;
        let transaction = runtime(&fixture).await?;
        let repeated = expire(&transaction, seeded[0].id).await?;
        transaction.commit().await?;
        let row = state(&fixture, seeded[0].id).await?;
        let query = Query::select()
            .expr_as(
                Func::cust("jsonb_extract_path_text")
                    .args([Expr::col("audit_context"), Expr::val("actor_account_id")]),
                "account",
            )
            .expr_as(
                Func::cust("jsonb_extract_path_text")
                    .args([Expr::col("audit_context"), Expr::val("active_role")]),
                "role",
            )
            .column("whodunnit")
            .from("versions")
            .and_where(Expr::col("item_type").eq("SupportAccessSession"))
            .and_where(Expr::col("item_id").eq(seeded[0].id))
            .and_where(Expr::col("event").eq("platform/support/expired"))
            .to_owned();
        let attribution = fixture
            .admin
            .query_one_raw(fixture.admin.get_database_backend().build(&query))
            .await?
            .ok_or_else(missing)?;

        assert!(changed);
        assert!(!repeated);
        assert_eq!(row.try_get::<String>("", "expiry")?, original_expiry);
        assert!(row.try_get::<bool>("", "expired")?);
        assert_eq!(row.try_get::<i64>("", "versions")?, 1);
        assert_eq!(row.try_get::<i64>("", "audits")?, 1);
        assert_eq!(
            attribution.try_get::<String>("", "account")?,
            fixture.account.id.to_string()
        );
        assert_eq!(
            attribution.try_get::<String>("", "role")?,
            "support_expiry_task"
        );
        assert_eq!(
            attribution.try_get::<Option<String>>("", "whodunnit")?,
            None
        );
        Ok(())
    }
    .await;
    release_admin(&fixture).await?;
    fixture.close().await;
    result
}

#[tokio::test]
async fn expiry_audit_failure_rolls_back_session_and_version_then_retries() -> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        let seeded=seed(&fixture, 1).await?;
        let fault=format!("reject_worker_expiry_{}",uuid::Uuid::new_v4().simple());
        let household=fixture.household.id;
        // SeaORM has no trigger or PL/pgSQL function builder.
        fixture.admin.execute_unprepared(&format!("CREATE FUNCTION public.{fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.household_id={household} AND NEW.event_type='support_access_session.expired' THEN RAISE EXCEPTION USING ERRCODE='23514',MESSAGE='Synthetic expiry audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER {fault} BEFORE INSERT ON public.security_audit_events FOR EACH ROW EXECUTE FUNCTION public.{fault}()")).await?;
        let transaction = runtime(&fixture).await?;
        let rejected = expire(&transaction, seeded[0].id).await;
        transaction.rollback().await?;
        let row = state(&fixture, seeded[0].id).await?;
        assert!(rejected.is_err(), "Synthetic audit failure must be reached");
        assert!(!row.try_get::<bool>("", "expired")?);
        assert_eq!(row.try_get::<i64>("", "versions")?, 0);
        assert_eq!(row.try_get::<i64>("", "audits")?, 0);
        fixture.admin.execute_unprepared(&format!("DROP TRIGGER {fault} ON public.security_audit_events; DROP FUNCTION public.{fault}()")).await?;
        let transaction = runtime(&fixture).await?;
        assert!(expire(&transaction, seeded[0].id).await?);
        transaction.commit().await?;
        let row = state(&fixture, seeded[0].id).await?;
        assert!(row.try_get::<bool>("", "expired")?);
        assert_eq!(row.try_get::<i64>("", "versions")?, 1);
        assert_eq!(row.try_get::<i64>("", "audits")?, 1);
        Ok(())
    }.await;
    release_admin(&fixture).await?;
    fixture.close().await;
    result
}

#[tokio::test]
async fn expiry_cannot_mutate_future_ended_expired_or_unknown_sessions() -> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        let seeded = seed(&fixture, 3).await?;
        change(&fixture, seeded[0].id, 0).await?;
        change(&fixture, seeded[1].id, 1).await?;
        change(&fixture, seeded[2].id, 2).await?;
        let transaction = runtime(&fixture).await?;
        context(&transaction, "search_path", "pg_temp").await?;
        let table = sea_orm::sea_query::Table::create()
            .temporary()
            .table("support_access_sessions")
            .col(sea_orm::sea_query::ColumnDef::new("id").big_integer())
            .col(sea_orm::sea_query::ColumnDef::new("expired_at").timestamp())
            .to_owned();
        transaction
            .execute_raw(transaction.get_database_backend().build(&table))
            .await?;
        let insert = Query::insert()
            .into_table("support_access_sessions")
            .columns(["id", "expired_at"])
            .values_panic([
                Expr::val(seeded[0].id),
                Expr::val(Option::<chrono::NaiveDateTime>::None),
            ])
            .to_owned();
        transaction
            .execute_raw(transaction.get_database_backend().build(&insert))
            .await?;

        for id in [seeded[0].id, seeded[1].id, seeded[2].id, i64::MAX] {
            assert!(
                !expire(&transaction, id).await?,
                "Ineligible session must remain unchanged"
            );
        }
        transaction.commit().await?;
        for session in &seeded {
            let row = state(&fixture, session.id).await?;
            assert_eq!(row.try_get::<i64>("", "versions")?, 0);
            assert_eq!(row.try_get::<i64>("", "audits")?, 0);
        }
        Ok(())
    }
    .await;
    release_admin(&fixture).await?;
    fixture.close().await;
    result
}

#[tokio::test]
async fn expiry_uses_utc_deadlines_and_timestamps_under_non_utc_caller_timezone()
-> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        let seeded = seed(&fixture, 2).await?;
        change(&fixture, seeded[1].id, 0).await?;
        let transaction = runtime(&fixture).await?;
        context(&transaction, "TimeZone", "Pacific/Kiritimati").await?;
        let due = page(&transaction, seeded[0].id - 1, 500).await?;
        let future = expire(&transaction, seeded[1].id).await?;
        let past = expire(&transaction, seeded[0].id).await?;
        let query = Query::select()
            .expr_as(
                Func::cust("current_setting").arg(Expr::val("TimeZone")),
                "timezone",
            )
            .expr_as(
                Func::cust("timezone")
                    .args([Expr::val("UTC"), Func::cust("clock_timestamp").into()]),
                "now",
            )
            .to_owned();
        let clock = transaction
            .query_one_raw(transaction.get_database_backend().build(&query))
            .await?
            .ok_or_else(missing)?;
        let now: chrono::NaiveDateTime = clock.try_get("", "now")?;
        let timezone: String = clock.try_get("", "timezone")?;
        let expired = support_access_sessions::Entity::find_by_id(seeded[0].id)
            .one(&transaction)
            .await?
            .ok_or_else(missing)?;
        let utc_stamp = expired
            .expired_at
            .is_some_and(|stamp| (stamp - now).num_milliseconds().abs() < 60_000);
        transaction.commit().await?;
        let (source_count, source_utc, ledger_count, ledger_utc) =
            utc_evidence(&fixture, seeded[0].id, now).await?;
        Ok::<_, DbErr>((
            due,
            future,
            past,
            utc_stamp,
            timezone,
            source_count,
            source_utc,
            ledger_count,
            ledger_utc,
            seeded[0].id,
        ))
    }
    .await;
    release_admin(&fixture).await?;
    fixture.close().await;
    let observed = result?;
    let expected_id = observed.9;
    assert_eq!(
        observed,
        (
            vec![expected_id],
            false,
            true,
            true,
            "Pacific/Kiritimati".to_owned(),
            2,
            true,
            2,
            true,
            expected_id
        )
    );
    Ok(())
}

#[tokio::test]
async fn caller_rollback_restores_expiry_and_both_audits() -> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        let seeded = seed(&fixture, 1).await?;
        let transaction = runtime(&fixture).await?;
        assert!(expire(&transaction, seeded[0].id).await?);
        transaction.rollback().await?;
        let row = state(&fixture, seeded[0].id).await?;
        assert!(!row.try_get::<bool>("", "expired")?);
        assert_eq!(row.try_get::<i64>("", "versions")?, 0);
        assert_eq!(row.try_get::<i64>("", "audits")?, 0);
        Ok(())
    }
    .await;
    release_admin(&fixture).await?;
    fixture.close().await;
    result
}

#[tokio::test]
async fn simultaneous_expiry_invocations_create_one_dual_audit() -> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        let seeded = seed(&fixture, 1).await?;
        let left = async {
            let transaction = runtime(&fixture).await?;
            let changed = expire(&transaction, seeded[0].id).await?;
            transaction.commit().await?;
            Ok::<_, DbErr>(changed)
        };
        let right = async {
            let transaction = runtime(&fixture).await?;
            let changed = expire(&transaction, seeded[0].id).await?;
            transaction.commit().await?;
            Ok::<_, DbErr>(changed)
        };
        let (left, right) = tokio::time::timeout(std::time::Duration::from_secs(15), async {
            tokio::join!(left, right)
        })
        .await
        .map_err(|_| DbErr::Custom("Synthetic expiry concurrency timeout".into()))?;
        assert_ne!(
            left?, right?,
            "Exactly one invocation must apply the transition"
        );
        let row = state(&fixture, seeded[0].id).await?;
        assert!(row.try_get::<bool>("", "expired")?);
        assert_eq!(row.try_get::<i64>("", "versions")?, 1);
        assert_eq!(row.try_get::<i64>("", "audits")?, 1);
        Ok(())
    }
    .await;
    release_admin(&fixture).await?;
    fixture.close().await;
    result
}

#[tokio::test]
async fn expiry_capabilities_have_fixed_nonlogin_owner_and_no_public_execution() -> Result<(), DbErr>
{
    let fixture = Fixture::new().await;
    let result = async {
        let acl = Query::select()
            .expr(Expr::col("grantee").count())
            .from_function(
                Func::cust("aclexplode").arg(
                    Func::cust("coalesce").args([
                        Expr::col(("p", "proacl")),
                        Func::cust("acldefault")
                            .args([
                                Func::cast_as_quoted(Expr::val("f"), "char").into(),
                                Expr::col(("p", "proowner")),
                            ])
                            .into(),
                    ]),
                ),
                "a",
            )
            .and_where(Expr::col("grantee").eq(0))
            .and_where(Expr::col("privilege_type").eq("EXECUTE"))
            .to_owned();
        let query = Query::select()
            .columns([
                ("p", "prosecdef"),
                ("r", "rolname"),
                ("r", "rolcanlogin"),
                ("r", "rolsuper"),
                ("r", "rolbypassrls"),
            ])
            .expr_as(
                Func::cust("to_jsonb").arg(Expr::col(("p", "proconfig"))),
                "config",
            )
            .expr_as(
                Func::cust("has_function_privilege").args([
                    Expr::val("med_tracker_app"),
                    Expr::col(("p", "oid")),
                    Expr::val("EXECUTE"),
                ]),
                "execute",
            )
            .expr_as(Expr::from(acl), "public_execute")
            .from_as("pg_proc", "p")
            .join_as(
                JoinType::InnerJoin,
                "pg_namespace",
                "n",
                Expr::col(("n", "oid")).eq(Expr::col(("p", "pronamespace"))),
            )
            .join_as(
                JoinType::InnerJoin,
                "pg_roles",
                "r",
                Expr::col(("r", "oid")).eq(Expr::col(("p", "proowner"))),
            )
            .and_where(Expr::col(("n", "nspname")).eq("med_tracker"))
            .and_where(Expr::col(("p", "proname")).is_in([
                "due_support_access_sessions",
                "expire_support_access_session",
            ]))
            .to_owned();
        let rows = fixture
            .admin
            .query_all_raw(fixture.admin.get_database_backend().build(&query))
            .await?;
        assert_eq!(rows.len(), 2);
        for row in rows {
            let config: serde_json::Value = row.try_get("", "config")?;
            assert!(row.try_get::<bool>("", "prosecdef")?);
            assert_eq!(row.try_get::<String>("", "rolname")?, "med_tracker_owner");
            for field in ["rolcanlogin", "rolsuper", "rolbypassrls"] {
                assert!(!row.try_get::<bool>("", field)?);
            }
            assert!(config.as_array().is_some_and(|items| {
                items.contains(&serde_json::json!("search_path=pg_catalog"))
                    && items.contains(&serde_json::json!("TimeZone=UTC"))
            }));
            assert!(row.try_get::<bool>("", "execute")?);
            assert_eq!(row.try_get::<i64>("", "public_execute")?, 0);
        }
        let seeded = seed(&fixture, 1).await?;
        let foreign = Fixture::new().await;
        let foreign_sessions = seed(&foreign, 1).await?;
        platform_admins::Entity::update_many()
            .col_expr(platform_admins::Column::Status, Expr::value("revoked"))
            .filter(platform_admins::Column::AccountId.eq(foreign.account.id))
            .exec(&fixture.admin)
            .await?;
        let transaction = runtime(&fixture).await?;
        context(&transaction, "search_path", "pg_temp").await?;
        for name in ["versions", "security_audit_events"] {
            let table = sea_orm::sea_query::Table::create()
                .temporary()
                .table(name)
                .col(sea_orm::sea_query::ColumnDef::new("id").big_integer())
                .to_owned();
            transaction
                .execute_raw(transaction.get_database_backend().build(&table))
                .await?;
        }
        context(
            &transaction,
            "med_tracker.current_household_id",
            &foreign.household.id.to_string(),
        )
        .await?;
        context(
            &transaction,
            "med_tracker.current_account_id",
            &fixture.account.id.to_string(),
        )
        .await?;
        assert!(expire(&transaction, seeded[0].id).await?);
        assert!(expire(&transaction, foreign_sessions[0].id).await?);
        let query = Query::select()
            .expr_as(
                Func::cust("current_setting").arg(Expr::val("med_tracker.current_household_id")),
                "household",
            )
            .expr_as(
                Func::cust("current_setting").arg(Expr::val("med_tracker.current_account_id")),
                "account",
            )
            .to_owned();
        let context = transaction
            .query_one_raw(transaction.get_database_backend().build(&query))
            .await?
            .ok_or_else(missing)?;
        assert_eq!(
            context.try_get::<String>("", "household")?,
            foreign.household.id.to_string()
        );
        assert_eq!(
            context.try_get::<String>("", "account")?,
            fixture.account.id.to_string()
        );
        transaction.commit().await?;
        let row = state(&fixture, seeded[0].id).await?;
        assert_eq!(row.try_get::<i64>("", "audits")?, 1);
        let original = security_audit_events::Entity::find()
            .filter(audit_condition(foreign_sessions[0].id))
            .one(&fixture.admin)
            .await?
            .ok_or_else(missing)?;
        assert_eq!(original.actor_account_id, Some(foreign.account.id));
        assert_eq!(original.household_id, foreign.household.id);
        assert_eq!(original.metadata["outcome"], "expired");
        let deadline = original.metadata["expired_at"]
            .as_str()
            .ok_or_else(missing)?
            .parse::<chrono::DateTime<chrono::FixedOffset>>()
            .map_err(|_| missing())?;
        assert_eq!(
            deadline.naive_utc(),
            foreign_sessions[0]
                .expires_at
                .with_nanosecond(0)
                .ok_or_else(missing)?
        );
        foreign.close().await;

        Ok(())
    }
    .await;
    release_admin(&fixture).await?;
    fixture.close().await;
    result
}

#[test]
fn support_expiry_cli_and_minute_schedules_are_registered() -> Result<(), Box<dyn std::error::Error>>
{
    let mut tasks = Tasks::default();
    med_tracker::app::App::register_tasks(&mut tasks);
    assert!(
        tasks
            .names()
            .iter()
            .any(|name| name == "support-access-expire")
    );
    for environment in [
        loco_rs::environment::Environment::Development,
        loco_rs::environment::Environment::Production,
    ] {
        let config = loco_rs::config::Config::new(&environment)?;
        let scheduler = config.scheduler.ok_or_else(missing)?;
        let job = scheduler
            .jobs
            .get("support_access_expiry")
            .ok_or_else(missing)?;
        assert_eq!(job.run, "support-access-expire");
        assert_eq!(job.cron, "0 * * * * *");
    }
    Ok(())
}

#[tokio::test]
async fn registered_worker_drains_backlog_and_retry_is_a_noop()
-> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new().await;
    let result = async {
        let seeded = seed(&app.fixture, 1001).await?;
        let mut tasks = Tasks::default();
        med_tracker::app::App::register_tasks(&mut tasks);
        tasks
            .run(&app.context, "support-access-expire", &Vars::default())
            .await?;
        tasks
            .run(&app.context, "support-access-expire", &Vars::default())
            .await?;
        let ids = seeded.iter().map(|row| row.id).collect::<Vec<_>>();
        let query = Query::select()
            .expr_as(
                count(
                    "support_access_sessions",
                    Expr::col("id")
                        .is_in(ids.clone())
                        .and(Expr::col("expired_at").is_not_null()),
                ),
                "expired",
            )
            .expr_as(
                count(
                    "versions",
                    Expr::col("item_type")
                        .eq("SupportAccessSession")
                        .and(Expr::col("event").eq("platform/support/expired"))
                        .and(Expr::col("item_id").is_in(ids.clone())),
                ),
                "versions",
            )
            .expr_as(
                count(
                    "security_audit_events",
                    Expr::col("household_id")
                        .eq(app.fixture.household.id)
                        .and(Expr::col("event_type").eq("support_access_session.expired")),
                ),
                "audits",
            )
            .to_owned();
        let row = app
            .fixture
            .admin
            .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?
            .ok_or_else(missing)?;
        assert_eq!(row.try_get::<i64>("", "expired")?, 1001);
        assert_eq!(row.try_get::<i64>("", "versions")?, 1001);
        assert_eq!(row.try_get::<i64>("", "audits")?, 1001);
        Ok::<_, Box<dyn std::error::Error>>(())
    }
    .await;
    release_admin(&app.fixture).await?;
    app.close().await;
    result
}

#[tokio::test]
async fn registered_worker_pages_past_500_failures_and_retries_them()
-> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new().await;
    let result = async {
        let seeded=seed(&app.fixture, 501).await?;
        let fault=format!("reject_expiry_page_{}",uuid::Uuid::new_v4().simple());
        let household=app.fixture.household.id;
        let last=seeded[499].id;
        // SeaORM has no trigger or PL/pgSQL function builder.
        app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION public.{fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.household_id={household} AND NEW.event_type='support_access_session.expired' AND (NEW.metadata->>'support_access_session_id')::bigint<={last} THEN RAISE EXCEPTION USING ERRCODE='23514',MESSAGE='Synthetic first expiry page failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER {fault} BEFORE INSERT ON public.security_audit_events FOR EACH ROW EXECUTE FUNCTION public.{fault}()")).await?;
        let mut tasks = Tasks::default();
        med_tracker::app::App::register_tasks(&mut tasks);
        let partial = tasks.run(&app.context, "support-access-expire", &Vars::default()).await;
        let last = state(&app.fixture, seeded[500].id).await?;
        let first = state(&app.fixture, seeded[0].id).await?;
        assert!(partial.is_err(), "Partial expiry must be visible to the scheduler");
        assert!(last.try_get::<bool>("", "expired")?, "A failed first page must not starve later rows");
        assert!(!first.try_get::<bool>("", "expired")?);
        assert_eq!(first.try_get::<i64>("", "versions")?, 0);
        assert_eq!(first.try_get::<i64>("", "audits")?, 0);
        app.fixture.admin.execute_unprepared(&format!("DROP TRIGGER {fault} ON public.security_audit_events; DROP FUNCTION public.{fault}()")).await?;
        tasks.run(&app.context, "support-access-expire", &Vars::default()).await?;
        let first = state(&app.fixture, seeded[0].id).await?;
        assert!(first.try_get::<bool>("", "expired")?);
        assert_eq!(first.try_get::<i64>("", "versions")?, 1);
        assert_eq!(first.try_get::<i64>("", "audits")?, 1);
        Ok::<_, Box<dyn std::error::Error>>(())
    }.await;
    release_admin(&app.fixture).await?;
    app.close().await;
    result
}

#[tokio::test]
async fn real_listing_and_worker_overlap_without_deadlock_or_lost_audits()
-> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new().await;
    let fault = format!("support_listing_{}", uuid::Uuid::new_v4().simple());
    let probe = format!("{fault}_pid");
    let gate_key = i32::from_be_bytes(uuid::Uuid::new_v4().as_bytes()[..4].try_into().unwrap());
    let mut listing = None;
    let mut worker = None;
    let result = async {
        let cookie = super::platform::admin_session(&app).await;
        let older = super::platform::active_support(&app, &cookie).await;
        let mut old=support_access_sessions::Entity::find_by_id(older).one(&app.fixture.admin).await?.ok_or_else(missing)?.into_active_model();
        old.expires_at=Set(chrono::Utc::now().naive_utc()-chrono::Duration::seconds(1));
        let updated=old.update(&app.fixture.admin).await?;
        let mut newer_row=updated.into_active_model();newer_row.id=sea_orm::ActiveValue::NotSet;
        newer_row.reason=Set("Synthetic newer overlap session".into());
        let newer=newer_row.insert(&app.fixture.admin).await?.id;
        let before=deadlocks(&app.fixture).await?;
        // SeaORM has no trigger, PL/pgSQL function or object-owner/grant builder.
        app.fixture.admin.execute_unprepared(&format!(
            "CREATE SEQUENCE public.{probe}; ALTER SEQUENCE public.{probe} OWNER TO med_tracker_owner; CREATE FUNCTION public.{fault}() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$ BEGIN IF NEW.item_type='SupportAccessSession' AND NEW.event='platform/support/expired' AND NEW.item_id={newer} THEN PERFORM pg_catalog.setval('public.{probe}'::regclass,pg_catalog.pg_backend_pid(),true); PERFORM pg_catalog.pg_advisory_xact_lock({gate_key},1); END IF; RETURN NEW; END $$; ALTER FUNCTION public.{fault}() OWNER TO med_tracker_owner; REVOKE ALL ON FUNCTION public.{fault}() FROM PUBLIC; CREATE TRIGGER {fault} AFTER INSERT ON public.versions FOR EACH ROW EXECUTE FUNCTION public.{fault}()"
        )).await?;
        let gate = app.fixture.admin.begin().await?;
        let lock_query=Query::select().expr(Func::cust("pg_advisory_xact_lock").args([Expr::val(gate_key),Expr::val(1)])).to_owned();
        gate.execute_raw(gate.get_database_backend().build(&lock_query)).await?;
        let client = app.client.clone();
        let origin = app.origin.clone();
        listing = Some(tokio::spawn(async move {
            client.get(format!("{origin}/platform/support")).header("cookie",cookie)
                .timeout(std::time::Duration::from_secs(30)).send().await.map(|response|response.status())
        }));
        let mut listing_pid = None;
        for _ in 0..100 {
            let query=Query::select().columns(["last_value","is_called"]).from(Alias::new(probe.clone())).to_owned();
            let row=app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&query)).await?.ok_or_else(missing)?;
            if row.try_get::<bool>("", "is_called")? {
                listing_pid = Some(i32::try_from(row.try_get::<i64>("", "last_value")?)?);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        let listing_pid = listing_pid.ok_or_else(||std::io::Error::other("Real listing did not reach its post-ledger gate"))?;
        let context = app.context.clone();
        worker = Some(tokio::spawn(async move {
            let mut tasks = Tasks::default();
            med_tracker::app::App::register_tasks(&mut tasks);
            tasks.run(&context,"support-access-expire",&Vars::default()).await
        }));
        let mut blocked = false;
        for _ in 0..100 {
            blocked=is_blocked(&app.fixture,listing_pid).await?;
            if blocked { break; }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        if !blocked { return Err(std::io::Error::other("Registered worker did not wait on the real listing ledger lock").into()); }
        gate.commit().await?;
        let listing_handle = listing.as_mut().ok_or_else(missing)?;
        let worker_handle = worker.as_mut().ok_or_else(missing)?;
        let (listing_result,worker_result) = tokio::time::timeout(std::time::Duration::from_secs(20), async {
            tokio::join!(listing_handle,worker_handle)
        }).await.map_err(|_|std::io::Error::other("Support listing/worker overlap timed out"))?;
        let listing_status = listing_result??;
        let worker_succeeded = worker_result?.is_ok();
        let mut after = before;
        for _ in 0..20 {
            after=deadlocks(&app.fixture).await?;
            if after>before { break; }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        let query=Query::select()
            .expr_as(count("versions",Expr::col("item_type").eq("SupportAccessSession").and(Expr::col("event").eq("platform/support/expired")).and(Expr::col("item_id").is_in([older,newer]))),"versions")
            .expr_as(count("security_audit_events",audit_condition(older).or(audit_condition(newer))),"security").to_owned();
        let counts=app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&query)).await?.ok_or_else(missing)?;
        Ok::<_,Box<dyn std::error::Error>>((listing_status.as_u16(),worker_succeeded,after-before,
            counts.try_get::<i64>("", "versions")?,counts.try_get::<i64>("", "security")?))
    }.await;
    if let Some(handle) = listing
        && !handle.is_finished()
    {
        handle.abort();
        let _ = handle.await;
    }
    if let Some(handle) = worker
        && !handle.is_finished()
    {
        handle.abort();
        let _ = handle.await;
    }
    app.fixture.admin.execute_unprepared(&format!("DROP TRIGGER IF EXISTS {fault} ON public.versions; DROP FUNCTION IF EXISTS public.{fault}(); DROP SEQUENCE IF EXISTS public.{probe}")).await?;
    release_admin(&app.fixture).await?;
    app.close().await;
    assert_eq!(result?, (200, true, 0, 2, 2));
    Ok(())
}

#[tokio::test]
async fn busy_expiry_is_false_without_audits_and_succeeds_after_unlock() -> Result<(), DbErr> {
    let fixture = Fixture::new().await;
    let result = async {
        let seeded = seed(&fixture, 1).await?;
        let lock = fixture.admin.begin().await?;
        support_access_sessions::Entity::find_by_id(seeded[0].id)
            .lock_exclusive()
            .one(&lock)
            .await?;
        let transaction = runtime(&fixture).await?;
        let busy = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            expire(&transaction, seeded[0].id),
        )
        .await
        .map_err(|_| DbErr::Custom("Busy expiry did not return without waiting".into()))??;
        transaction.commit().await?;
        let before = state(&fixture, seeded[0].id).await?;
        let before = (
            before.try_get::<bool>("", "expired")?,
            before.try_get::<i64>("", "versions")?,
            before.try_get::<i64>("", "audits")?,
        );
        lock.commit().await?;
        let transaction = runtime(&fixture).await?;
        let retried = expire(&transaction, seeded[0].id).await?;
        transaction.commit().await?;
        let after = state(&fixture, seeded[0].id).await?;
        let after = (
            after.try_get::<bool>("", "expired")?,
            after.try_get::<i64>("", "versions")?,
            after.try_get::<i64>("", "audits")?,
        );
        Ok::<_, DbErr>((busy, before, retried, after))
    }
    .await;
    release_admin(&fixture).await?;
    fixture.close().await;
    assert_eq!(result?, (false, (false, 0, 0), true, (true, 1, 1)));
    Ok(())
}

async fn utc_evidence(
    fixture: &Fixture,
    id: i64,
    now: chrono::NaiveDateTime,
) -> Result<(i64, bool, i64, bool), DbErr> {
    let version = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("SupportAccessSession"))
        .filter(versions::Column::ItemId.eq(id))
        .all(&fixture.admin)
        .await?;
    let audit = security_audit_events::Entity::find()
        .filter(audit_condition(id))
        .all(&fixture.admin)
        .await?;
    let source_count = i64::try_from(version.len() + audit.len()).map_err(|_| missing())?;
    let utc = version.iter().all(|row| {
        row.created_at
            .is_some_and(|stamp| (stamp - now).num_milliseconds().abs() < 60_000)
    }) && audit
        .iter()
        .all(|row| (row.created_at - now).num_milliseconds().abs() < 60_000);
    let rows = audit_ledger_entries::Entity::find()
        .filter(
            Condition::any()
                .add(
                    Condition::all()
                        .add(audit_ledger_entries::Column::SourceTable.eq("versions"))
                        .add(
                            audit_ledger_entries::Column::SourceId
                                .is_in(version.iter().map(|row| row.id)),
                        ),
                )
                .add(
                    Condition::all()
                        .add(audit_ledger_entries::Column::SourceTable.eq("security_audit_events"))
                        .add(
                            audit_ledger_entries::Column::SourceId
                                .is_in(audit.iter().map(|row| row.id)),
                        ),
                ),
        )
        .all(&fixture.admin)
        .await?;
    let ledger_utc = rows.iter().all(|row| {
        let envelope = row.envelope["occurred_at"]
            .as_str()
            .and_then(|stamp| stamp.parse::<chrono::DateTime<chrono::FixedOffset>>().ok());
        (row.occurred_at - now).num_milliseconds().abs() < 60_000
            && (row.created_at - now).num_milliseconds().abs() < 60_000
            && envelope
                .is_some_and(|stamp| (stamp.naive_utc() - now).num_milliseconds().abs() < 60_000)
    });
    Ok((
        source_count,
        utc,
        i64::try_from(rows.len()).map_err(|_| missing())?,
        ledger_utc,
    ))
}

async fn release_admin(fixture: &Fixture) -> Result<(), DbErr> {
    platform_admins::Entity::update_many()
        .col_expr(platform_admins::Column::Status, Expr::value("revoked"))
        .filter(platform_admins::Column::AccountId.eq(fixture.account.id))
        .exec(&fixture.admin)
        .await?;
    Ok(())
}

async fn deadlocks(fixture: &Fixture) -> Result<i64, DbErr> {
    let query = Query::select()
        .column("deadlocks")
        .from("pg_stat_database")
        .and_where(Expr::col("datname").eq(Func::cust("current_database")))
        .to_owned();
    fixture
        .admin
        .query_one_raw(fixture.admin.get_database_backend().build(&query))
        .await?
        .ok_or_else(missing)?
        .try_get("", "deadlocks")
}

async fn is_blocked(fixture: &Fixture, pid: i32) -> Result<bool, DbErr> {
    let query = Query::select()
        .expr_as(
            Func::cust("to_jsonb").arg(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))),
            "blockers",
        )
        .from("pg_stat_activity")
        .and_where(Expr::col("datname").eq(Func::cust("current_database")))
        .to_owned();
    let rows = fixture
        .admin
        .query_all_raw(fixture.admin.get_database_backend().build(&query))
        .await?;
    for row in rows {
        let blockers: Vec<i32> =
            serde_json::from_value(row.try_get::<serde_json::Value>("", "blockers")?)
                .map_err(|_| missing())?;
        if blockers.contains(&pid) {
            return Ok(true);
        }
    }
    Ok(false)
}
