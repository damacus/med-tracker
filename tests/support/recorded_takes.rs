use med_tracker::models::ids::{
    AccountId, HouseholdId, HouseholdMembershipId, MedicationId, TakeId,
};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, DbErr, Statement,
    TransactionTrait,
};

pub struct SyntheticActor {
    account_id: AccountId,
    household_id: HouseholdId,
    membership_id: HouseholdMembershipId,
}

impl SyntheticActor {
    pub const fn new(account_id: i64, household_id: i64, membership_id: i64) -> Self {
        Self {
            account_id: AccountId::from_database(account_id),
            household_id: HouseholdId::from_database(household_id),
            membership_id: HouseholdMembershipId::from_database(membership_id),
        }
    }

    pub const fn canonical() -> Self {
        Self::new(71001, 72001, 74001)
    }
}

pub struct TakeSeed {
    take_id: TakeId,
    prescribed_medication_id: MedicationId,
}

impl TakeSeed {
    pub const fn new(take_id: i64, prescribed_medication_id: i64) -> Self {
        Self {
            take_id: TakeId::from_database(take_id),
            prescribed_medication_id: MedicationId::from_database(prescribed_medication_id),
        }
    }
}

pub async fn begin(
    admin: &DatabaseConnection,
    actor: SyntheticActor,
) -> Result<DatabaseTransaction, DbErr> {
    let transaction = admin.begin().await?;
    let configured = transaction.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT set_config('med_tracker.current_account_id',$1,true), set_config('med_tracker.current_household_id',$2,true), set_config('med_tracker.current_membership_id',$3,true)",
        [actor.account_id.to_string().into(), actor.household_id.to_string().into(), actor.membership_id.to_string().into()]
    )).await;
    if let Err(error) = configured {
        transaction.rollback().await?;
        return Err(error);
    }
    Ok(transaction)
}

pub async fn capture(
    transaction: &DatabaseTransaction,
    take_id: TakeId,
    prescribed_medication_id: MedicationId,
) -> Result<(), DbErr> {
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT med_tracker.append_take_history($1,$2)",
            [take_id.get().into(), prescribed_medication_id.get().into()],
        ))
        .await?;
    Ok(())
}

pub async fn seed(
    admin: &DatabaseConnection,
    actor: SyntheticActor,
    sql: &str,
    takes: &[TakeSeed],
) -> Result<(), DbErr> {
    let transaction = begin(admin, actor).await?;
    let result = async {
        transaction.execute_unprepared(sql).await?;
        for take in takes {
            capture(&transaction, take.take_id, take.prescribed_medication_id).await?;
        }
        Ok::<_, DbErr>(())
    }
    .await;
    match result {
        Ok(()) => transaction.commit().await,
        Err(error) => {
            transaction.rollback().await?;
            Err(error)
        }
    }
}
