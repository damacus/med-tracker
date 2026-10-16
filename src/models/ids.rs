#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AccountId(i64);

impl AccountId {
    pub(crate) const fn from_database(value: i64) -> Self {
        Self(value)
    }

    pub(crate) const fn get(self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for AccountId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct HouseholdId(i64);

impl HouseholdId {
    pub(crate) const fn get(self) -> i64 {
        self.0
    }
}

impl std::str::FromStr for HouseholdId {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let id = value.parse::<i64>().map_err(|_| "invalid household ID")?;
        (id > 0).then_some(Self(id)).ok_or("invalid household ID")
    }
}

impl std::fmt::Display for HouseholdId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
