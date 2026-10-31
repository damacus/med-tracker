use crate::models::entities::medication;
use sea_orm::{ColumnTrait, Condition};

pub(crate) const NHS_DMD_SYSTEM: &str = "https://dmd.nhs.uk";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DmdProductClass {
    Vmp,
    Amp,
    Vmpp,
    Ampp,
}

impl DmdProductClass {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "VMP" => Some(Self::Vmp),
            "AMP" => Some(Self::Amp),
            "VMPP" => Some(Self::Vmpp),
            "AMPP" => Some(Self::Ampp),
            _ => None,
        }
    }
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Vmp => "VMP",
            Self::Amp => "AMP",
            Self::Vmpp => "VMPP",
            Self::Ampp => "AMPP",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ExactProductIdentity<'a> {
    class: DmdProductClass,
    code: &'a str,
    stock_unit: &'a str,
}

impl<'a> ExactProductIdentity<'a> {
    pub(crate) fn from_parts(
        system: Option<&str>,
        class: Option<&str>,
        code: Option<&'a str>,
        stock_unit: Option<&'a str>,
    ) -> Option<Self> {
        if system? != NHS_DMD_SYSTEM {
            return None;
        }
        Some(Self {
            class: DmdProductClass::parse(class?)?,
            code: code.filter(|value| !value.trim().is_empty())?,
            stock_unit: stock_unit.filter(|value| !value.trim().is_empty())?,
        })
    }
    pub(crate) fn from_stock(stock: &'a medication::Model) -> Option<Self> {
        Self::from_parts(
            stock.dmd_system.as_deref(),
            stock.dmd_concept_class.as_deref(),
            stock.dmd_code.as_deref(),
            stock.dose_unit.as_deref(),
        )
    }
    pub(crate) fn condition(self) -> Condition {
        Condition::all()
            .add(medication::Column::DmdSystem.eq(NHS_DMD_SYSTEM))
            .add(medication::Column::DmdConceptClass.eq(self.class.as_str()))
            .add(medication::Column::DmdCode.eq(self.code))
            .add(medication::Column::DoseUnit.eq(self.stock_unit))
    }
}
