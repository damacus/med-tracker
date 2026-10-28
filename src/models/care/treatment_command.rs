use uuid::Uuid;

macro_rules! selector {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub(crate) struct $name(String);

        impl std::str::FromStr for $name {
            type Err = ();

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                if value.parse::<i64>().is_ok_and(|number| number > 0)
                    || Uuid::parse_str(value).is_ok()
                {
                    Ok(Self(value.to_owned()))
                } else {
                    Err(())
                }
            }
        }

        impl $name {
            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

selector!(PersonId);
selector!(MedicationId);
selector!(DosageOptionId);

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DoseCycle {
    Daily,
    Weekly,
    Monthly,
}

impl DoseCycle {
    pub(crate) fn from_database(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Daily),
            1 => Some(Self::Weekly),
            2 => Some(Self::Monthly),
            _ => None,
        }
    }
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "daily" => Some(Self::Daily),
            "weekly" => Some(Self::Weekly),
            "monthly" => Some(Self::Monthly),
            _ => None,
        }
    }

    pub(crate) const fn database(self) -> i32 {
        match self {
            Self::Daily => 0,
            Self::Weekly => 1,
            Self::Monthly => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AdministrationKind {
    Routine,
    AsNeeded,
}

impl AdministrationKind {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "routine" => Some(Self::Routine),
            "as_needed" => Some(Self::AsNeeded),
            _ => None,
        }
    }

    pub(crate) const fn database(self) -> i32 {
        match self {
            Self::Routine => 0,
            Self::AsNeeded => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ScheduleType {
    Daily,
    MultipleDaily,
    Weekly,
    SpecificDates,
    #[serde(rename = "prn")]
    AsNeeded,
    Tapering,
    EveryOtherDay,
}

impl ScheduleType {
    pub(crate) fn from_database(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Daily),
            1 => Some(Self::MultipleDaily),
            2 => Some(Self::Weekly),
            3 => Some(Self::SpecificDates),
            4 => Some(Self::AsNeeded),
            5 => Some(Self::Tapering),
            6 => Some(Self::EveryOtherDay),
            _ => None,
        }
    }
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "daily" => Some(Self::Daily),
            "multiple_daily" => Some(Self::MultipleDaily),
            "weekly" => Some(Self::Weekly),
            "specific_dates" => Some(Self::SpecificDates),
            "prn" => Some(Self::AsNeeded),
            "tapering" => Some(Self::Tapering),
            "every_other_day" => Some(Self::EveryOtherDay),
            _ => None,
        }
    }

    pub(crate) const fn database(self) -> i32 {
        match self {
            Self::Daily => 0,
            Self::MultipleDaily => 1,
            Self::Weekly => 2,
            Self::SpecificDates => 3,
            Self::AsNeeded => 4,
            Self::Tapering => 5,
            Self::EveryOtherDay => 6,
        }
    }
}
