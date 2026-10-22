use super::{Application, database_code, declaration, effects, require, scope, withdraw};
use med_tracker::models::{
    _entities::{accounts, carer_relationships, household_memberships, people, users},
    access::{self, PersonAccess},
    care::administration::recovery::{
        self, RecoveryAction, RecoveryCommand, RecoveryGuardianId, RecoverySubjectId,
    },
};
use sea_orm::{
    ActiveValue::Set,
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, TransactionTrait,
    sea_query::{Alias, Expr, ExprTrait, Func, Query},
};

fn operation_error(error: impl std::fmt::Display) -> DbErr {
    DbErr::Custom(error.to_string())
}

fn link(subject: i64, declaration_id: i64) -> carer_relationships::ActiveModel {
    let now = chrono::Utc::now().naive_utc();
    carer_relationships::ActiveModel {
        active: Set(true),
        carer_id: Set(73001),
        patient_id: Set(subject),
        household_id: Set(72001),
        guardian_declaration_id: Set(Some(declaration_id)),
        relationship_type: Set(Some("family_member".into())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}

async fn unlinked_initial(app: &Application) -> Result<(i64, i64), DbErr> {
    let now = chrono::Utc::now().naive_utc();
    people::Entity::insert(people::ActiveModel {
        id: Set(73994),
        name: Set("Synthetic unlinked initial minor".into()),
        date_of_birth: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1)),
        person_type: Set(1),
        has_capacity: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .exec(&app.fixture.admin)
    .await?;
    household_memberships::Entity::insert(household_memberships::ActiveModel {
        id: Set(74994),
        household_id: Set(72001),
        person_id: Set(73994),
        role: Set("member".into()),
        status: Set("active".into()),
        permissions_version: Set(1),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .exec(&app.fixture.admin)
    .await?;
    let tenant = access::begin(&app.fixture.runtime, &scope(71001, 72001), None)
        .await
        .map_err(operation_error)?;
    let role_query = Query::select()
        .expr_as(
            Func::cust("pg_catalog.current_setting").arg(Expr::val("is_superuser")),
            "superuser",
        )
        .expr_as(
            Func::cust("pg_catalog.pg_has_role")
                .args([Expr::val("med_tracker_app"), Expr::val("MEMBER")]),
            "app_role",
        )
        .to_owned();
    let role = tenant
        .transaction()
        .query_one_raw(
            tenant
                .transaction()
                .get_database_backend()
                .build(&role_query),
        )
        .await?
        .ok_or_else(|| DbErr::Custom("Runtime role witness missing".into()))?;
    require(
        role.try_get::<String>("", "superuser")? == "off"
            && role.try_get::<bool>("", "app_role")?,
        "Link attack must use the restricted application role",
    )?;
    let query = Query::insert()
        .into_table((Alias::new("public"), Alias::new("guardian_declarations")))
        .columns([
            "subject_person_id",
            "guardian_person_id",
            "origin_household_id",
            "declared_by_account_id",
            "subject_name_at_declaration",
        ])
        .values_panic([
            Expr::val(73994_i64),
            Expr::val(73001_i64),
            Expr::val(72001_i64),
            Expr::val(71001_i64),
            Expr::val("Synthetic unlinked initial minor"),
        ])
        .returning_col("id")
        .to_owned();
    let row = tenant
        .transaction()
        .query_one_raw(tenant.transaction().get_database_backend().build(&query))
        .await?
        .ok_or_else(|| DbErr::Custom("Initial declaration missing".into()))?;
    let id = row.try_get("", "id")?;
    tenant.commit().await.map_err(operation_error)?;
    Ok((73994, id))
}

async fn local_administrator(app: &Application) -> Result<(), DbErr> {
    let now = chrono::Utc::now().naive_utc();
    people::Entity::insert(people::ActiveModel {
        id: Set(73999),
        name: Set("Synthetic link administrator".into()),
        date_of_birth: Set(chrono::NaiveDate::from_ymd_opt(1980, 1, 1)),
        person_type: Set(0),
        has_capacity: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .exec(&app.fixture.admin)
    .await?;
    accounts::Entity::insert(accounts::ActiveModel {
        id: Set(71999),
        person_id: Set(73999),
        email: Set("link-manager@example.test".into()),
        status: Set(2),
        created_at: Set(now.and_utc()),
        updated_at: Set(now.and_utc()),
        ..Default::default()
    })
    .exec(&app.fixture.admin)
    .await?;
    users::Entity::insert(users::ActiveModel {
        id: Set(77999),
        person_id: Set(73999),
        email_address: Set("link-manager@example.test".into()),
        active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    })
    .exec(&app.fixture.admin)
    .await?;
    household_memberships::Entity::insert(household_memberships::ActiveModel {
        id: Set(74999),
        household_id: Set(72001),
        person_id: Set(73999),
        role: Set("administrator".into()),
        status: Set("active".into()),
        permissions_version: Set(1),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .exec(&app.fixture.admin)
    .await?;
    Ok(())
}

async fn terminal_insert(superseded: bool) -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let token = app.token().await;
        let (subject, id) = unlinked_initial(&app).await?;
        if superseded {
            local_administrator(&app).await?;
            accounts::Entity::update_many().col_expr(accounts::Column::Status, Expr::val(1))
                .filter(accounts::Column::Id.eq(71001)).exec(&app.fixture.admin).await?;
            let manager = access::begin(&app.fixture.runtime, &scope(71999, 72001), None)
                .await.map_err(operation_error)?;
            let selected = recovery::options(&manager).await.map_err(operation_error)?.subjects
                .into_iter().find(|item| item.id.get() == subject)
                .ok_or_else(|| DbErr::Custom("Unlinked orphan unavailable for verified recovery".into()))?;
            recovery::execute(&manager, RecoveryCommand {
                subject: RecoverySubjectId::parse(subject).map_err(operation_error)?,
                guardian: RecoveryGuardianId::parse(73999).map_err(operation_error)?,
                expected: selected.snapshot, action: RecoveryAction::Appoint,
                reason: "Appoint a replacement for this orphaned minor.".into(),
            }).await.map_err(operation_error)?;
            manager.commit().await.map_err(operation_error)?;
            accounts::Entity::update_many().col_expr(accounts::Column::Status, Expr::val(2))
                .filter(accounts::Column::Id.eq(71001)).exec(&app.fixture.admin).await?;
        } else {
            require(withdraw(&app, &token, id).await?.status().as_u16() == 200,
                "Terminal prerequisite must use genuine withdrawal")?;
        }
        let before = effects(&app).await?;
        let tenant = access::begin(&app.fixture.runtime, &scope(71001, 72001), None)
            .await.map_err(operation_error)?;
        let attempted = carer_relationships::Entity::insert(link(subject, id))
            .exec(tenant.transaction()).await;
        tenant.rollback().await.map_err(operation_error)?;
        require(attempted.as_ref().err().is_some_and(|error| database_code(error, "42501")),
            "Restricted INSERT must reject a terminal initial declaration, not create an active link")?;
        require(effects(&app).await? == before, "Rejected link must preserve clinical, audit and permission state")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_link_rejects_withdrawn_initial_insert() -> Result<(), DbErr> {
    terminal_insert(false).await
}

#[tokio::test]
async fn guardian_recovery_link_rejects_superseded_initial_insert() -> Result<(), DbErr> {
    terminal_insert(true).await
}

#[tokio::test]
async fn guardian_recovery_link_rejects_duplicate_and_other_actor() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let token = app.token().await;
        let (subject, id) = declaration(&app, &token).await?;
        let (unlinked_subject, unlinked_id) = unlinked_initial(&app).await?;
        local_administrator(&app).await?;
        let before = effects(&app).await?;
        for (account, subject, id) in [(71001, subject, id), (71999, unlinked_subject, unlinked_id)]
        {
            let tenant = access::begin(&app.fixture.runtime, &scope(account, 72001), None)
                .await
                .map_err(operation_error)?;
            let attempted = carer_relationships::Entity::insert(link(subject, id))
                .exec(tenant.transaction())
                .await;
            tenant.rollback().await.map_err(operation_error)?;
            require(
                attempted
                    .as_ref()
                    .err()
                    .is_some_and(|error| database_code(error, "42501")),
                "Only a fresh initial link by its declaring guardian may reach INSERT",
            )?;
        }
        require(
            effects(&app).await? == before,
            "Rejected actor and duplicate inserts must preserve all effects",
        )?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_link_delete_reinsert_cannot_restore_authority() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let token = app.token().await;
        let (subject, id) = declaration(&app, &token).await?;
        let tenant = access::begin(&app.fixture.runtime, &scope(71001, 72001), None)
            .await.map_err(operation_error)?;
        access::require_person_access(&tenant, subject, PersonAccess::Manage).await.map_err(operation_error)?;
        carer_relationships::Entity::update_many()
            .col_expr(carer_relationships::Column::Active, Expr::val(false))
            .filter(carer_relationships::Column::GuardianDeclarationId.eq(id))
            .exec(tenant.transaction()).await?;
        tenant.commit().await.map_err(operation_error)?;
        let before = effects(&app).await?;
        let tenant = access::begin(&app.fixture.runtime, &scope(71001, 72001), None)
            .await.map_err(operation_error)?;
        require(matches!(access::require_person_access(&tenant, subject, PersonAccess::Manage).await,
            Err(med_tracker::models::errors::OperationError::Forbidden)), "Inactive link must not grant management")?;
        let attempt = tenant.transaction().begin().await?;
        let deletion = carer_relationships::Entity::delete_many()
            .filter(carer_relationships::Column::GuardianDeclarationId.eq(id)).exec(&attempt).await;
        let rejected = match deletion {
            Err(error) => database_code(&error, "23514"),
            Ok(deleted) => {
                require(deleted.rows_affected == 1, "Attack must reach the genuine inactive relationship")?;
                carer_relationships::Entity::insert(link(subject, id)).exec(&attempt).await?;
                access::require_person_access(&tenant, subject, PersonAccess::Manage).await.map_err(operation_error)?;
                false
            }
        };
        attempt.rollback().await?;
        tenant.rollback().await.map_err(operation_error)?;
        require(!rejected || effects(&app).await? == before, "Denied deletion must preserve provenance and effects")?;
        require(rejected, "Restricted DELETE/reinsert restored guardian management without recovery confirmation or audit")?;
        Ok(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_link_valid_initial_insert_remains_available() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let (subject, id) = unlinked_initial(&app).await?;
        let tenant = access::begin(&app.fixture.runtime, &scope(71001, 72001), None)
            .await
            .map_err(operation_error)?;
        carer_relationships::Entity::insert(link(subject, id))
            .exec(tenant.transaction())
            .await?;
        let mut generic = link(subject, id);
        generic.guardian_declaration_id = Set(None);
        let inserted = carer_relationships::Entity::insert(generic)
            .exec(tenant.transaction())
            .await?;
        let deleted = carer_relationships::Entity::delete_by_id(inserted.last_insert_id)
            .exec(tenant.transaction())
            .await?;
        require(
            deleted.rows_affected == 1,
            "Generic relationship deletion must remain available",
        )?;
        access::require_person_access(&tenant, subject, PersonAccess::Manage)
            .await
            .map_err(operation_error)?;
        tenant.commit().await.map_err(operation_error)?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn guardian_recovery_link_insert_waits_for_terminal_transition() -> Result<(), DbErr> {
    let app = Application::new().await;
    let result = async {
        let (subject, id) = unlinked_initial(&app).await?;
        let terminal = access::begin(&app.fixture.runtime, &scope(71001, 72001), None)
            .await
            .map_err(operation_error)?;
        let query = Query::update()
            .table((Alias::new("public"), Alias::new("guardian_declarations")))
            .values([
                (
                    "withdrawn_at",
                    Expr::from(Func::cust("pg_catalog.clock_timestamp")),
                ),
                ("revision", Expr::val(2_i64)),
            ])
            .and_where(Expr::col("id").eq(id))
            .to_owned();
        terminal
            .transaction()
            .execute_raw(terminal.transaction().get_database_backend().build(&query))
            .await?;
        let runtime = app.fixture.runtime.clone();
        let (send_pid, receive_pid) = tokio::sync::oneshot::channel();
        let attempt = tokio::spawn(async move {
            let tenant = access::begin(&runtime, &scope(71001, 72001), None)
                .await
                .map_err(operation_error)?;
            let query = Query::select()
                .expr_as(Func::cust("pg_catalog.pg_backend_pid"), "pid")
                .to_owned();
            let row = tenant
                .transaction()
                .query_one_raw(tenant.transaction().get_database_backend().build(&query))
                .await?
                .ok_or_else(|| DbErr::Custom("Concurrent insert PID unavailable".into()))?;
            send_pid
                .send(row.try_get::<i32>("", "pid")?)
                .map_err(operation_error)?;
            let result = carer_relationships::Entity::insert(link(subject, id))
                .exec(tenant.transaction())
                .await;
            tenant.rollback().await.map_err(operation_error)?;
            result
        });
        let pid = receive_pid.await.map_err(operation_error)?;
        let mut waiting = false;
        for _ in 0..40 {
            let activity = Query::select()
                .expr(1)
                .from((Alias::new("pg_catalog"), Alias::new("pg_stat_activity")))
                .and_where(Expr::col("pid").eq(pid))
                .and_where(Expr::col("wait_event_type").eq("Lock"))
                .to_owned();
            let query = Query::select()
                .expr_as(Expr::exists(activity), "waiting")
                .to_owned();
            let row = app
                .fixture
                .admin
                .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
                .await?
                .ok_or_else(|| DbErr::Custom("Concurrent link lock witness missing".into()))?;
            waiting = row.try_get("", "waiting")?;
            if waiting || attempt.is_finished() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        terminal.commit().await.map_err(operation_error)?;
        let result = tokio::time::timeout(std::time::Duration::from_secs(10), attempt)
            .await
            .map_err(operation_error)?
            .map_err(operation_error)?;
        require(
            waiting
                && result
                    .as_ref()
                    .err()
                    .is_some_and(|error| database_code(error, "42501")),
            "Initial insert must wait for withdrawal and then reject the terminal declaration",
        )?;
        Ok(())
    }
    .await;
    app.close().await;
    result
}
