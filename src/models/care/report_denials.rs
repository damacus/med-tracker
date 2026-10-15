use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement, TransactionTrait};

use crate::models::{
    access::{self, TenantTransaction},
    care::doses::CredentialProvenance,
    errors::OperationError,
    identity::resource::ValidatedPrincipal,
    ids::{AccountId, HouseholdId, HouseholdMembershipId, PersonId},
};

#[derive(Clone, Copy)]
pub(crate) enum ReportKind {
    Health,
    Reviews,
}

pub(crate) enum Denial {
    Unauthenticated,
    Forbidden,
}

pub(crate) struct AuthenticatedSnapshot {
    account: AccountId,
    person: PersonId,
    membership: HouseholdMembershipId,
    household: HouseholdId,
    role: String,
    permissions_version: i32,
    provenance: CredentialProvenance,
    request_id: String,
    kind: ReportKind,
}

impl AuthenticatedSnapshot {
    pub(crate) async fn capture(
        principal: &ValidatedPrincipal,
        tenant: &TenantTransaction,
        kind: ReportKind,
    ) -> Result<Self, OperationError> {
        if principal.account_id() != tenant.scope().actor.account_id {
            return Err(OperationError::Forbidden);
        }
        access::recheck(tenant).await?;
        Ok(Self {
            account: AccountId::from_database(principal.account_id()),
            person: tenant.person_id(),
            membership: HouseholdMembershipId::from_database(tenant.membership().id),
            household: HouseholdId::from_database(tenant.scope().household_id),
            role: tenant.membership().role.clone(),
            permissions_version: tenant.membership().permissions_version,
            provenance: principal.provenance().clone(),
            request_id: tenant.scope().request_id.clone(),
            kind,
        })
    }

    pub(crate) async fn record(
        self,
        db: &DatabaseConnection,
        denial: Denial,
    ) -> Result<(), OperationError> {
        let transaction = db.begin().await?;
        let result = async {
            transaction.execute_unprepared(
                "SET LOCAL ROLE med_tracker_app; SELECT set_config('med_tracker.current_account_id','',true),set_config('med_tracker.current_person_id','',true),set_config('med_tracker.current_household_id','',true),set_config('med_tracker.current_membership_id','',true),set_config('med_tracker.current_invitation_token_digest','',true),set_config('med_tracker.identity_factor_id','',true)"
            ).await?;
            transaction.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                "SELECT med_tracker.append_denied_report_request($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
                [self.account.get().into(),self.person.get().into(),self.membership.get().into(),self.household.get().into(),self.role.into(),self.permissions_version.into(),self.provenance.method.label().into(),self.provenance.reference.into(),self.request_id.into(),match self.kind {ReportKind::Health=>"health",ReportKind::Reviews=>"reviews"}.into(),match denial {Denial::Unauthenticated=>401_i32,Denial::Forbidden=>403_i32}.into()]
            )).await?;
            Ok::<_,sea_orm::DbErr>(())
        }.await;
        match result {
            Ok(()) => transaction.commit().await.map_err(Into::into),
            Err(error) => Err(failed_record(error, transaction.rollback()).await),
        }
    }
}

async fn failed_record(
    error: DbErr,
    rollback: impl Future<Output = Result<(), DbErr>>,
) -> OperationError {
    let original_error = OperationError::from(error);
    if let Err(rollback_error) = rollback.await {
        let _ = OperationError::from(rollback_error);
    }
    original_error
}

#[cfg(test)]
mod tests {
    use super::failed_record;
    use crate::models::errors::OperationError;
    use sea_orm::{ConnAcquireErr, DbErr, RuntimeErr};
    use std::{
        collections::HashSet,
        sync::{Arc, Mutex},
    };
    use tracing::{
        Event, Metadata, Subscriber,
        field::{Field, Visit},
        span::{Attributes, Id, Record},
    };

    #[derive(Clone, Default)]
    struct CapturedEvents(Arc<Mutex<Vec<Vec<(String, String)>>>>);

    #[derive(Default)]
    struct CapturedFields(Vec<(String, String)>);

    impl Visit for CapturedFields {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.0.push((field.name().to_owned(), format!("{value:?}")));
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            self.0.push((field.name().to_owned(), value.to_owned()));
        }
    }

    impl Subscriber for CapturedEvents {
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &Attributes<'_>) -> Id {
            Id::from_u64(1)
        }
        fn record(&self, _: &Id, _: &Record<'_>) {}
        fn record_follows_from(&self, _: &Id, _: &Id) {}
        fn event(&self, event: &Event<'_>) {
            assert_eq!(*event.metadata().level(), tracing::Level::ERROR);
            let mut fields = CapturedFields::default();
            event.record(&mut fields);
            let Ok(mut events) = self.0.lock() else {
                panic!("captured event lock poisoned");
            };
            events.push(fields.0);
        }
        fn enter(&self, _: &Id) {}
        fn exit(&self, _: &Id) {}
    }

    #[tokio::test(flavor = "current_thread")]
    async fn failed_record_logs_original_and_rollback_categories_without_disclosing_messages() {
        let events = CapturedEvents::default();
        let sentinel = "patient=TEST-CLINICAL;password=TEST-SECRET;SQL=SELECT private";
        let result = {
            let _subscriber = tracing::subscriber::set_default(events.clone());
            failed_record(
                DbErr::Exec(RuntimeErr::Internal(sentinel.to_owned())),
                std::future::ready(Err(DbErr::ConnectionAcquire(ConnAcquireErr::Timeout))),
            )
            .await
        };
        assert_eq!(result, OperationError::Unavailable);
        assert_eq!(result.to_string(), "Operation temporarily unavailable");
        let Ok(recorded) = events.0.lock() else {
            panic!("captured event lock poisoned");
        };
        assert_eq!(recorded.len(), 2);
        let mut ids = HashSet::new();
        for (fields, expected) in recorded.iter().zip(["execution", "timeout"]) {
            assert_eq!(fields.len(), 3);
            assert_eq!(
                fields
                    .iter()
                    .find(|(name, _)| name == "category")
                    .map(|(_, value)| value.as_str()),
                Some(expected)
            );
            let trace = fields
                .iter()
                .find(|(name, _)| name == "error_trace_id")
                .map(|(_, value)| value.as_str())
                .unwrap_or_default();
            assert!(uuid::Uuid::parse_str(trace).is_ok());
            assert!(ids.insert(trace));
            assert_eq!(
                fields
                    .iter()
                    .find(|(name, _)| name == "message")
                    .map(|(_, value)| value.as_str()),
                Some("Database operation failed")
            );
            let rendered = format!("{fields:?}");
            assert!(!rendered.contains(sentinel));
            assert!(!rendered.contains("TEST-CLINICAL"));
            assert!(!rendered.contains("TEST-SECRET"));
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn failed_record_with_successful_rollback_logs_only_original_category() {
        let events = CapturedEvents::default();
        let result = {
            let _subscriber = tracing::subscriber::set_default(events.clone());
            failed_record(
                DbErr::Query(RuntimeErr::Internal("synthetic".into())),
                std::future::ready(Ok(())),
            )
            .await
        };
        assert_eq!(result, OperationError::Unavailable);
        let Ok(recorded) = events.0.lock() else {
            panic!("captured event lock poisoned");
        };
        assert_eq!(recorded.len(), 1);
        assert_eq!(
            recorded
                .first()
                .and_then(|fields| fields.iter().find(|(name, _)| name == "category"))
                .map(|(_, value)| value.as_str()),
            Some("query")
        );
    }
}
