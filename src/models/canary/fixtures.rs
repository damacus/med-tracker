use super::{Canary, SCOPE, invalid, target, text};
use crate::models::_entities::{
    carer_relationships, household_memberships, households, identity_onboarding, locations,
    medication_takes, medications, notification_events, people, person_medications, schedules,
    users,
};
use chrono::{DateTime, Utc};
use loco_rs::app::AppContext;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DbErr, EntityTrait, IntoActiveModel, Set, TransactionTrait,
    sea_query::{Alias, ColumnDef, Expr, ExprTrait, Func, Query, Table},
};
use std::sync::Arc;

pub(super) async fn seed(ctx: &AppContext) -> Result<(), DbErr> {
    if !super::enabled(ctx) || !target(&ctx.config.database.uri, "medtracker_canary") {
        return Err(invalid());
    }
    let state = ctx.shared_store.get::<Arc<Canary>>().ok_or_else(invalid)?;
    let epoch: DateTime<Utc> = text(ctx, "epoch")?.parse().map_err(|_| invalid())?;
    let run_id = text(ctx, "run_id")?;
    let query = Query::select()
        .columns([
            Alias::new("phase"),
            Alias::new("image"),
            Alias::new("run_id"),
            Alias::new("updated_at"),
        ])
        .from(Alias::new("canary_control"))
        .and_where(Expr::col(Alias::new("scope")).eq(SCOPE))
        .to_owned();
    let row = state.control.query_one(&query).await?.ok_or_else(invalid)?;
    let heartbeat: DateTime<Utc> = row.try_get("", "updated_at")?;
    if row.try_get::<String>("", "phase")? != "seeding"
        || row.try_get::<String>("", "image")? != state.image
        || row.try_get::<String>("", "run_id")? != run_id
        || Utc::now() - heartbeat > chrono::Duration::seconds(30)
        || heartbeat > Utc::now()
    {
        return Err(invalid());
    }
    let transaction = ctx.db.begin().await?;
    transaction
        .execute_unprepared("SELECT pg_advisory_xact_lock(5567948073909809715)")
        .await?;
    let marker = Table::create()
        .table(Alias::new("canary_seed"))
        .if_not_exists()
        .col(ColumnDef::new(Alias::new("scope")).text().primary_key())
        .col(ColumnDef::new(Alias::new("image")).text().not_null())
        .col(ColumnDef::new(Alias::new("complete")).boolean().not_null())
        .to_owned();
    transaction.execute(&marker).await?;
    let query = Query::select()
        .columns([
            Alias::new("scope"),
            Alias::new("image"),
            Alias::new("complete"),
        ])
        .from(Alias::new("canary_seed"))
        .to_owned();
    let markers = transaction.query_all(&query).await?;
    if !markers.is_empty() {
        if markers.len() == 1
            && markers[0].try_get::<String>("", "scope")? == SCOPE
            && markers[0].try_get::<String>("", "image")? == state.image
            && markers[0].try_get::<bool>("", "complete")?
        {
            return transaction.commit().await;
        }
        return Err(invalid());
    }
    let query = Query::select()
        .column(Alias::new("table_name"))
        .from((Alias::new("information_schema"), Alias::new("tables")))
        .and_where(Expr::col(Alias::new("table_schema")).eq("public"))
        .and_where(Expr::col(Alias::new("table_type")).eq("BASE TABLE"))
        .to_owned();
    for row in transaction.query_all(&query).await? {
        let table: String = row.try_get("", "table_name")?;
        if matches!(table.as_str(), "seaql_migrations" | "canary_seed") {
            continue;
        }
        let query = Query::select()
            .expr(Expr::val(1))
            .from((Alias::new("public"), Alias::new(table)))
            .limit(1)
            .to_owned();
        if transaction.query_one(&query).await?.is_some() {
            return Err(invalid());
        }
    }
    let now = epoch.naive_utc();
    macro_rules! model {
        ($entity:ident) => {
            serde_yaml_ng::from_str::<Vec<$entity::Model>>(include_str!(concat!(
                "../../fixtures/",
                stringify!($entity),
                ".yaml"
            )))
            .map_err(|_| invalid())?
            .remove(0)
        };
    }
    let adult = model!(people);
    for (id, name, kind, capacity, birth) in [
        (
            73001,
            "Synthetic adult",
            0,
            true,
            chrono::NaiveDate::from_ymd_opt(1985, 2, 3),
        ),
        (
            73002,
            "Synthetic child",
            1,
            false,
            Some(epoch.date_naive() - chrono::Duration::days(3650)),
        ),
        (
            73003,
            "Synthetic unrelated adult",
            0,
            true,
            chrono::NaiveDate::from_ymd_opt(1980, 1, 1),
        ),
    ] {
        let mut row = adult.clone().into_active_model();
        row.id = Set(id);
        row.name = Set(name.into());
        row.person_type = Set(kind);
        row.has_capacity = Set(capacity);
        row.date_of_birth = Set(birth);
        row.portable_id = Set(uuid_for(id));
        row.created_at = Set(now);
        row.updated_at = Set(now);
        people::Entity::insert(row).exec(&transaction).await?;
    }
    let password = better_auth_core::utils::password::hash_password(None, "password")
        .await
        .map_err(|_| invalid())?;
    for (id, person_id, email) in [
        (71001, 73001, "persistence@example.test"),
        (71003, 73003, "unrelated@example.test"),
    ] {
        crate::models::identity::better_auth::schema::user::ActiveModel {
            id: Set(id),
            person_id: Set(person_id),
            email: Set(Some(email.into())),
            email_verified: Set(true),
            status: Set(2),
            name: Set(Some("Synthetic account".into())),
            preferences: Set(serde_json::json!({})),
            metadata: Set(serde_json::json!({})),
            two_factor_enabled: Set(false),
            created_at: Set(epoch),
            updated_at: Set(epoch),
            ..Default::default()
        }
        .insert(&transaction)
        .await?;
        crate::models::identity::better_auth::schema::account::ActiveModel {
            id: Set(format!("canary-{id}")),
            account_id: Set(id.to_string()),
            provider_id: Set("credential".into()),
            user_id: Set(id),
            password: Set(Some(password.clone())),
            created_at: Set(epoch),
            updated_at: Set(epoch),
            ..Default::default()
        }
        .insert(&transaction)
        .await?;
        let mut user = model!(users).into_active_model();
        user.id = Set(id + 6000);
        user.person_id = Set(person_id);
        user.email_address = Set(email.into());
        user.created_at = Set(now);
        user.updated_at = Set(now);
        users::Entity::insert(user).exec(&transaction).await?;
        identity_onboarding::ActiveModel {
            account_id: Set(id),
            completed_at: Set(Some(epoch.fixed_offset())),
        }
        .insert(&transaction)
        .await?;
    }
    for (id, creator, slug) in [
        (72001, 71001, "persistence-fixture"),
        (72002, 71001, "canary-second"),
        (72003, 71003, "canary-unrelated"),
    ] {
        let mut row = model!(households).into_active_model();
        row.id = Set(id);
        row.created_by_account_id = Set(Some(creator));
        row.name = Set(format!("Synthetic household {id}"));
        row.slug = Set(slug.into());
        row.created_at = Set(now);
        row.updated_at = Set(now);
        households::Entity::insert(row).exec(&transaction).await?;
    }
    for (id, household_id, person_id, role) in [
        (74001, 72001, 73001, "administrator"),
        (74002, 72001, 73002, "member"),
        (74003, 72002, 73001, "administrator"),
        (74004, 72003, 73003, "administrator"),
    ] {
        let mut row = model!(household_memberships).into_active_model();
        row.id = Set(id);
        row.household_id = Set(household_id);
        row.person_id = Set(person_id);
        row.role = Set(role.into());
        row.created_at = Set(now);
        row.updated_at = Set(now);
        row.joined_at = Set(now);
        household_memberships::Entity::insert(row)
            .exec(&transaction)
            .await?;
    }
    let mut context = Query::select();
    for (key, value) in [
        ("med_tracker.current_account_id", "71001"),
        ("med_tracker.current_household_id", "72001"),
        ("med_tracker.current_membership_id", "74001"),
    ] {
        context.expr(Func::cust(Alias::new("set_config")).args([
            Expr::val(key),
            Expr::val(value),
            Expr::val(true),
        ]));
    }
    transaction.query_one(&context).await?;
    let declaration = Query::insert()
        .into_table(Alias::new("guardian_declarations"))
        .columns([
            Alias::new("id"),
            Alias::new("subject_person_id"),
            Alias::new("guardian_person_id"),
            Alias::new("origin_household_id"),
            Alias::new("declared_by_account_id"),
            Alias::new("statement_version"),
            Alias::new("subject_name_at_declaration"),
            Alias::new("declared_at"),
        ])
        .values_panic([
            75001i64.into(),
            73002i64.into(),
            73001i64.into(),
            72001i64.into(),
            71001i64.into(),
            1i16.into(),
            "Synthetic child".into(),
            epoch.into(),
        ])
        .to_owned();
    transaction.execute(&declaration).await?;
    carer_relationships::ActiveModel {
        id: Set(75002),
        carer_id: Set(73001),
        patient_id: Set(73002),
        household_id: Set(72001),
        guardian_declaration_id: Set(Some(75001)),
        active: Set(true),
        relationship_type: Set(Some("parent".into())),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&transaction)
    .await?;
    let mut location = model!(locations).into_active_model();
    location.created_at = Set(now);
    location.updated_at = Set(now);
    locations::Entity::insert(location)
        .exec(&transaction)
        .await?;
    for (id, location_id, name) in [
        (80001, 79001, "Synthetic tablets"),
        (80002, 79001, "Synthetic spare stock"),
    ] {
        let mut row = model!(medications).into_active_model();
        row.id = Set(id);
        row.location_id = Set(location_id);
        row.name = Set(Some(name.into()));
        row.portable_id = Set(uuid_for(id));
        row.created_at = Set(now);
        row.updated_at = Set(now);
        medications::Entity::insert(row).exec(&transaction).await?;
    }
    for (id, person_id) in [(81001, 73001), (81002, 73002)] {
        let mut row = model!(person_medications).into_active_model();
        row.id = Set(id);
        row.person_id = Set(person_id);
        row.portable_id = Set(uuid_for(id));
        row.created_at = Set(now);
        row.updated_at = Set(now);
        person_medications::Entity::insert(row)
            .exec(&transaction)
            .await?;
    }
    schedules::ActiveModel {
        id: Set(82001),
        household_id: Set(72001),
        person_id: Set(73002),
        medication_id: Set(80001),
        portable_id: Set(uuid_for(82001)),
        active: Set(true),
        dose_amount: Set(Some(sea_orm::prelude::Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".into())),
        schedule_type: Set(1),
        schedule_config: Set(serde_json::json!({"times":["08:00"]})),
        start_date: Set(Some(epoch.date_naive())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&transaction)
    .await?;
    medication_takes::ActiveModel {
        id: Set(83001),
        household_id: Set(72001),
        person_medication_id: Set(Some(81001)),
        portable_id: Set(uuid_for(83001)),
        dose_amount: Set(Some(sea_orm::prelude::Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".into())),
        taken_at: Set(Some(now - chrono::Duration::hours(1))),
        taken_from_location_id: Set(Some(79001)),
        taken_from_medication_id: Set(Some(80001)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&transaction)
    .await?;
    notification_events::ActiveModel {
        id: Set(84001),
        household_id: Set(72001),
        person_id: Set(Some(73002)),
        event_key: Set("canary-reminder".into()),
        event_type: Set("dose_due".into()),
        metadata: Set(serde_json::json!({})),
        intended_at: Set(Some(now)),
        expires_at: Set(Some(now + chrono::Duration::hours(1))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&transaction)
    .await?;
    let marker = Query::insert()
        .into_table(Alias::new("canary_seed"))
        .columns([
            Alias::new("scope"),
            Alias::new("image"),
            Alias::new("complete"),
        ])
        .values_panic([SCOPE.into(), state.image.clone().into(), true.into()])
        .to_owned();
    transaction.execute(&marker).await?;
    transaction.commit().await
}
fn uuid_for(id: i64) -> String {
    format!("00000000-0000-4000-8000-{id:012}")
}
