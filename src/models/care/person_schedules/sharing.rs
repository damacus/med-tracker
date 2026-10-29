use crate::models::{
    access::TenantTransaction,
    entities::{membership, sharing_approval, version},
    errors::OperationError,
    ids::PersonId,
};
use crate::models::{
    entities::{schedule, shared_schedule},
    ids::AccountId,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel, QueryFilter,
    QuerySelect, Set,
    sea_query::{Expr, ExprTrait, Func, Query},
};
use sea_orm::{DatabaseTransaction, QueryOrder, TransactionTrait};
use serde_json::json;

pub struct SharingConsent(());

#[derive(Clone, Copy)]
pub(crate) enum ConsentAuthority {
    SelfPerson,
    Guardian(super::super::guardian_declarations::DeclarationId),
}

impl ConsentAuthority {
    pub(crate) async fn current(
        self,
        db: &impl ConnectionTrait,
        subject: PersonId,
    ) -> Result<bool, OperationError> {
        match self {
            Self::SelfPerson => Ok(true),
            Self::Guardian(id) => Ok(guardian(db, subject).await? == Some(id.get())),
        }
    }
    pub(crate) const fn dependent(self) -> bool {
        matches!(self, Self::Guardian(_))
    }
}

#[derive(serde::Serialize)]
pub(crate) struct EligibleDependent {
    id: String,
    name: String,
}

pub(crate) async fn dependents(
    db: &impl ConnectionTrait,
    actor: AccountId,
) -> Result<Vec<EligibleDependent>, OperationError> {
    let person =
        crate::models::access::person_queries::account_person(crate::models::access::Actor {
            account_id: actor.get(),
        })
        .one(db)
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let declared = Query::select()
        .column("subject_person_id")
        .from("guardian_declarations")
        .and_where(Expr::col("guardian_person_id").eq(person.id))
        .and_where(Expr::col("withdrawn_at").is_null())
        .and_where(Expr::col("superseded_at").is_null())
        .to_owned();
    let subjects = crate::models::entities::person::Entity::find()
        .filter(Expr::col(crate::models::entities::person::Column::Id).in_subquery(declared))
        .filter(
            Expr::expr(
                Func::cust("med_tracker.sharing_guardian")
                    .arg(Expr::col(crate::models::entities::person::Column::Id)),
            )
            .is_not_null(),
        )
        .order_by_asc(crate::models::entities::person::Column::Name)
        .all(db)
        .await?;
    Ok(subjects
        .into_iter()
        .map(|person| EligibleDependent {
            id: person.id.to_string(),
            name: person.name,
        })
        .collect())
}

impl SharingConsent {
    pub fn parse(accepted: Option<bool>) -> Result<Self, OperationError> {
        if accepted == Some(true) {
            Ok(Self(()))
        } else {
            Err(super::super::guardian_declarations::validation(
                "sharing_consent",
                "must be explicitly accepted",
            ))
        }
    }
}

pub(crate) async fn guardian(
    db: &impl ConnectionTrait,
    subject: PersonId,
) -> Result<Option<i64>, OperationError> {
    let query = Query::select()
        .expr_as(
            Func::cust("med_tracker.sharing_guardian").arg(Expr::val(subject.get())),
            "declaration_id",
        )
        .to_owned();
    db.query_one_raw(db.get_database_backend().build(&query))
        .await?
        .ok_or(OperationError::Unavailable)?
        .try_get("", "declaration_id")
        .map_err(Into::into)
}

