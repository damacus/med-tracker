use super::now;
use chrono::{Duration, NaiveDate};
use med_tracker::models::{
    _entities::{
        accounts, app_settings, carer_relationships, household_invitations, household_memberships,
        identity_onboarding, people, users,
    },
    access,
};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::Set,
    ColumnTrait, ConnectionTrait, DatabaseConnection, DeriveIden, EntityTrait, ExprTrait,
    QueryFilter,
    sea_query::{Expr, Func, Query},
};

#[derive(DeriveIden)]
enum GuardianDeclarations {
    Table,
    Id,
    SubjectPersonId,
    GuardianPersonId,
    OriginHouseholdId,
    DeclaredByAccountId,
    SubjectNameAtDeclaration,
}

#[derive(DeriveIden)]
enum InvitationMailAttempts {
    Table,
    Id,
    HouseholdId,
    InvitationId,
    OriginalInvitationId,
    SenderAccountId,
    SenderMembershipId,
    SenderPermissionsVersion,
    TokenDigest,
    Status,
    Category,
    StartedAt,
    FinishedAt,
}

async fn person<C: ConnectionTrait>(
    db: &C,
    id: Option<i64>,
    name: &str,
    birthday: Option<NaiveDate>,
    minor: bool,
) -> i64 {
    let record = people::ActiveModel {
        id: id.map_or(sea_orm::ActiveValue::NotSet, Set),
        name: Set(name.into()),
        date_of_birth: Set(birthday),
        person_type: Set(i32::from(minor)),
        has_capacity: Set(!minor),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    };
    if let Some(id) = id {
        people::Entity::insert(record)
            .exec_without_returning(db)
            .await
            .expect("seed browser person before membership");
        id
    } else {
        record.insert(db).await.expect("seed browser person").id
    }
}

