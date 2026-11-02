use super::now;
use chrono::{Duration, NaiveDate};
use med_tracker::models::{
    _entities::{
        accounts, api_sessions, app_settings, carer_relationships, household_invitations,
        household_memberships, identity_onboarding, medications, people,
    },
    access,
};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::Set,
    ConnectionTrait, DatabaseConnection, DeriveIden, EntityTrait, ExprTrait,
    sea_query::{Expr, Func, Query},
};
use sha2::{Digest, Sha256};

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

/// The dependent-sharing journey tokens and stored-session secrets used by
/// `tests/browser/care_sharing.rs`.
pub const DEPENDENT_SHARING_TOKEN: &str = "synthetic-dependent-share";
pub const FOREIGN_SHARING_TOKEN: &str = "synthetic-foreign-share";
pub const FOREIGN_BIND_TOKEN: &str = "synthetic-foreign-bind";
pub const GUARDIAN_SESSION_TOKEN: &str = "synthetic-guardian-session";

/// A current guardian declaration for the synthetic dependent adult plus the
/// foreign household's dependent-sharing invitation, manager membership and
/// stored API sessions. Declarations and carer links must run inside the real
/// tenant transaction so the recovery guards see live account/membership GUCs.
pub async fn seed_dependent_sharing(db: &DatabaseConnection) {
    for (id, birthday) in [
        (73003, NaiveDate::from_ymd_opt(1990, 1, 1)),
        (73005, NaiveDate::from_ymd_opt(1982, 1, 1)),
    ] {
        people::ActiveModel {
            id: Set(id),
            date_of_birth: Set(birthday),
            ..Default::default()
        }
        .update(db)
        .await
        .expect("person birth date");
    }
    household_memberships::ActiveModel {
        id: Set(74002),
        household_id: Set(72002),
        person_id: Set(73005),
        role: Set("owner".into()),
        status: Set("active".into()),
        permissions_version: Set(1),
        joined_at: Set(now()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("foreign manager membership");
    for (id, name) in [
        (80001, "Synthetic tablets"),
        (80002, "Synthetic foreign medicine"),
    ] {
        medications::ActiveModel {
            id: Set(id),
            name: Set(Some(name.into())),
            dmd_system: Set(Some("https://dmd.nhs.uk".into())),
            dmd_concept_class: Set(Some("VMP".into())),
            dmd_code: Set(Some("synthetic-sharing-product".into())),
            dose_amount: Set(Some(2.0)),
            dose_unit: Set(Some("tablet".into())),
            ..Default::default()
        }
        .update(db)
        .await
        .expect("dm+d stock identity");
    }
    for (email, token) in [
        ("persistence@example.test", DEPENDENT_SHARING_TOKEN),
        ("foreign@example.test", FOREIGN_SHARING_TOKEN),
    ] {
        household_invitations::ActiveModel {
            household_id: Set(72002),
            email: Set(email.into()),
            membership_role: Set("member".into()),
            dependent_sharing: Set(true),
            token_digest: Set(hex::encode(Sha256::digest(token.as_bytes()))),
            invited_by_membership_id: Set(74002),
            expires_at: Set(now() + Duration::days(7)),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .expect("dependent sharing invitation");
    }
    for (account, membership, token, id) in [
        (71002_i64, 74002_i64, FOREIGN_BIND_TOKEN, 77001_i64),
        (71001, 74001, GUARDIAN_SESSION_TOKEN, 77002),
    ] {
        api_sessions::ActiveModel {
            id: Set(id),
            account_id: Set(account),
            household_membership_id: Set(Some(membership)),
            access_token_digest: Set(hex::encode(Sha256::digest(token.as_bytes()))),
            refresh_token_digest: Set(hex::encode(Sha256::digest(
                format!("{token}-refresh").as_bytes(),
            ))),
            access_expires_at: Set(now() + Duration::hours(1)),
            refresh_expires_at: Set(now() + Duration::days(30)),
            last_used_at: Set(now()),
            permissions_version: Set(1),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .expect("stored api session");
    }
    let scope = access::HouseholdScope {
        actor: access::Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-browser-dependent-sharing".into(),
    };
    let fixture = access::begin(db, &scope, None).await.unwrap();
    let tx = fixture.transaction();
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
            73003_i64.into(),
            73001_i64.into(),
            72001_i64.into(),
            71001_i64.into(),
            "Synthetic dependent adult".into(),
        ])
        .returning_col(GuardianDeclarations::Id)
        .to_owned();
    let declaration_id: i64 = tx
        .query_one(&declaration)
        .await
        .unwrap()
        .unwrap()
        .try_get("", "id")
        .unwrap();
    carer_relationships::ActiveModel {
        carer_id: Set(73001),
        patient_id: Set(73003),
        household_id: Set(72001),
        active: Set(true),
        guardian_declaration_id: Set(Some(declaration_id)),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(tx)
    .await
    .unwrap();
    fixture.commit().await.unwrap();
}

pub async fn seed_household_entrant(db: &DatabaseConnection, invitation: bool) {
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
