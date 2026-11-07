use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use sea_orm::{ColumnTrait, QueryFilter};

use crate::models::{
    access::{self, TenantTransaction, person_queries},
    authorization,
    care::{guardian_declarations, person_schedules::sharing, sync},
    entities::person,
    errors::OperationError,
    ids::{AccountId, PersonId},
};

pub(crate) struct PreparedImportAuthority<'a> {
    tenant: &'a TenantTransaction,
    today: NaiveDate,
    own: Option<i64>,
    declarations: HashMap<i64, HashSet<i64>>,
    created: HashMap<i64, i64>,
}

fn matches_guardian(
    declarations: &HashMap<i64, HashSet<i64>>,
    created: &HashMap<i64, i64>,
    subject: i64,
    current: i64,
) -> bool {
    declarations
        .get(&subject)
        .is_some_and(|ids| ids.contains(&current))
        || created.get(&subject) == Some(&current)
}

fn captured_subject(
    own: Option<i64>,
    declarations: &HashMap<i64, HashSet<i64>>,
    created: &HashMap<i64, i64>,
    id: i64,
) -> bool {
    own == Some(id) || declarations.contains_key(&id) || created.contains_key(&id)
}

impl<'a> PreparedImportAuthority<'a> {
    pub(crate) async fn prepare(
        tenant: &'a TenantTransaction,
        today: NaiveDate,
    ) -> Result<Self, OperationError> {
        if !tenant
            .holds_exclusive_account(AccountId::from_database(tenant.scope().actor.account_id))
        {
            return Err(OperationError::Forbidden);
        }
        sync::lock(tenant).await?;
        let locked = guardian_declarations::manageable_subject_declarations(tenant).await?;
        let guardian_eligible = match guardian_declarations::eligible_actor(tenant, today).await {
            Ok(_) => true,
            Err(OperationError::Forbidden) => false,
            Err(error) => return Err(error),
        };
        let mut declarations: HashMap<i64, HashSet<i64>> = HashMap::new();
        if guardian_eligible {
            for (subject, declaration) in locked {
                declarations.entry(subject).or_default().insert(declaration);
            }
        }
        let own = person_queries::household_people(tenant.scope())
            .filter(person::Column::Id.eq(tenant.membership().person_id))
            .one(tenant.transaction())
            .await?
            .filter(|person| authorization::own_person_manage(tenant.membership(), person))
            .map(|person| person.id);
        access::recheck(tenant).await?;
        Ok(Self {
            tenant,
            today,
            own,
            declarations,
            created: HashMap::new(),
        })
    }

    pub(crate) fn tenant(&self) -> &'a TenantTransaction {
        self.tenant
    }

    pub(crate) fn check_date(&self, today: NaiveDate) -> Result<(), OperationError> {
        if self.today == today {
            Ok(())
        } else {
            Err(OperationError::Forbidden)
        }
    }

    pub(crate) async fn allows(&self, person: PersonId) -> Result<bool, OperationError> {
        access::recheck(self.tenant).await?;
        let id = person.get();
        if !captured_subject(self.own, &self.declarations, &self.created, id) {
            return Ok(false);
        }
        if !sharing::clinical_authority(self.tenant.transaction(), person).await? {
            return Ok(false);
        }
        if self.own == Some(id) {
            return Ok(true);
        }
        let Some(current) = sharing::guardian(self.tenant.transaction(), person).await? else {
            return Ok(false);
        };
        Ok(matches_guardian(
            &self.declarations,
            &self.created,
            id,
            current,
        ))
    }

    pub(super) async fn current_subjects(&self) -> Result<HashSet<i64>, OperationError> {
        let mut result = HashSet::new();
        for id in self
            .own
            .into_iter()
            .chain(self.declarations.keys().copied())
            .chain(self.created.keys().copied())
        {
            if self.allows(PersonId::from_database(id)).await? {
                result.insert(id);
            }
        }
        Ok(result)
    }

    pub(super) async fn register_created_dependent(
        &mut self,
        id: PersonId,
    ) -> Result<(), OperationError> {
        let declaration = sharing::guardian(self.tenant.transaction(), id)
            .await?
            .ok_or(OperationError::Forbidden)?;
        if !sharing::clinical_authority(self.tenant.transaction(), id).await? {
            return Err(OperationError::Forbidden);
        }
        self.created.insert(id.get(), declaration);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_guardian_must_match_the_subjects_prelocked_or_owned_declaration() {
        let declarations = HashMap::from([(2, HashSet::from([10]))]);
        let created = HashMap::from([(3, 11)]);
        assert!(matches_guardian(&declarations, &created, 2, 10));
        assert!(matches_guardian(&declarations, &created, 3, 11));
        assert!(!matches_guardian(&declarations, &created, 2, 11));
        assert!(!matches_guardian(&declarations, &created, 4, 10));
        assert!(!matches_guardian(&declarations, &created, 2, 12));
    }

    #[test]
    fn self_is_captured_without_guardian_and_does_not_capture_other_subjects() {
        let declarations = HashMap::new();
        let created = HashMap::new();
        assert!(captured_subject(Some(1), &declarations, &created, 1));
        assert!(!captured_subject(Some(1), &declarations, &created, 2));
    }

    #[test]
    fn borrowed_authority_exposes_only_its_own_tenant_lifetime() {
        fn tenant_from_guard<'a>(guard: &PreparedImportAuthority<'a>) -> &'a TenantTransaction {
            guard.tenant()
        }
        let _: for<'a> fn(&PreparedImportAuthority<'a>) -> &'a TenantTransaction =
            tenant_from_guard;
    }
}
