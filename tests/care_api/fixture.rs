use crate::scenario::Scenario;
use med_tracker::models::_entities::{
    api_change_events, household_memberships, households, locations, medication_takes, medications,
    oauth_applications, oauth_grants, people, person_access_grants, person_medications, users,
    versions,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, prelude::Decimal,
};

pub struct Fixture {
    pub scenario: Scenario,
    pub runtime_uri: String,
    pub adult: people::Model,
    pub user: users::Model,
    pub minor: people::Model,
    pub dependent: people::Model,
    pub minor_membership: household_memberships::Model,
    pub dependent_membership: household_memberships::Model,
    pub location: locations::Model,
    pub medication: medications::Model,
    pub assignment: person_medications::Model,
    pub grant: person_access_grants::Model,
    pub application: oauth_applications::Model,
    pub authorization: oauth_grants::Model,
    pub client_id: String,
    pub code: String,
    pub client_uuid: uuid::Uuid,
}

impl std::ops::Deref for Fixture {
    type Target = Scenario;
    fn deref(&self) -> &Self::Target {
        &self.scenario
    }
}

impl Fixture {
    pub async fn new() -> Self {
        let mut scenario = Scenario::new().await;
        let mut membership = scenario.membership.clone().into_active_model();
        membership.role = Set("administrator".into());
        scenario.membership = membership.update(&scenario.admin).await.unwrap();
        let now = chrono::Utc::now().naive_utc();
        let adult = people::Entity::find_by_id(scenario.account.person_id)
            .one(&scenario.admin)
            .await
            .unwrap()
            .unwrap();
        let user = users::Entity::find()
            .filter(users::Column::PersonId.eq(adult.id))
            .one(&scenario.admin)
            .await
            .unwrap()
            .unwrap();
        let mut participants = Vec::new();
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
            let membership = household_memberships::ActiveModel {
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
            participants.push((person, membership));
        }
        let [(minor, minor_membership), (dependent, dependent_membership)]: [(
            people::Model,
            household_memberships::Model,
        ); 2] = participants.try_into().unwrap();
        let grant = person_access_grants::ActiveModel {
            household_id: Set(scenario.household.id),
            household_membership_id: Set(scenario.membership.id),
            person_id: Set(adult.id),
            access_level: Set("manage".into()),
            relationship_type: Set("self".into()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&scenario.admin)
        .await
        .unwrap();
        let medication = scenario
            .medication("Synthetic tablets", Decimal::new(10, 0))
            .await;
        let location = locations::Entity::find_by_id(medication.location_id)
            .one(&scenario.admin)
            .await
            .unwrap()
            .unwrap();
        let assignment = scenario.personal_assignment(medication.id, 0).await;
        let client_id = format!("native-{}", uuid::Uuid::new_v4());
        let code = uuid::Uuid::new_v4().to_string();
        let application = oauth_applications::ActiveModel {
            account_id: Set(Some(scenario.account.id)),
            client_id: Set(client_id.clone()),
            client_kind: Set("mobile".into()),
            name: Set("Synthetic native client".into()),
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
            code: Set(Some(code.clone())),
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
        let runtime_uri = std::env::var("DATABASE_URL").unwrap();
        Self {
            scenario,
            runtime_uri,
            adult,
            user,
            minor,
            dependent,
            minor_membership,
            dependent_membership,
            location,
            medication,
            assignment,
            grant,
            application,
            authorization,
            client_id,
            code,
            client_uuid: uuid::Uuid::new_v4(),
        }
    }

    pub async fn close(self) {
        self.scenario.close().await;
    }

    pub async fn foreign_household(&self) -> Scenario {
        Scenario::new().await
    }

    pub async fn seed_foreign_household(&self, _legacy_id: i64) -> households::Model {
        // Unmigrated callers must use the returned generated ID, not the legacy argument.
        let foreign = self.foreign_household().await;
        let household = foreign.household.clone();
        foreign.close().await;
        household
    }

    pub async fn effect(&self) -> (i64, String, i64, i64) {
        let household_id = self.scenario.household.id;
        let takes = medication_takes::Entity::find()
            .filter(medication_takes::Column::HouseholdId.eq(household_id))
            .count(&self.scenario.admin)
            .await
            .unwrap();
        let medication = medications::Entity::find_by_id(self.medication.id)
            .one(&self.scenario.admin)
            .await
            .unwrap()
            .unwrap();
        let audits = versions::Entity::find()
            .filter(versions::Column::HouseholdId.eq(household_id))
            .filter(versions::Column::ItemType.eq("MedicationTake"))
            .filter(versions::Column::Event.eq("create"))
            .count(&self.scenario.admin)
            .await
            .unwrap();
        let changes = api_change_events::Entity::find()
            .filter(api_change_events::Column::HouseholdId.eq(household_id))
            .filter(api_change_events::Column::RecordType.eq("MedicationTake"))
            .filter(api_change_events::Column::Action.eq("create"))
            .count(&self.scenario.admin)
            .await
            .unwrap();
        (
            i64::try_from(takes).unwrap(),
            format!("{:.2}", medication.current_supply.unwrap()),
            i64::try_from(audits).unwrap(),
            i64::try_from(changes).unwrap(),
        )
    }

    pub async fn versions(&self) -> Vec<versions::Model> {
        versions::Entity::find()
            .filter(
                Condition::any()
                    .add(versions::Column::HouseholdId.eq(self.scenario.household.id))
                    .add(
                        Condition::all()
                            .add(versions::Column::ItemType.is_in([
                                "NativeAuthentication",
                                "Account",
                                "AuthenticationToken",
                            ]))
                            .add(versions::Column::ItemId.eq(self.scenario.account.id)),
                    ),
            )
            .all(&self.scenario.admin)
            .await
            .unwrap()
    }

    pub async fn revoke(&self) {
        let now = chrono::Utc::now().naive_utc();
        let mut grant = self.grant.clone().into_active_model();
        grant.revoked_at = Set(Some(now));
        grant.update(&self.scenario.admin).await.unwrap();
        let mut membership = self.scenario.membership.clone().into_active_model();
        membership.revoked_at = Set(Some(now));
        membership.update(&self.scenario.admin).await.unwrap();
    }

    pub async fn member(&self) {
        let mut membership = self.scenario.membership.clone().into_active_model();
        membership.role = Set("member".into());
        membership.update(&self.scenario.admin).await.unwrap();
    }
}