pub(crate) async fn approve(
    db: &impl ConnectionTrait,
    actor: &super::super::invitations::acceptance::AcceptanceActor,
    invitation: &crate::models::entities::household_invitation::Model,
    subject: PersonId,
    authority: ConsentAuthority,
    request_id: &str,
) -> Result<(), OperationError> {
    shared_schedule::Entity::find()
        .filter(shared_schedule::Column::PersonId.eq(subject.get()))
        .order_by_asc(shared_schedule::Column::Id)
        .lock_exclusive()
        .all(db)
        .await?;
    crate::models::access::verify_account(db, actor.account_id).await?;
    if !authority.current(db, subject).await? {
        return Err(OperationError::Forbidden);
    }
    let declaration = match authority {
        ConsentAuthority::SelfPerson if subject.get() == actor.person_id => None,
        ConsentAuthority::Guardian(id) if subject.get() != actor.person_id => Some(id.get()),
        _ => return Err(OperationError::Forbidden),
    };
    if let Some(id) = declaration {
        let lock = Query::select()
            .column("id")
            .from("guardian_declarations")
            .and_where(Expr::col("id").eq(id))
            .lock(sea_orm::sea_query::LockType::Share)
            .to_owned();
        db.query_one_raw(db.get_database_backend().build(&lock))
            .await?
            .ok_or(OperationError::Forbidden)?;
        if guardian(db, subject).await? != Some(id) {
            return Err(OperationError::Forbidden);
        }
    }
    let now = Utc::now().naive_utc();
    let row = sharing_approval::ActiveModel {
        household_id: Set(invitation.household_id),
        person_id: Set(subject.get()),
        invitation_id: Set(invitation.id),
        approved_by_account_id: Set(actor.account_id),
        approved_by_person_id: Set(actor.person_id),
        guardian_declaration_id: Set(declaration),
        statement_version: Set(1),
        approved_at: Set(now),
        ended_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await?;
    version::ActiveModel {
        item_type:Set("PersonHouseholdApproval".into()), item_id:Set(row.id), event:Set("approve".into()),
        whodunnit:Set(Some(actor.account_id.to_string())), household_id:Set(Some(invitation.household_id)),
        actor_membership_id:Set(None), created_at:Set(Some(now)),request_id:Set(Some(request_id.into())),
        audit_context:Set(json!({"actor_account_id":actor.account_id,"actor_person_id":actor.person_id,"actor_membership_id":null,"person_id":subject.get(),"invitation_id":invitation.id,"guardian_declaration_id":declaration,"statement_version":1})),
        ..Default::default()
    }.insert(db).await?;
    Ok(())
}

pub(crate) async fn attach_origins(
    db: &impl ConnectionTrait,
    invitation_id: i64,
    household_id: i64,
    subject: PersonId,
) -> Result<(), OperationError> {
    use crate::models::entities::shared_execution;
    let approval = sharing_approval::Entity::find()
        .filter(sharing_approval::Column::InvitationId.eq(invitation_id))
        .filter(sharing_approval::Column::PersonId.eq(subject.get()))
        .filter(sharing_approval::Column::HouseholdId.eq(household_id))
        .filter(sharing_approval::Column::EndedAt.is_null())
        .one(db)
        .await?
        .ok_or(OperationError::Forbidden)?;
    let previous = Query::select()
        .expr_as(
            Func::cust("current_setting").args([
                Expr::val("med_tracker.sharing_attachment_approval_id"),
                Expr::val(true),
            ]),
            "purpose",
        )
        .to_owned();
    let previous: Option<String> = db
        .query_one_raw(db.get_database_backend().build(&previous))
        .await?
        .ok_or(OperationError::Unavailable)?
        .try_get("", "purpose")?;
    attachment_purpose(db, &approval.id.to_string()).await?;
    let sources: Vec<i64> = shared_execution::Entity::find()
        .select_only()
        .column(shared_execution::Column::ScheduleId)
        .filter(shared_execution::Column::HouseholdId.eq(household_id))
        .filter(shared_execution::Column::PersonId.eq(subject.get()))
        .filter(shared_execution::Column::ApprovalId.is_null())
        .order_by_asc(shared_execution::Column::PersonScheduleId)
        .order_by_asc(shared_execution::Column::ScheduleId)
        .into_tuple()
        .all(db)
        .await?;
    for source in sources {
        // The validation trigger skips only a retained ended origin. Any
        // authority/identity error aborts the complete acceptance transaction.
        shared_execution::Entity::update_many()
            .col_expr(shared_execution::Column::ApprovalId, Expr::val(approval.id))
            .filter(shared_execution::Column::ScheduleId.eq(source))
            .filter(shared_execution::Column::ApprovalId.is_null())
            .exec(db)
            .await?;
    }
    attachment_purpose(db, &previous.unwrap_or_default()).await?;
    Ok(())
}

async fn attachment_purpose(
    db: &impl ConnectionTrait,
    purpose: &str,
) -> Result<(), OperationError> {
    let query = Query::select()
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.sharing_attachment_approval_id"),
            Expr::val(purpose),
            Expr::val(true),
        ]))
        .to_owned();
    db.execute_raw(db.get_database_backend().build(&query))
        .await?;
    Ok(())
}

