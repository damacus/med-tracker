use serde::{Deserialize, Serialize};

macro_rules! identity_id {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(i64);

        impl $name {
            pub const fn from_database(value: i64) -> Self {
                Self(value)
            }

            pub const fn get(self) -> i64 {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

identity_id!(AccountId);
identity_id!(PersonId);
identity_id!(HouseholdId);

identity_id!(HouseholdMembershipId);

identity_id!(MedicationId);
identity_id!(ScheduleId);
identity_id!(AssignmentId);
identity_id!(TakeId);
identity_id!(HistoryEntryId);

identity_id!(LocationId);
