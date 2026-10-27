use super::*;
use med_tracker::models::_entities::{
    api_app_tokens, api_change_events, api_sessions, household_memberships, medication_takes,
    medications, oauth_applications, oauth_grants, people, person_access_grants, versions,
};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, Set,
};

pub enum CredentialKind {
    Session,
    App,
    Mobile,
}

pub struct RequestFixture {
    pub foreign: Scenario,
    pub user: med_tracker::models::_entities::users::Model,
    pub runtime_uri: String,
    pub scenario: Scenario,
    pub medication: medications::Model,
    pub grant: person_access_grants::Model,
    pub minor: people::Model,
    pub dependent: people::Model,
    pub application: oauth_applications::Model,
    pub authorization: oauth_grants::Model,
}

impl std::ops::Deref for RequestFixture {
    type Target = Scenario;
    fn deref(&self) -> &Scenario {
        &self.scenario
    }
}

impl RequestFixture {
    pub async fn new() -> Self {
        let mut scenario = Scenario::new().await;
        let mut household = scenario.household.clone().into_active_model();
        household.timezone = Set("Europe/London".into());
        scenario.household = household.update(&scenario.admin).await.unwrap();
        let foreign = Scenario::new().await;
        let medication = scenario
            .medication("Synthetic tablets", Decimal::new(10, 0))
            .await;
        scenario.personal_assignment(medication.id, 0).await;
        let now = chrono::Utc::now().naive_utc();
        let mut dependants = Vec::new();
        for (name, person_type) in [("Synthetic minor", 1), ("Synthetic dependent adult", 2)] {
            let person = people::ActiveModel {
                name: Set(name.into()),
                person_type: Set(person_type),
                has_capacity: Set(false),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&scenario.admin)
            .await
            .unwrap();
            household_memberships::ActiveModel {
                household_id: Set(scenario.household.id),
                person_id: Set(person.id),
                role: Set("member".into()),
                status: Set("active".into()),
                joined_at: Set(now),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&scenario.admin)
            .await
            .unwrap();
            dependants.push(person);
        }
        let grant = person_access_grants::ActiveModel {
            household_id: Set(scenario.household.id),
            household_membership_id: Set(scenario.membership.id),
            person_id: Set(scenario.account.person_id),
            access_level: Set("manage".into()),
            relationship_type: Set("self".into()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&scenario.admin)
        .await
        .unwrap();
        let application = oauth_applications::ActiveModel {
            account_id: Set(Some(scenario.account.id)),
            client_id: Set(format!("native-{}", uuid::Uuid::new_v4())),
            client_kind: Set("mobile".into()),
            name: Set("Synthetic native".into()),
            redirect_uri: Set("io.damacus.medtracker:/oauth2redirect".into()),
            scopes: Set("medtracker offline_access".into()),
            token_endpoint_auth_method: Set("none".into()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&scenario.admin)
        .await
        .unwrap();
        let authorization = oauth_grants::ActiveModel {
            account_id: Set(scenario.account.id),
            oauth_application_id: Set(application.id),
            client_kind: Set("mobile".into()),
            code: Set(Some(uuid::Uuid::new_v4().to_string())),
            code_challenge: Set(Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".into())),
            code_challenge_method: Set(Some("S256".into())),
            redirect_uri: Set(Some(application.redirect_uri.clone())),
            scopes: Set(application.scopes.clone()),
            expires_in: Set(now + chrono::Duration::minutes(5)),
            authenticated_at: Set(Some(now - chrono::Duration::days(2))),
            last_used_at: Set(Some(now - chrono::Duration::days(2))),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&scenario.admin)
        .await
        .unwrap();
        let dependent = dependants.pop().unwrap();
        let minor = dependants.pop().unwrap();
        let user = med_tracker::models::_entities::users::Entity::find()
            .filter(
                med_tracker::models::_entities::users::Column::PersonId
                    .eq(scenario.account.person_id),
            )
            .one(&scenario.admin)
            .await
            .unwrap()
            .unwrap();
        Self {
            user,
            runtime_uri: std::env::var("DATABASE_URL").unwrap(),
            foreign,
            scenario,
            medication,
            grant,
            minor,
            dependent,
            application,
            authorization,
        }
    }

    pub async fn effect(&self) -> (u64, String, u64, u64) {
        let takes = medication_takes::Entity::find()
            .filter(medication_takes::Column::HouseholdId.eq(self.household.id))
            .count(&self.admin)
            .await
            .unwrap();
        let medication = medications::Entity::find_by_id(self.medication.id)
            .one(&self.admin)
            .await
            .unwrap()
            .unwrap();
        let audits = versions::Entity::find()
            .filter(versions::Column::HouseholdId.eq(self.household.id))
            .filter(versions::Column::ItemType.eq("MedicationTake"))
            .filter(versions::Column::Event.eq("create"))
            .count(&self.admin)
            .await
            .unwrap();
        let changes = api_change_events::Entity::find()
            .filter(api_change_events::Column::HouseholdId.eq(self.household.id))
            .filter(api_change_events::Column::RecordType.eq("MedicationTake"))
            .filter(api_change_events::Column::Action.eq("create"))
            .count(&self.admin)
            .await
            .unwrap();
        (
            takes,
            format!("{:.2}", medication.current_supply.unwrap()),
            audits,
            changes,
        )
    }

    pub async fn session(&self, token: &str) -> api_sessions::Model {
        let now = chrono::Utc::now().naive_utc();
        api_sessions::ActiveModel {
            account_id: Set(self.account.id),
            household_membership_id: Set(Some(self.membership.id)),
            access_token_digest: Set(hex::encode(Sha256::digest(token.as_bytes()))),
            refresh_token_digest: Set(uuid::Uuid::new_v4().to_string()),
            permissions_version: Set(self.membership.permissions_version),
            access_expires_at: Set(now + chrono::Duration::minutes(15)),
            refresh_expires_at: Set(now + chrono::Duration::days(1)),
            last_used_at: Set(now),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&self.admin)
        .await
        .unwrap()
    }

    pub async fn app_token(&self, token: &str) -> api_app_tokens::Model {
        let now = chrono::Utc::now().naive_utc();
        api_app_tokens::ActiveModel {
            account_id: Set(self.account.id),
            household_membership_id: Set(self.membership.id),
            name: Set("Synthetic app".into()),
            token_digest: Set(hex::encode(Sha256::digest(token.as_bytes()))),
            permissions_version: Set(self.membership.permissions_version),
            expires_at: Set(now + chrono::Duration::days(30)),
            last_used_at: Set(now),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&self.admin)
        .await
        .unwrap()
    }

    pub async fn session_id(&self, label: &str) -> i64 {
        api_sessions::Entity::find()
            .filter(api_sessions::Column::AccountId.eq(self.account.id))
            .filter(
                api_sessions::Column::AccessTokenDigest.eq(hex::encode(Sha256::digest(
                    self.credential(label).as_bytes(),
                ))),
            )
            .one(&self.admin)
            .await
            .unwrap()
            .unwrap()
            .id
    }
    pub async fn app_token_id(&self, label: &str) -> i64 {
        api_app_tokens::Entity::find()
            .filter(api_app_tokens::Column::AccountId.eq(self.account.id))
            .filter(
                api_app_tokens::Column::TokenDigest.eq(hex::encode(Sha256::digest(
                    self.credential(label).as_bytes(),
                ))),
            )
            .one(&self.admin)
            .await
            .unwrap()
            .unwrap()
            .id
    }
    pub async fn member(&self) {
        use sea_orm::sea_query::Expr;
        household_memberships::Entity::update_many()
            .col_expr(household_memberships::Column::Role, Expr::value("member"))
            .filter(household_memberships::Column::Id.eq(self.membership.id))
            .exec(&self.admin)
            .await
            .unwrap();
    }

    pub async fn birth(&self, birth: Option<chrono::NaiveDate>) {
        use sea_orm::sea_query::Expr;
        people::Entity::update_many()
            .col_expr(people::Column::DateOfBirth, Expr::value(birth))
            .filter(people::Column::Id.eq(self.account.person_id))
            .exec(&self.admin)
            .await
            .unwrap();
    }

    pub async fn membership_state(&self, version: i32, revoked: bool) {
        use sea_orm::sea_query::Expr;
        household_memberships::Entity::update_many()
            .col_expr(
                household_memberships::Column::PermissionsVersion,
                Expr::value(version),
            )
            .col_expr(
                household_memberships::Column::RevokedAt,
                Expr::value(revoked.then(|| chrono::Utc::now().naive_utc())),
            )
            .filter(household_memberships::Column::Id.eq(self.membership.id))
            .exec(&self.admin)
            .await
            .unwrap();
    }

    pub async fn revoke_all(&self) {
        self.revoke(CredentialKind::Session, true).await;
        self.revoke(CredentialKind::App, true).await;
        self.revoke(CredentialKind::Mobile, true).await;
    }

    pub async fn view_grant(&self) {
        use sea_orm::sea_query::Expr;
        person_access_grants::Entity::update_many()
            .col_expr(
                person_access_grants::Column::AccessLevel,
                Expr::value("view"),
            )
            .filter(person_access_grants::Column::Id.eq(self.grant.id))
            .exec(&self.admin)
            .await
            .unwrap();
    }

    pub async fn mobile_login(
        &self,
        label: &str,
        authentication_days: i64,
        inactivity_days: i64,
        access_expired: bool,
    ) -> oauth_grants::Model {
        use base64::Engine as _;
        let now = chrono::Utc::now().naive_utc();
        oauth_grants::ActiveModel {
            account_id: Set(self.account.id),
            oauth_application_id: Set(self.application.id),
            client_kind: Set("mobile".into()),
            redirect_uri: Set(Some(self.application.redirect_uri.clone())),
            scopes: Set(self.application.scopes.clone()),
            token_hash: Set(Some(
                base64::engine::general_purpose::URL_SAFE
                    .encode(Sha256::digest(self.credential(label).as_bytes())),
            )),
            expires_in: Set(now + chrono::Duration::minutes(if access_expired { -1 } else { 15 })),
            authenticated_at: Set(Some(now - chrono::Duration::days(authentication_days))),
            last_used_at: Set(Some(now - chrono::Duration::days(inactivity_days))),
            created_at: Set(now - chrono::Duration::days(authentication_days)),
            updated_at: Set(now),
            device_name: Set(Some(label.into())),
            ..Default::default()
        }
        .insert(&self.admin)
        .await
        .unwrap()
    }

    pub async fn integration(&self, token: &str) -> oauth_grants::Model {
        use base64::Engine as _;
        let now = chrono::Utc::now().naive_utc();
        let application = oauth_applications::ActiveModel {
            account_id: Set(Some(self.account.id)),
            client_id: Set(format!("integration-{}", uuid::Uuid::new_v4())),
            client_kind: Set("integration".into()),
            name: Set("Synthetic integration".into()),
            redirect_uri: Set("https://example.test/callback?tenant=7".into()),
            scopes: Set("launch/patient patient/*.rs offline_access".into()),
            token_endpoint_auth_method: Set("client_secret_post".into()),
            client_secret_hash: Set(Some(bcrypt::hash("password", 4).unwrap())),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&self.admin)
        .await
        .unwrap();
        oauth_grants::ActiveModel {
            account_id: Set(self.account.id),
            oauth_application_id: Set(application.id),
            client_kind: Set("integration".into()),
            household_membership_id: Set(Some(self.membership.id)),
            person_id: Set(Some(self.account.person_id)),
            permissions_version: Set(Some(self.membership.permissions_version)),
            redirect_uri: Set(Some(application.redirect_uri)),
            scopes: Set(application.scopes),
            token_hash: Set(Some(
                base64::engine::general_purpose::URL_SAFE.encode(Sha256::digest(token.as_bytes())),
            )),
            expires_in: Set(now + chrono::Duration::minutes(15)),
            authenticated_at: Set(Some(now)),
            last_used_at: Set(Some(now)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&self.admin)
        .await
        .unwrap()
    }

    pub async fn revoke(&self, kind: CredentialKind, revoked: bool) {
        use sea_orm::sea_query::Expr;
        let at = revoked.then(|| chrono::Utc::now().naive_utc());
        match kind {
            CredentialKind::Session => {
                api_sessions::Entity::update_many()
                    .col_expr(api_sessions::Column::RevokedAt, Expr::value(at))
                    .filter(api_sessions::Column::AccountId.eq(self.account.id))
                    .exec(&self.admin)
                    .await
                    .unwrap();
            }
            CredentialKind::App => {
                api_app_tokens::Entity::update_many()
                    .col_expr(api_app_tokens::Column::RevokedAt, Expr::value(at))
                    .filter(api_app_tokens::Column::AccountId.eq(self.account.id))
                    .exec(&self.admin)
                    .await
                    .unwrap();
            }
            CredentialKind::Mobile => {
                oauth_grants::Entity::update_many()
                    .col_expr(oauth_grants::Column::RevokedAt, Expr::value(at))
                    .filter(oauth_grants::Column::Id.eq(self.authorization.id))
                    .exec(&self.admin)
                    .await
                    .unwrap();
            }
        }
    }

    pub async fn wrong_scope(&self) {
        use sea_orm::sea_query::Expr;
        oauth_grants::Entity::update_many()
            .col_expr(
                oauth_grants::Column::AccessTokenScopes,
                Expr::value("patient/*.read"),
            )
            .col_expr(
                oauth_grants::Column::AccessTokenScopeHash,
                Expr::col(oauth_grants::Column::TokenHash),
            )
            .filter(oauth_grants::Column::Id.eq(self.authorization.id))
            .exec(&self.admin)
            .await
            .unwrap();
    }

    pub fn credential(&self, label: &str) -> String {
        format!("{label}-{}", self.account.id)
    }

    pub async fn token(&self) -> String {
        let request = axum::http::Request::builder()
            .method("POST")
            .uri("/token")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(axum::body::Body::from(
                serde_urlencoded::to_string([
                    ("grant_type", "authorization_code"),
                    ("client_id", self.application.client_id.as_str()),
                    ("code", self.authorization.code.as_deref().unwrap()),
                    ("redirect_uri", self.application.redirect_uri.as_str()),
                    (
                        "code_verifier",
                        "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
                    ),
                ])
                .unwrap(),
            ))
            .unwrap();
        med_tracker::models::identity::exchange(&self.runtime, request, None)
            .await
            .unwrap()["access_token"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    pub async fn close(self) {
        self.foreign.close().await;
        self.scenario.close().await;
    }
}
