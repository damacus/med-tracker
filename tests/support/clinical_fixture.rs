use crate::scenario::Scenario;
use med_tracker::models::_entities::{locations, medications, person_medications};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, prelude::Decimal};

impl Scenario {
    pub async fn medication(&self, name: &str, supply: Decimal) -> medications::Model {
        let now = chrono::Utc::now().naive_utc();
        let location = locations::ActiveModel {
            household_id: Set(self.household.id),
            name: Set(format!("Synthetic cabinet {}", uuid::Uuid::new_v4())),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&self.admin)
        .await
        .unwrap();
        medications::ActiveModel {
            household_id: Set(self.household.id),
            location_id: Set(location.id),
            created_by_membership_id: Set(Some(self.membership.id)),
            name: Set(Some(name.into())),
            current_supply: Set(Some(supply)),
            dose_amount: Set(Some(2.0)),
            dose_unit: Set(Some("tablet".into())),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&self.admin)
        .await
        .unwrap()
    }

    pub async fn personal_assignment(
        &self,
        medication_id: i64,
        position: i32,
    ) -> person_medications::Model {
        let now = chrono::Utc::now().naive_utc();
        person_medications::ActiveModel {
            household_id: Set(self.household.id),
            person_id: Set(self.account.person_id),
            medication_id: Set(medication_id),
            dose_amount: Set(Some(Decimal::new(2, 0))),
            dose_unit: Set(Some("tablet".into())),
            position: Set(position),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&self.admin)
        .await
        .unwrap()
    }
}
