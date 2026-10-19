use super::entities::*;

impl loco_rs::model::TenantEntity for api_change_event::Entity {
    type TenantId = i64;

    fn tenant_column() -> api_change_event::Column {
        api_change_event::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for api_idempotency_key::Entity {
    type TenantId = i64;

    fn tenant_column() -> api_idempotency_key::Column {
        api_idempotency_key::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for api_tombstone::Entity {
    type TenantId = i64;

    fn tenant_column() -> api_tombstone::Column {
        api_tombstone::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for carer_relationship::Entity {
    type TenantId = i64;

    fn tenant_column() -> carer_relationship::Column {
        carer_relationship::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for dosage::Entity {
    type TenantId = i64;

    fn tenant_column() -> dosage::Column {
        dosage::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for dose_occurrence::Entity {
    type TenantId = i64;

    fn tenant_column() -> dose_occurrence::Column {
        dose_occurrence::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for grant::Entity {
    type TenantId = i64;

    fn tenant_column() -> grant::Column {
        grant::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for health_event::Entity {
    type TenantId = i64;

    fn tenant_column() -> health_event::Column {
        health_event::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for health_event_medication::Entity {
    type TenantId = i64;

    fn tenant_column() -> health_event_medication::Column {
        health_event_medication::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for household_invitation::Entity {
    type TenantId = i64;

    fn tenant_column() -> household_invitation::Column {
        household_invitation::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for household_invitation_grant::Entity {
    type TenantId = i64;

    fn tenant_column() -> household_invitation_grant::Column {
        household_invitation_grant::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for location::Entity {
    type TenantId = i64;

    fn tenant_column() -> location::Column {
        location::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for location_membership::Entity {
    type TenantId = i64;

    fn tenant_column() -> location_membership::Column {
        location_membership::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for medication::Entity {
    type TenantId = i64;

    fn tenant_column() -> medication::Column {
        medication::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for medication_take::Entity {
    type TenantId = i64;

    fn tenant_column() -> medication_take::Column {
        medication_take::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for membership::Entity {
    type TenantId = i64;

    fn tenant_column() -> membership::Column {
        membership::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for notification_preference::Entity {
    type TenantId = i64;

    fn tenant_column() -> notification_preference::Column {
        notification_preference::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for pause_period::Entity {
    type TenantId = i64;

    fn tenant_column() -> pause_period::Column {
        pause_period::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for person_medication::Entity {
    type TenantId = i64;

    fn tenant_column() -> person_medication::Column {
        person_medication::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for review_prompt::Entity {
    type TenantId = i64;

    fn tenant_column() -> review_prompt::Column {
        review_prompt::Column::HouseholdId
    }
}

impl loco_rs::model::TenantEntity for schedule::Entity {
    type TenantId = i64;

    fn tenant_column() -> schedule::Column {
        schedule::Column::HouseholdId
    }
}
