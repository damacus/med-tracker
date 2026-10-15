use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, Statement,
    TransactionTrait,
};

use crate::models::{care::doses::CredentialProvenance, errors::OperationError};

pub(crate) enum AccessDenial {
    Unauthenticated,
    Forbidden,
}

pub(crate) struct BrowserAccess {
    account: crate::models::ids::AccountId,
    person: crate::models::ids::PersonId,
    provenance: CredentialProvenance,
    trace: uuid::Uuid,
}

impl BrowserAccess {
    pub(crate) async fn capture(
        transaction: &DatabaseTransaction,
        principal: &crate::models::identity::browser::BrowserPrincipal,
        trace: uuid::Uuid,
    ) -> Result<Self, OperationError> {
        if !matches!(
            principal.provenance().method,
            super::doses::CredentialMethod::BrowserSession
        ) {
            return Err(OperationError::Forbidden);
        }
        let row=transaction.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT person_id FROM public.accounts WHERE id=$1 AND id=med_tracker.current_account_id()",
            [principal.account_id().into()])).await?.ok_or(OperationError::Forbidden)?;
        Ok(Self {
            account: crate::models::ids::AccountId::from_database(principal.account_id()),
            person: crate::models::ids::PersonId::from_database(row.try_get("", "person_id")?),
            provenance: principal.provenance().clone(),
            trace,
        })
    }

    pub(crate) async fn recheck(
        &self,
        transaction: &DatabaseTransaction,
        principal: &crate::models::identity::browser::BrowserPrincipal,
    ) -> Result<(), OperationError> {
        let current = Self::capture(transaction, principal, self.trace).await?;
        if self.account != current.account
            || self.person != current.person
            || self.provenance.reference != current.provenance.reference
        {
            return Err(OperationError::Forbidden);
        }
        Ok(())
    }

    pub(crate) async fn record_read(
        &self,
        transaction: &DatabaseTransaction,
    ) -> Result<(), OperationError> {
        record_read(transaction, &self.provenance, self.trace).await
    }

    pub(crate) async fn record_download(
        &self,
        transaction: &DatabaseTransaction,
    ) -> Result<(), OperationError> {
        transaction.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT med_tracker.append_account_clinical_access('self_medication_history.download',$1,$2,$3::uuid)",
            [self.provenance.method.label().into(),self.provenance.reference.clone().into(),self.trace.to_string().into()])).await?;
        Ok(())
    }

    pub(crate) async fn record_denial(
        &self,
        db: &DatabaseConnection,
        denial: AccessDenial,
    ) -> Result<(), OperationError> {
        let transaction = db.begin().await?;
        let result=async {
            transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app; SET LOCAL search_path=pg_catalog,public,pg_temp; SELECT set_config('med_tracker.current_account_id','',true),set_config('med_tracker.current_person_id','',true),set_config('med_tracker.current_household_id','',true),set_config('med_tracker.current_membership_id','',true),set_config('med_tracker.current_invitation_token_digest','',true),set_config('med_tracker.identity_factor_id','',true)").await?;
            transaction.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                "SELECT med_tracker.append_denied_account_clinical_access($1,$2,'self_medication_history.download',$3,$4,$5::uuid,$6)",
                [self.account.get().into(),self.person.get().into(),self.provenance.method.label().into(),self.provenance.reference.clone().into(),self.trace.to_string().into(),match denial {AccessDenial::Unauthenticated=>401_i32,AccessDenial::Forbidden=>403_i32}.into()])).await?;
            Ok::<_,sea_orm::DbErr>(())
        }.await;
        match result {
            Ok(()) => transaction.commit().await.map_err(Into::into),
            Err(error) => {
                let error = OperationError::from(error);
                if let Err(rollback) = transaction.rollback().await {
                    let _ = OperationError::from(rollback);
                }
                Err(error)
            }
        }
    }
}

pub(crate) async fn record_read(
    transaction: &DatabaseTransaction,
    provenance: &CredentialProvenance,
    trace: uuid::Uuid,
) -> Result<(), OperationError> {
    transaction.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT med_tracker.append_account_clinical_access('self_medication_history.read',$1,$2,$3::uuid)",
        [provenance.method.label().into(),provenance.reference.clone().into(),trace.to_string().into()]
    )).await?;
    Ok(())
}