pub(crate) async fn end(
    tenant: &TenantTransaction,
    subject: PersonId,
) -> Result<(), OperationError> {
    let rows = sharing_approval::Entity::find()
        .filter(sharing_approval::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(sharing_approval::Column::PersonId.eq(subject.get()))
        .filter(sharing_approval::Column::EndedAt.is_null())
        .lock_exclusive()
        .all(tenant.transaction())
        .await?;
    for row in rows {
        let id = row.id;
        let mut ended = row.into_active_model();
        ended.ended_at = Set(Some(Utc::now().naive_utc()));
        ended.update(tenant.transaction()).await?;
        super::super::administration::persistence::record_version_as(
            tenant,
            "PersonHouseholdApproval",
            id,
            "end",
            None,
            json!({"ended":true}),
            None,
        )
        .await?;
    }
    Ok(())
}

pub(crate) async fn current(
    db: &impl ConnectionTrait,
    household_id: i64,
    subject: i64,
    invitation_id: i64,
) -> Result<bool, OperationError> {
    let approval = sharing_approval::Entity::find()
        .filter(sharing_approval::Column::HouseholdId.eq(household_id))
        .filter(sharing_approval::Column::PersonId.eq(subject))
        .filter(sharing_approval::Column::InvitationId.eq(invitation_id))
        .filter(sharing_approval::Column::EndedAt.is_null())
        .one(db)
        .await?;
    let Some(approval) = approval else {
        return Ok(false);
    };
    if let Some(declaration) = approval.guardian_declaration_id
        && guardian(db, PersonId::from_database(subject)).await? != Some(declaration)
    {
        return Ok(false);
    }
    Ok(membership::Entity::find()
        .filter(membership::Column::HouseholdId.eq(household_id))
        .filter(membership::Column::PersonId.eq(subject))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .one(db)
        .await?
        .is_some())
}

pub(super) async fn clinical_authority(
    db: &impl ConnectionTrait,
    subject: PersonId,
) -> Result<bool, OperationError> {
    let query = Query::select()
        .expr_as(
            Func::cust("med_tracker.sharing_clinical_authority").arg(Expr::val(subject.get())),
            "permitted",
        )
        .to_owned();
    db.query_one_raw(db.get_database_backend().build(&query))
        .await?
        .ok_or(OperationError::Unavailable)?
        .try_get("", "permitted")
        .map_err(Into::into)
}

pub(crate) async fn withdraw(
    db: &DatabaseTransaction,
    actor: AccountId,
    id: i64,
    request_id: &str,
    provenance: &super::super::doses::CredentialProvenance,
) -> Result<(), OperationError> {
    crate::models::access::verify_account(db, actor.get()).await?;
    let initial = sharing_approval::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or(OperationError::NotFound)?;
    let subject = PersonId::from_database(initial.person_id);
    if !clinical_authority(db, subject).await? {
        return Err(OperationError::Forbidden);
    }
    let person =
        crate::models::access::person_queries::account_person(crate::models::access::Actor {
            account_id: actor.get(),
        })
        .one(db)
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    let authority = if person.id == subject.get() {
        ConsentAuthority::SelfPerson
    } else {
        ConsentAuthority::Guardian(
            super::super::guardian_declarations::DeclarationId::from_database(
                guardian(db, subject)
                    .await?
                    .ok_or(OperationError::Forbidden)?,
            ),
        )
    };
    shared_schedule::Entity::find()
        .filter(shared_schedule::Column::PersonId.eq(subject.get()))
        .order_by_asc(shared_schedule::Column::Id)
        .lock_exclusive()
        .all(db)
        .await?;
    let approval = sharing_approval::Entity::find_by_id(id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or(OperationError::NotFound)?;
    crate::models::access::verify_account(db, actor.get()).await?;
    if !clinical_authority(db, subject).await? || !authority.current(db, subject).await? {
        return Err(OperationError::Forbidden);
    }
    if approval.ended_at.is_some() {
        return Ok(());
    }
    let savepoint = db.begin().await?;
    let previous = Query::select()
        .expr_as(
            Func::cust("current_setting").args([
                Expr::val("med_tracker.sharing_withdrawal_approval_id"),
                Expr::val(true),
            ]),
            "purpose",
        )
        .to_owned();
    let previous: Option<String> = savepoint
        .query_one_raw(savepoint.get_database_backend().build(&previous))
        .await?
        .ok_or(OperationError::Unavailable)?
        .try_get("", "purpose")?;
    let purpose = Query::select()
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.sharing_withdrawal_approval_id"),
            Expr::val(id.to_string()),
            Expr::val(true),
        ]))
        .to_owned();
    savepoint
        .execute_raw(savepoint.get_database_backend().build(&purpose))
        .await?;
    let now = Utc::now().naive_utc();
    let result = async {
    use crate::models::entities::shared_execution;
    let identifiers: Vec<(i64,String,i64)> = shared_execution::Entity::find().select_only()
        .columns([shared_execution::Column::ScheduleId, shared_execution::Column::PortableId, shared_execution::Column::HouseholdId])
        .filter(shared_execution::Column::ApprovalId.eq(id)).into_tuple().all(&savepoint).await?;
    let closure = schedule::Entity::update_many()
        .col_expr(schedule::Column::ExecutionEndedAt, Expr::val(now))
        .col_expr(schedule::Column::Active, Expr::val(false))
        .col_expr(schedule::Column::UpdatedAt, Expr::val(now))
        .filter(Expr::expr(Func::cust("current_setting").args([
            Expr::val("med_tracker.sharing_withdrawal_approval_id"), Expr::val(true),
        ])).eq(id.to_string()))
        .exec(&savepoint)
        .await?;
    let mut ended = approval.into_active_model();
    ended.ended_at = Set(Some(now));
    ended.update(&savepoint).await?;
    let subject_record=crate::models::entities::person::Entity::find_by_id(subject.get()).one(&savepoint).await?.ok_or(OperationError::NotFound)?;
    for (source_id,portable_id,household_id) in identifiers.into_iter().filter(|_|closure.rows_affected>0) {
        crate::models::entities::api_tombstone::Entity::insert(crate::models::entities::api_tombstone::ActiveModel {
            household_id:Set(household_id),household_membership_id:Set(None),account_id:Set(Some(actor.get())),
            action:Set("delete".into()),record_type:Set("Schedule".into()),record_portable_id:Set(portable_id.clone()),deleted_at:Set(now),created_at:Set(now),updated_at:Set(now),
            metadata:Set(json!({"record_type":"Schedule","record_id":source_id,"portable_id":portable_id,"person_portable_id":subject_record.portable_id,"approval_id":id})),..Default::default()
        }).exec_without_returning(&savepoint).await?;
    }
    let restore = Query::select()
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.sharing_withdrawal_approval_id"),
            Expr::val(previous.unwrap_or_default()),
            Expr::val(true),
        ]))
        .to_owned();
    savepoint
        .execute_raw(savepoint.get_database_backend().build(&restore))
        .await?;
    Ok::<_,OperationError>(closure.rows_affected)
    }.await;
    let closed_executions = match result {
        Ok(count) => count,
        Err(error) => {
            savepoint.rollback().await?;
            return Err(error);
        }
    };
    savepoint.commit().await?;
    version::ActiveModel {
        item_type:Set("PersonHouseholdApproval".into()),item_id:Set(id),event:Set("withdraw".into()),whodunnit:Set(Some(actor.get().to_string())),
        household_id:Set(None),actor_membership_id:Set(None),request_id:Set(Some(request_id.into())),created_at:Set(Some(now)),
        audit_context:Set(json!({"actor_account_id":actor.get(),"actor_person_id":person.id,"actor_membership_id":null,"approval_id":id,"closed_executions":closed_executions,"authentication_method":provenance.method.label(),"session_reference":format!("{}:{}",provenance.method.reference_prefix(),provenance.reference)})),
        ..Default::default()
    }.insert(db).await?;
    Ok(())
}

pub(crate) async fn withdraw_declaration(
    db: &DatabaseTransaction,
    actor: AccountId,
    declaration: i64,
    request_id: &str,
    provenance: &super::super::doses::CredentialProvenance,
) -> Result<(), OperationError> {
    let approvals = sharing_approval::Entity::find()
        .filter(sharing_approval::Column::GuardianDeclarationId.eq(declaration))
        .filter(sharing_approval::Column::EndedAt.is_null())
        .order_by_asc(sharing_approval::Column::Id)
        .all(db)
        .await?;
    for approval in approvals {
        withdraw(db, actor, approval.id, request_id, provenance).await?;
    }
    Ok(())
}