async fn membership<C: ConnectionTrait>(db: &C, person_id: i64, status: &str) {
    household_memberships::ActiveModel {
        household_id: Set(72001),
        person_id: Set(person_id),
        role: Set("member".into()),
        status: Set(status.into()),
        joined_at: Set(now()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("seed browser membership");
}

pub async fn seed_guardian_recovery(db: &DatabaseConnection, restore: bool) {
    let scope = access::HouseholdScope {
        actor: access::Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-browser-guardian-recovery".into(),
    };
    let fixture = access::begin(db, &scope, None).await.unwrap();
    let db = fixture.transaction();
    household_memberships::ActiveModel {
        id: Set(74001),
        role: Set("owner".into()),
        ..Default::default()
    }
    .update(db)
    .await
    .unwrap();
    let name = if restore {
        "Synthetic restore child"
    } else {
        "Synthetic recovery child"
    };
    person(
        db,
        Some(73994),
        name,
        NaiveDate::from_ymd_opt(2020, 1, 1),
        true,
    )
    .await;
    membership(db, 73994, "active").await;
    if restore {
        people::ActiveModel {
            id: Set(73001),
            name: Set("Linked guardian <care>".into()),
            ..Default::default()
        }
        .update(db)
        .await
        .unwrap();
        person(
            db,
            Some(73999),
            "Other eligible adult",
            NaiveDate::from_ymd_opt(1980, 1, 1),
            false,
        )
        .await;
        membership(db, 73999, "active").await;
        accounts::ActiveModel {
            person_id: Set(73999),
            email: Set("screenshot-other@example.test".into()),
            status: Set(2),
            created_at: Set(now().and_utc()),
            updated_at: Set(now().and_utc()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
        users::ActiveModel {
            person_id: Set(73999),
            email_address: Set("screenshot-other@example.test".into()),
            active: Set(true),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
        let declaration = Query::insert()
            .into_table(GuardianDeclarations::Table)
            .columns([
                GuardianDeclarations::SubjectPersonId,
                GuardianDeclarations::GuardianPersonId,
                GuardianDeclarations::OriginHouseholdId,
                GuardianDeclarations::DeclaredByAccountId,
                GuardianDeclarations::SubjectNameAtDeclaration,
            ])
            .values_panic([
                73994_i64.into(),
                73001_i64.into(),
                72001_i64.into(),
                71001_i64.into(),
                name.into(),
            ])
            .returning_col(GuardianDeclarations::Id)
            .to_owned();
        let declaration_id: i64 = db
            .query_one(&declaration)
            .await
            .unwrap()
            .unwrap()
            .try_get("", "id")
            .unwrap();
        carer_relationships::ActiveModel {
            carer_id: Set(73001),
            patient_id: Set(73994),
            household_id: Set(72001),
            active: Set(false),
            guardian_declaration_id: Set(Some(declaration_id)),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }
    fixture.commit().await.unwrap();
}

pub async fn seed_household_entrant(db: &DatabaseConnection, invitation: bool) {
    if invitation {
        users::Entity::delete_many()
            .filter(users::Column::PersonId.eq(73001))
            .exec(db)
            .await
            .unwrap();
    }
    let person_id = person(
        db,
        None,
        "Household entrant",
        NaiveDate::from_ymd_opt(1990, 1, 1),
        false,
    )
    .await;
    let mut account = accounts::ActiveModel {
        person_id: Set(person_id),
        email: Set("household-entry@example.test".into()),
        status: Set(2),
        created_at: Set(now().and_utc()),
        updated_at: Set(now().and_utc()),
        ..Default::default()
    };
    if !invitation {
        account.preferences = Set(serde_json::json!({"time_zone":"Europe/London"}));
    }
    let account = account.insert(db).await.unwrap();
    identity_onboarding::ActiveModel {
        account_id: Set(account.id),
        completed_at: Set(Some(now().and_utc().fixed_offset())),
    }
    .insert(db)
    .await
    .unwrap();
    if !invitation {
        app_settings::Entity::delete_many().exec(db).await.unwrap();
        app_settings::ActiveModel {
            invite_only: Set(true),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }
    super::seed_native_password(db, account.id).await;
}

pub async fn seed_suspended_members(db: &DatabaseConnection) {
    let adult = person(db, None, "Hidden person name", None, false).await;
    let minor = person(db, None, "Accountless dependant", None, true).await;
    accounts::ActiveModel {
        person_id: Set(adult),
        name: Set(Some("Inactive member".into())),
        email: Set("inactive@example.test".into()),
        status: Set(2),
        created_at: Set(now().and_utc()),
        updated_at: Set(now().and_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    membership(db, adult, "suspended").await;
    membership(db, minor, "suspended").await;
}

pub async fn seed_invitation(
    db: &DatabaseConnection,
    digest: &str,
) -> household_invitations::Model {
    invitation(db, "household-entry@example.test", "member", digest, 0, 7).await
}

async fn invitation(
    db: &DatabaseConnection,
    email: &str,
    role: &str,
    digest: &str,
    created_days_ago: i64,
    expires_days: i64,
) -> household_invitations::Model {
    let at = now();
    household_invitations::ActiveModel {
        household_id: Set(72001),
        email: Set(email.into()),
        membership_role: Set(role.into()),
        token_digest: Set(digest.into()),
        invited_by_membership_id: Set(74001),
        expires_at: Set(at + Duration::days(expires_days)),
        created_at: Set(at - Duration::days(created_days_ago)),
        updated_at: Set(at),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

pub async fn seed_invitation_management(db: &DatabaseConnection) -> household_invitations::Model {
    let maya = invitation(
        db,
        "maya@example.test",
        "member",
        "browser-invitation-maya",
        0,
        7,
    )
    .await;
    let alex = invitation(
        db,
        "alex@example.test",
        "administrator",
        "browser-invitation-alex",
        1,
        7,
    )
    .await;
    invitation(
        db,
        "legacy@example.test",
        "member",
        "browser-invitation-legacy",
        8,
        -1,
    )
    .await;
    for (invitation, failed) in [(&maya, true), (&alex, false)] {
        let at = now();
        let attempt = Query::insert()
            .into_table(InvitationMailAttempts::Table)
            .columns([
                InvitationMailAttempts::HouseholdId,
                InvitationMailAttempts::InvitationId,
                InvitationMailAttempts::OriginalInvitationId,
                InvitationMailAttempts::SenderAccountId,
                InvitationMailAttempts::SenderMembershipId,
                InvitationMailAttempts::SenderPermissionsVersion,
                InvitationMailAttempts::TokenDigest,
                InvitationMailAttempts::Status,
                InvitationMailAttempts::Category,
                InvitationMailAttempts::StartedAt,
                InvitationMailAttempts::FinishedAt,
            ])
            .values_panic([
                invitation.household_id.into(),
                invitation.id.into(),
                invitation.id.into(),
                71001_i64.into(),
                74001_i64.into(),
                1_i32.into(),
                invitation.token_digest.clone().into(),
                (if failed { "failed" } else { "queued" }).into(),
                failed.then_some("uncertain").into(),
                failed.then_some(at - Duration::hours(1)).into(),
                failed.then_some(at - Duration::minutes(30)).into(),
            ])
            .to_owned();
        db.execute(&attempt).await.unwrap();
    }
    maya
}

pub async fn detached_invitation_attempts(
    db: &DatabaseConnection,
    invitation: &household_invitations::Model,
) -> i64 {
    let count = Query::select()
        .expr_as(
            Func::count(Expr::col(InvitationMailAttempts::Id)),
            sea_orm::sea_query::Alias::new("count"),
        )
        .from(InvitationMailAttempts::Table)
        .and_where(Expr::col(InvitationMailAttempts::HouseholdId).eq(invitation.household_id))
        .and_where(Expr::col(InvitationMailAttempts::OriginalInvitationId).eq(invitation.id))
        .and_where(Expr::col(InvitationMailAttempts::InvitationId).is_null())
        .to_owned();
    db.query_one(&count)
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap()
}
