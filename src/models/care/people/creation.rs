use super::*;
use crate::models::care::guardian_declarations::{self, InitialGuardianAttestation};
use loco_rs::model::TenantQueryExt;
use sea_orm::ConnectionTrait;

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum DependentKind {
    Minor,
    DependentAdult,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    name: String,
    email: Option<String>,
    date_of_birth: NaiveDate,
    person_type: DependentKind,
    has_capacity: Option<bool>,
    initial_guardian_declaration: Option<bool>,
}

#[derive(serde::Deserialize)]
struct ImportedDependent {
    name: String,
    email: Option<String>,
    date_of_birth: NaiveDate,
    person_type: DependentKind,
    has_capacity: Option<bool>,
}

pub struct CreateDependent {
    name: String,
    email: Option<String>,
    date_of_birth: NaiveDate,
    kind: DependentKind,
    attestation: InitialGuardianAttestation,
}

impl CreateDependent {
    pub fn parse(attributes: &Value, today: NaiveDate) -> Result<Self, OperationError> {
        let request: Request = serde_json::from_value(attributes.clone())
            .map_err(|_| invalid("person", "contains unsupported or invalid fields"))?;
        let attestation = InitialGuardianAttestation::parse(request.initial_guardian_declaration)?;
        Self::validate(request, attestation, today)
    }

    pub(crate) fn parse_import(
        row: &Value,
        attestation: InitialGuardianAttestation,
        today: NaiveDate,
    ) -> Result<Self, OperationError> {
        let imported: ImportedDependent = serde_json::from_value(row.clone())
            .map_err(|_| invalid("person", "contains unsupported or invalid dependent fields"))?;
        Self::validate(
            Request {
                name: imported.name,
                email: imported.email,
                date_of_birth: imported.date_of_birth,
                person_type: imported.person_type,
                has_capacity: imported.has_capacity,
                initial_guardian_declaration: None,
            },
            attestation,
            today,
        )
    }

    fn validate(
        request: Request,
        attestation: InitialGuardianAttestation,
        today: NaiveDate,
    ) -> Result<Self, OperationError> {
        if request.name.trim().is_empty() {
            return Err(invalid("name", "can't be blank"));
        }
        if request.has_capacity == Some(true) {
            return Err(invalid("has_capacity", "must be false for a dependent"));
        }
        let years = age(request.date_of_birth, today);
        if years < 0
            || !matches!(
                (&request.person_type, years < 18),
                (DependentKind::Minor, true) | (DependentKind::DependentAdult, false)
            )
        {
            return Err(invalid("person_type", "does not match age"));
        }
        let email = request
            .email
            .map(|email| email.trim().to_lowercase())
            .filter(|email| !email.is_empty());
        if email
            .as_deref()
            .is_some_and(|email| !validation::email_valid(email))
        {
            return Err(invalid("email", "is invalid"));
        }
        Ok(Self {
            name: request.name,
            email,
            date_of_birth: request.date_of_birth,
            kind: request.person_type,
            attestation,
        })
    }
}

pub async fn create(
    tenant: &TenantTransaction,
    command: CreateDependent,
    today: NaiveDate,
    provenance: Option<&CredentialProvenance>,
) -> Result<person::Model, OperationError> {
    let saved = create_imported(
        tenant,
        command,
        uuid::Uuid::new_v4().to_string(),
        today,
        provenance,
    )
    .await?;
    let now = Utc::now().naive_utc();
    let home = location::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(location::Column::Name.eq("Home"))
        .one(tenant.transaction())
        .await?;
    let home = match home {
        Some(home) => home,
        None => {
            let home = location::ActiveModel {
                household_id: Set(tenant.scope().household_id),
                portable_id: Set(uuid::Uuid::new_v4().to_string()),
                name: Set("Home".to_owned()),
                description: Set(Some("Primary home location".to_owned())),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(tenant.transaction())
            .await?;
            persistence::record_version(tenant,"Location",home.id,"create",None,json!({"id":home.id,"household_id":home.household_id,"portable_id":home.portable_id,"name":home.name,"description":home.description,"created_at":home.created_at,"updated_at":home.updated_at}),provenance).await?;
            persistence::change(
                tenant,
                "Location",
                home.id,
                &home.portable_id,
                "create",
                provenance,
            )
            .await?;
            home
        }
    };
    let link = location_membership::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        location_id: Set(home.id),
        person_id: Set(saved.id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    persistence::record_version(
        tenant,
        "Person",
        saved.id,
        "create",
        None,
        persistence::snapshot(&saved),
        provenance,
    )
    .await?;
    persistence::record_version(tenant,"LocationMembership",link.id,"create",None,json!({"id":link.id,"household_id":link.household_id,"location_id":link.location_id,"person_id":link.person_id,"created_at":link.created_at,"updated_at":link.updated_at}),provenance).await?;
    persistence::change(
        tenant,
        "Person",
        saved.id,
        &saved.portable_id,
        "create",
        provenance,
    )
    .await?;
    Ok(saved)
}

pub(crate) async fn create_imported(
    tenant: &TenantTransaction,
    command: CreateDependent,
    portable_id: String,
    today: NaiveDate,
    provenance: Option<&CredentialProvenance>,
) -> Result<person::Model, OperationError> {
    authorize_create(tenant).await?;
    let guardian = guardian_declarations::eligible_actor(tenant, today).await?;
    let now = Utc::now().naive_utc();
    let identity = tenant
        .transaction()
        .query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            "SELECT nextval(pg_get_serial_sequence('public.people','id')) AS id",
        ))
        .await?
        .ok_or(OperationError::Unavailable)?;
    let id: i64 = identity.try_get("", "id")?;
    let record = person::ActiveModel {
        id: Set(id),
        portable_id: Set(portable_id),
        name: Set(command.name),
        email: Set(command.email),
        person_type: Set(match command.kind {
            DependentKind::Minor => 1,
            DependentKind::DependentAdult => 2,
        }),
        date_of_birth: Set(Some(command.date_of_birth)),
        has_capacity: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    person::Entity::insert(record)
        .exec_without_returning(tenant.transaction())
        .await
        .map_err(write_error)?;
    let participation = membership::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        person_id: Set(id),
        role: Set("member".to_owned()),
        status: Set("active".to_owned()),
        joined_at: Set(now),
        permissions_version: Set(1),
        revoked_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    let saved = person::Entity::find_by_id(id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unavailable)?;
    let declaration = guardian_declarations::record_initial(
        tenant,
        &saved,
        command.attestation,
        today,
        provenance,
    )
    .await?;
    let relationship = carer_relationship::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        carer_id: Set(guardian.id),
        patient_id: Set(saved.id),
        guardian_declaration_id: Set(Some(declaration.get())),
        relationship_type: Set(Some("family_member".to_owned())),
        active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    persistence::record_version(tenant,"HouseholdMembership",participation.id,"create",None,json!({"id":participation.id,"household_id":participation.household_id,"person_id":saved.id,"role":participation.role,"status":participation.status}),provenance).await?;
    persistence::record_version(tenant,"CarerRelationship",relationship.id,"create",None,json!({"id":relationship.id,"household_id":relationship.household_id,"carer_id":relationship.carer_id,"patient_id":relationship.patient_id,"guardian_declaration_id":declaration.get(),"relationship_type":relationship.relationship_type,"active":relationship.active}),provenance).await?;
    Ok(saved)
}
