use chrono::{FixedOffset, Utc};
use med_tracker::models::_entities::{
    accounts, household_memberships, households, identity_onboarding, locations, medications,
    oauth_applications, oauth_grants, people, person_access_grants, person_medications, users,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, IntoActiveModel,
    entity::prelude::Decimal,
};

fn now() -> chrono::NaiveDateTime {
    Utc::now().naive_utc()
}

#[allow(dead_code)]
fn now_tz() -> chrono::DateTime<FixedOffset> {
    Utc::now().fixed_offset()
}

fn password_hash() -> String {
    bcrypt::hash("password", 4).expect("fixture password hash")
}

/// Equivalent of tests/fixtures/persistence-records.sql and care-doses.sql.
pub async fn seed_core(db: &DatabaseConnection) {
    people::Entity::insert_many([
        people::ActiveModel {
            id: Set(73001),
            name: Set("Synthetic adult".into()),
            person_type: Set(0),
            has_capacity: Set(true),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        },
        people::ActiveModel {
            id: Set(73002),
            name: Set("Synthetic minor".into()),
            person_type: Set(1),
            has_capacity: Set(false),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        },
        people::ActiveModel {
            id: Set(73003),
            name: Set("Synthetic dependent adult".into()),
            person_type: Set(2),
            has_capacity: Set(false),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        },
    ])
    .exec(db)
    .await
    .expect("seed people");

    accounts::Entity::insert(accounts::ActiveModel {
        id: Set(71001),
        person_id: Set(73001),
        email: Set("persistence@example.test".into()),
        password_hash: Set(Some(password_hash())),
        status: Set(2),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed account");

    households::Entity::insert(households::ActiveModel {
        id: Set(72001),
        created_by_account_id: Set(Some(71001)),
        name: Set("Synthetic household".into()),
        slug: Set("persistence-fixture".into()),
        timezone: Set("Europe/London".into()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed household");

    household_memberships::Entity::insert_many([
        household_memberships::ActiveModel {
            id: Set(74001),
            household_id: Set(72001),
            person_id: Set(73001),
            role: Set("administrator".into()),
            joined_at: Set(now()),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        },
        household_memberships::ActiveModel {
            id: Set(74980),
            household_id: Set(72001),
            person_id: Set(73002),
            role: Set("member".into()),
            joined_at: Set(now()),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        },
        household_memberships::ActiveModel {
            id: Set(74981),
            household_id: Set(72001),
            person_id: Set(73003),
            role: Set("member".into()),
            joined_at: Set(now()),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        },
    ])
    .exec(db)
    .await
    .expect("seed memberships");

    oauth_applications::Entity::insert(oauth_applications::ActiveModel {
        id: Set(75001),
        account_id: Set(Some(71001)),
        client_id: Set("synthetic-client".into()),
        client_kind: Set("integration".into()),
        name: Set("Synthetic client".into()),
        redirect_uri: Set("https://example.test/callback".into()),
        scopes: Set("patient/*.read".into()),
        token_endpoint_auth_method: Set("none".into()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed oauth application");

    oauth_grants::Entity::insert(oauth_grants::ActiveModel {
        id: Set(76001),
        account_id: Set(71001),
        oauth_application_id: Set(75001),
        household_membership_id: Set(Some(74001)),
        person_id: Set(Some(73001)),
        permissions_version: Set(Some(1)),
        code: Set(Some("synthetic-code".into())),
        token: Set(Some("synthetic-token".into())),
        token_hash: Set(Some("synthetic-token-hash".into())),
        refresh_token: Set(Some("synthetic-refresh".into())),
        refresh_token_hash: Set(Some("synthetic-refresh-hash".into())),
        scopes: Set("patient/*.read".into()),
        expires_in: Set(now() + chrono::Duration::hours(1)),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed oauth grant");

    users::Entity::insert(users::ActiveModel {
        id: Set(77001),
        person_id: Set(73001),
        email_address: Set("persistence@example.test".into()),
        password_digest: Set(Some(password_hash())),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed user");

    person_access_grants::Entity::insert(person_access_grants::ActiveModel {
        id: Set(78001),
        household_id: Set(72001),
        household_membership_id: Set(74001),
        person_id: Set(73001),
        access_level: Set("manage".into()),
        relationship_type: Set("self".into()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed access grant");

    locations::Entity::insert(locations::ActiveModel {
        id: Set(79001),
        household_id: Set(72001),
        name: Set("Synthetic cabinet".into()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed location");

    medications::Entity::insert(medications::ActiveModel {
        id: Set(80001),
        household_id: Set(72001),
        location_id: Set(79001),
        name: Set(Some("Synthetic tablets".into())),
        current_supply: Set(Some(Decimal::new(10, 0))),
        dose_amount: Set(Some(2.0)),
        dose_unit: Set(Some("tablet".into())),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed medication");

    person_medications::Entity::insert(person_medications::ActiveModel {
        id: Set(81001),
        household_id: Set(72001),
        person_id: Set(73001),
        medication_id: Set(80001),
        dose_amount: Set(Some(Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".into())),
        position: Set(0),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed person medication");
}

/// The care API suite exchanges its authorization code against client id
/// `native`; repoint application 75001 after `seed_core` created it as
/// `synthetic-client`.
#[allow(dead_code)]
pub async fn set_native_client_id(db: &DatabaseConnection) {
    let application = oauth_applications::Entity::find_by_id(75001)
        .one(db)
        .await
        .expect("find oauth application")
        .expect("oauth application 75001");
    let mut active = application.into_active_model();
    active.client_id = Set("native".into());
    active.update(db).await.expect("set native client id");
}

/// Equivalent of tests/oauth_server/clients.sql.
#[allow(dead_code)]
pub async fn seed_oauth_clients(db: &DatabaseConnection) {
    oauth_applications::Entity::insert_many([
        oauth_applications::ActiveModel {
            id: Set(75002),
            account_id: Set(Some(71001)),
            client_id: Set("native-journey".into()),
            client_kind: Set("mobile".into()),
            name: Set("Synthetic native journey".into()),
            redirect_uri: Set("io.damacus.medtracker:/oauth2redirect".into()),
            scopes: Set("medtracker offline_access".into()),
            token_endpoint_auth_method: Set("none".into()),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        },
        oauth_applications::ActiveModel {
            id: Set(75003),
            account_id: Set(Some(71001)),
            client_id: Set("smart-journey".into()),
            client_kind: Set("integration".into()),
            name: Set("Synthetic SMART journey".into()),
            redirect_uri: Set("https://example.test/callback?tenant=7".into()),
            scopes: Set("launch/patient patient/*.rs offline_access".into()),
            token_endpoint_auth_method: Set("client_secret_post".into()),
            client_secret_hash: Set(Some(password_hash())),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        },
    ])
    .exec(db)
    .await
    .expect("seed oauth clients");
}

/// Equivalent of src/fixtures/browser.sql.
#[allow(dead_code)]
pub async fn seed_browser_rows(db: &DatabaseConnection) {
    people::Entity::insert(people::ActiveModel {
        id: Set(73005),
        name: Set("Synthetic foreign adult".into()),
        person_type: Set(0),
        has_capacity: Set(true),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed foreign person");

    accounts::Entity::insert(accounts::ActiveModel {
        id: Set(71002),
        person_id: Set(73005),
        email: Set("foreign@example.test".into()),
        password_hash: Set(Some(password_hash())),
        status: Set(2),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed foreign account");

    identity_onboarding::Entity::insert_many([
        identity_onboarding::ActiveModel {
            account_id: Set(71001),
            recovery_saved_at: Set(Some(now_tz())),
            ..Default::default()
        },
        identity_onboarding::ActiveModel {
            account_id: Set(71002),
            recovery_saved_at: Set(Some(now_tz())),
            ..Default::default()
        },
    ])
    .exec(db)
    .await
    .expect("seed onboarding");

    households::Entity::insert(households::ActiveModel {
        id: Set(72002),
        created_by_account_id: Set(Some(71002)),
        name: Set("Synthetic foreign household".into()),
        slug: Set("foreign-fixture".into()),
        timezone: Set("Europe/London".into()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed foreign household");

    locations::Entity::insert(locations::ActiveModel {
        id: Set(79002),
        household_id: Set(72002),
        name: Set("Synthetic foreign cabinet".into()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed foreign location");

    medications::Entity::insert(medications::ActiveModel {
        id: Set(80002),
        household_id: Set(72002),
        location_id: Set(79002),
        name: Set(Some("Synthetic foreign medicine".into())),
        current_supply: Set(Some(Decimal::new(10, 0))),
        dose_amount: Set(Some(2.0)),
        dose_unit: Set(Some("tablet".into())),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed foreign medication");
}

/// Foreign household used by care_api isolation tests: person 73999,
/// account 71999, a caller-owned household and an owner membership.
#[allow(dead_code)]
pub async fn seed_foreign_household(db: &DatabaseConnection, household_id: i64) {
    people::Entity::insert(people::ActiveModel {
        id: Set(73999),
        name: Set("Other care member".into()),
        person_type: Set(0),
        has_capacity: Set(true),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed foreign care person");

    accounts::Entity::insert(accounts::ActiveModel {
        id: Set(71999),
        person_id: Set(73999),
        email: Set("other-care-household@example.test".into()),
        password_hash: Set(Some(password_hash())),
        status: Set(2),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed foreign care account");

    households::Entity::insert(households::ActiveModel {
        id: Set(household_id),
        created_by_account_id: Set(Some(71999)),
        name: Set("Other care household".into()),
        slug: Set(format!("other-care-{household_id}")),
        timezone: Set("UTC".into()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed foreign care household");

    household_memberships::Entity::insert(household_memberships::ActiveModel {
        id: Set(74999),
        household_id: Set(household_id),
        person_id: Set(73999),
        role: Set("owner".into()),
        joined_at: Set(now()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed foreign care membership");
}

/// Per-journey barcode metadata for the scanner hit flow (the old
/// browser-care.yml scanner-barcode task set medications.barcode for 80001).
#[allow(dead_code)]
pub async fn seed_barcode_metadata(db: &DatabaseConnection) {
    let medication = medications::Entity::find_by_id(80001)
        .one(db)
        .await
        .expect("find medication")
        .expect("medication 80001");
    let mut active = medication.into_active_model();
    active.barcode = Set(Some("1234567890123".into()));
    active.update(db).await.expect("set medication barcode");
}

/// Equivalent of tests/fixtures/identity/mobile.sql: swap the integration
/// grant for a mobile grant on the same ids.
#[allow(dead_code)]
pub async fn seed_mobile_grant(db: &DatabaseConnection) {
    oauth_grants::Entity::delete_by_id(76001)
        .exec(db)
        .await
        .expect("delete integration grant");

    let application = oauth_applications::Entity::find_by_id(75001)
        .one(db)
        .await
        .expect("find oauth application")
        .expect("oauth application 75001");
    let mut active = application.into_active_model();
    active.client_kind = Set("mobile".into());
    active.redirect_uri = Set("io.damacus.medtracker:/oauth2redirect".into());
    active.scopes = Set("medtracker offline_access".into());
    active.update(db).await.expect("switch client to mobile");

    oauth_grants::Entity::insert(oauth_grants::ActiveModel {
        id: Set(76001),
        account_id: Set(71001),
        oauth_application_id: Set(75001),
        client_kind: Set("mobile".into()),
        code: Set(Some("synthetic-legacy-code".into())),
        code_challenge: Set(Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".into())),
        code_challenge_method: Set(Some("S256".into())),
        redirect_uri: Set(Some("io.damacus.medtracker:/oauth2redirect".into())),
        scopes: Set("medtracker offline_access".into()),
        expires_in: Set(now() + chrono::Duration::minutes(5)),
        authenticated_at: Set(Some(now() - chrono::Duration::days(2))),
        last_used_at: Set(Some(now() - chrono::Duration::days(2))),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    })
    .exec(db)
    .await
    .expect("seed mobile grant");
}
