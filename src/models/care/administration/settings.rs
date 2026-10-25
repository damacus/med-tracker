use super::*;

#[derive(Default)]
enum TextField {
    #[default]
    Missing,
    Value(String),
    Invalid,
}

#[derive(Default)]
pub struct HouseholdSettingsInput {
    name: TextField,
    timezone: TextField,
    subscription_plan: TextField,
    invalid_shape: bool,
    unknown_field: bool,
    empty: bool,
}

struct SettingsValues {
    name: Option<String>,
    timezone: Option<String>,
    subscription_plan: Option<String>,
}

impl HouseholdSettingsInput {
    pub fn from_json(value: Value) -> Self {
        let Value::Object(fields) = value else {
            return Self {
                invalid_shape: true,
                ..Self::default()
            };
        };
        let text = |field: &str| match fields.get(field) {
            None => TextField::Missing,
            Some(Value::String(value)) => TextField::Value(value.clone()),
            Some(_) => TextField::Invalid,
        };
        Self {
            name: text("name"),
            timezone: text("timezone"),
            subscription_plan: text("subscription_plan"),
            unknown_field: fields
                .keys()
                .any(|key| !matches!(key.as_str(), "name" | "timezone" | "subscription_plan")),
            empty: fields.is_empty(),
            invalid_shape: false,
        }
    }

    pub fn from_form_name(name: &str) -> Self {
        Self {
            name: TextField::Value(name.into()),
            ..Self::default()
        }
    }

    fn check_shape(&self) -> Result<(), OperationError> {
        if self.invalid_shape || self.unknown_field || self.empty {
            return Err(invalid("household", "is invalid"));
        }
        Ok(())
    }

    fn values(self) -> Result<SettingsValues, OperationError> {
        let parse =
            |field: &'static str, value: TextField| -> Result<Option<String>, OperationError> {
                match value {
                    TextField::Missing => Ok(None),
                    TextField::Value(value) if !value.trim().is_empty() => Ok(Some(value)),
                    _ => Err(invalid(field, "is invalid")),
                }
            };
        let name = parse("name", self.name)?;
        let subscription_plan = parse("subscription_plan", self.subscription_plan)?;
        if subscription_plan
            .as_deref()
            .is_some_and(|plan| !matches!(plan, "free" | "family_plus"))
        {
            return Err(invalid("subscription_plan", "is invalid"));
        }
        let timezone = parse("timezone", self.timezone)?;
        Ok(SettingsValues {
            name,
            timezone,
            subscription_plan,
        })
    }
}

fn value(row: &household::Model) -> Value {
    json!({"id":row.id,"name":row.name,"slug":row.slug,"timezone":row.timezone,"subscription_plan":row.subscription_plan,"updated_at":row.updated_at.and_utc().to_rfc3339()})
}
pub async fn read(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    authorize(tenant).await?;
    let row = household::Entity::find_by_id(tenant.scope().household_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    Ok(json!({"data":value(&row)}))
}
pub async fn update(
    tenant: &TenantTransaction,
    input: HouseholdSettingsInput,
    provenance: Option<&CredentialProvenance>,
) -> Result<Value, OperationError> {
    authorize(tenant).await?;
    input.check_shape()?;
    let row = household::Entity::find_by_id(tenant.scope().household_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    let before = value(&row);
    let mut active: household::ActiveModel = row.into();
    let values = input.values()?;
    if let Some(name) = values.name {
        active.name = Set(name);
    }
    if let Some(timezone) = values.timezone {
        active.timezone = Set(timezone);
    }
    if let Some(subscription_plan) = values.subscription_plan {
        active.subscription_plan = Set(subscription_plan);
    }
    active.updated_at = Set(Utc::now().naive_utc());
    let row = active.update(tenant.transaction()).await?;
    let after = value(&row);
    persistence::record_version(
        tenant,
        "Household",
        row.id,
        Some(before),
        after.clone(),
        provenance,
    )
    .await?;
    persistence::event(
        tenant,
        "api/admin/household_settings/updated",
        json!({"target_type":"Household","target_id":row.id,"outcome":"success"}),
        provenance,
    )
    .await?;
    Ok(json!({"data":after}))
}
