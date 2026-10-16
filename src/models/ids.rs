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
