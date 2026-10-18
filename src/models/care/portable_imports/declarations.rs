use super::*;
use crate::models::care::{guardian_declarations, people};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct InitialDeclaration {
    person_portable_id: String,
    accepted: bool,
}

pub(crate) struct InitialDeclarations {
    accepted: HashMap<String, guardian_declarations::InitialGuardianAttestation>,
}

pub(crate) struct PreparedDeclarations {
    commands: HashMap<String, people::CreateDependent>,
    today: NaiveDate,
}

impl InitialDeclarations {
    pub(crate) fn parse(value: Option<&Value>) -> Result<Self, OperationError> {
        let declarations: Vec<InitialDeclaration> = match value {
            Some(value) => serde_json::from_value(value.clone()).map_err(|_| invalid())?,
            None => Vec::new(),
        };
        let mut accepted = HashMap::new();
        for declaration in declarations {
            let attestation = guardian_declarations::InitialGuardianAttestation::parse(Some(
                declaration.accepted,
            ))?;
            if declaration.person_portable_id.trim().is_empty()
                || accepted
                    .insert(declaration.person_portable_id, attestation)
                    .is_some()
            {
                return Err(invalid());
            }
        }
        Ok(Self { accepted })
    }

    pub(crate) async fn prepare(
        mut self,
        tenant: &TenantTransaction,
        plan: &ImportPlan,
        index: &ExistingIndex,
        today: NaiveDate,
    ) -> Result<PreparedDeclarations, OperationError> {
        let mut commands = HashMap::new();
        for row in plan.rows("people") {
            let portable_id = row["portable_id"].as_str().ok_or_else(invalid)?;
            if index.get("people", portable_id).is_some() {
                if self.accepted.contains_key(portable_id) {
                    return Err(invalid());
                }
                continue;
            }
            let attestation = self.accepted.remove(portable_id).ok_or_else(invalid)?;
            let command = people::CreateDependent::parse_import(row, attestation, today)?;
            commands.insert(portable_id.to_owned(), command);
        }
        if !self.accepted.is_empty() {
            return Err(invalid());
        }
        if !commands.is_empty() {
            if !people::can_create(tenant).await? {
                return Err(OperationError::Forbidden);
            }
            guardian_declarations::eligible_actor(tenant, today).await?;
        }
        Ok(PreparedDeclarations { commands, today })
    }
}

impl PreparedDeclarations {
    pub(super) fn contains(&self, portable_id: &str) -> bool {
        self.commands.contains_key(portable_id)
    }

    pub(super) async fn create(
        &mut self,
        tenant: &TenantTransaction,
        portable_id: &str,
        provenance: &CredentialProvenance,
    ) -> Result<i64, OperationError> {
        let command = self.commands.remove(portable_id).ok_or_else(invalid)?;
        people::create_imported(
            tenant,
            command,
            portable_id.to_owned(),
            self.today,
            Some(provenance),
        )
        .await
        .map(|person| person.id)
    }
}

fn invalid() -> OperationError {
    guardian_declarations::validation(
        "initial_guardian_declarations",
        "requires one fresh true declaration for each new dependent and none for existing people",
    )
}
