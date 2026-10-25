use super::*;
use loco_rs::model::TenantQueryExt;

pub struct EmptyChangeInput(bool);

impl EmptyChangeInput {
    pub fn from_body(body: &Value) -> Self {
        Self(body.as_object().is_some_and(|body| body.is_empty()))
    }

    pub fn empty() -> Self {
        Self(true)
    }

    fn validate(&self) -> Result<(), OperationError> {
        if self.0 {
            Ok(())
        } else {
            Err(invalid("body", "must be empty"))
        }
    }
}

#[derive(Clone, Copy)]
pub enum Direction {
    Up,
    Down,
}

pub struct DirectionInput(Option<Direction>);

impl DirectionInput {
    pub fn from_body(body: &Value) -> Self {
        let direction = body
            .as_object()
            .filter(|map| map.len() == 1)
            .and_then(|map| map.get("direction"))
            .and_then(Value::as_str);
        Self(match direction {
            Some("up") => Some(Direction::Up),
            Some("down") => Some(Direction::Down),
            _ => None,
        })
    }

    fn validate(&self) -> Result<Direction, OperationError> {
        self.0
            .ok_or_else(|| invalid("direction", "must be up or down"))
    }
}

async fn source_path(
    tenant: &TenantTransaction,
    kind: Kind,
    id: &str,
) -> Result<Source, OperationError> {
    let portable = if let Ok(id) = id.parse::<i64>() {
        match kind {
            Kind::Schedule => schedule::Entity::find_by_id(id)
                .in_tenant(tenant.scope().household_id)
                .one(tenant.transaction())
                .await?
                .map(|row| row.portable_id),
            Kind::Assignment => person_medication::Entity::find_by_id(id)
                .in_tenant(tenant.scope().household_id)
                .one(tenant.transaction())
                .await?
                .map(|row| row.portable_id),
        }
        .ok_or(OperationError::NotFound)?
    } else {
        Uuid::parse_str(id).map_err(|_| OperationError::NotFound)?;
        id.to_owned()
    };
    find_source(tenant, kind, &portable, false).await
}
async fn change(
    tenant: &TenantTransaction,
    id: &str,
    input: &EmptyChangeInput,
    provenance: Option<&CredentialProvenance>,
    kind: Kind,
    pause: bool,
) -> Result<(Value, String), OperationError> {
    lock(tenant).await?;
    let source = source_path(tenant, kind, id).await?;
    access::require_person_access(tenant, source.person_id(), PersonAccess::Manage).await?;
    input.validate()?;
    let context = AuthContext { tenant, provenance };
    let source = if pause {
        persistence::pause_source(
            tenant.transaction(),
            &context,
            source,
            "reason_not_recorded",
            None,
            &tenant.scope().request_id,
        )
        .await?
        .0
    } else {
        persistence::close_period(
            tenant.transaction(),
            &context,
            source,
            None,
            &tenant.scope().request_id,
        )
        .await?
        .0
    };
    match source {
        Source::Schedule(row) => {
            crate::models::care::treatments::lifecycle::read(tenant, &row.id.to_string()).await
        }
        Source::Assignment(row) => {
            crate::models::care::assignments::read(tenant, &row.id.to_string()).await
        }
    }
}
pub async fn pause_schedule(
    tenant: &TenantTransaction,
    id: &str,
    input: &EmptyChangeInput,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    change(tenant, id, input, provenance, Kind::Schedule, true).await
}
pub async fn resume_schedule(
    tenant: &TenantTransaction,
    id: &str,
    input: &EmptyChangeInput,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    change(tenant, id, input, provenance, Kind::Schedule, false).await
}
pub async fn pause_assignment(
    tenant: &TenantTransaction,
    id: &str,
    input: &EmptyChangeInput,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    change(tenant, id, input, provenance, Kind::Assignment, true).await
}
pub async fn resume_assignment(
    tenant: &TenantTransaction,
    id: &str,
    input: &EmptyChangeInput,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    change(tenant, id, input, provenance, Kind::Assignment, false).await
}
pub async fn reorder_assignment(
    tenant: &TenantTransaction,
    id: &str,
    input: &DirectionInput,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    lock(tenant).await?;
    let Source::Assignment(source) = source_path(tenant, Kind::Assignment, id).await? else {
        return Err(OperationError::NotFound);
    };
    access::require_person_access(tenant, source.person_id, PersonAccess::Manage).await?;
    let direction = input.validate()?;
    let mut query = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person_medication::Column::PersonId.eq(source.person_id))
        .filter(person_medication::Column::RetiredAt.is_null())
        .filter(if matches!(direction, Direction::Up) {
            person_medication::Column::Position.lt(source.position)
        } else {
            person_medication::Column::Position.gt(source.position)
        });
    query = if matches!(direction, Direction::Up) {
        query
            .order_by_desc(person_medication::Column::Position)
            .order_by_desc(person_medication::Column::Id)
    } else {
        query
            .order_by_asc(person_medication::Column::Position)
            .order_by_asc(person_medication::Column::Id)
    };
    let adjacent = query.one(tenant.transaction()).await?;
    let context = AuthContext { tenant, provenance };
    let updated = if let Some(adjacent) = adjacent {
        let original_position = source.position;
        let adjacent_position = adjacent.position;
        let mut moving: person_medication::ActiveModel = source.clone().into();
        moving.position = Set(adjacent_position);
        moving.updated_at = Set(Utc::now().naive_utc());
        let moving = moving.update(tenant.transaction()).await?;
        let mut swapping: person_medication::ActiveModel = adjacent.clone().into();
        swapping.position = Set(original_position);
        swapping.updated_at = Set(Utc::now().naive_utc());
        let swapping = swapping.update(tenant.transaction()).await?;
        let person_portable_id = person::Entity::find_by_id(source.person_id)
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::NotFound)?
            .portable_id;
        for (before, after) in [(&source, &moving), (&adjacent, &swapping)] {
            record_version(
                &context,
                "PersonMedication",
                after.id,
                "update",
                Some(json!({"position": before.position})),
                Some(json!({"position": after.position})),
            )
            .await?;
            record_change(
                tenant.transaction(),
                &context,
                &tenant.scope().request_id,
                SyncRecord {
                    record_type: "PersonMedication",
                    record_id: after.id,
                    portable_id: &after.portable_id,
                    action: "update",
                    person_portable_id: Some(&person_portable_id),
                },
            )
            .await?;
        }
        moving
    } else {
        source
    };
    crate::models::care::assignments::read(tenant, &updated.id.to_string()).await
}
