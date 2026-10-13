use crate::models::errors::OperationError;
use chrono::{DateTime, Utc};
use loco_rs::app::AppContext;
use sea_orm::{ConnectionTrait, DbBackend, Statement};

pub(crate) async fn terminal_cleanup(
    ctx: &AppContext,
    household_id: i64,
    account_id: Option<i64>,
    now: DateTime<Utc>,
) -> Result<(), OperationError> {
    let transaction = super::super::scheduler::household_transaction(ctx, household_id).await?;
    transaction.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "WITH candidates AS (
            SELECT d.id FROM notification_deliveries d
            WHERE d.household_id=$1 AND ($2::bigint IS NULL OR d.account_id=$2)
              AND ((d.status='sending' AND d.claimed_at<=$3)
                OR (d.status='pending' AND NOT EXISTS (
                    SELECT 1 FROM household_memberships m JOIN accounts a ON a.id=m.account_id
                    JOIN households h ON h.id=m.household_id
                    WHERE m.household_id=d.household_id AND m.account_id=d.account_id
                      AND m.status='active' AND m.revoked_at IS NULL AND a.status=2
                      AND h.status='active' AND h.lifecycle_state='active')))
            ORDER BY d.id LIMIT 100 FOR UPDATE OF d SKIP LOCKED
        ), updated AS (
            UPDATE notification_deliveries d SET
                status=CASE WHEN d.status='sending' THEN 'uncertain' ELSE 'invalid' END,
                outcome=CASE WHEN d.status='sending' THEN 'abandoned_claim' ELSE 'recipient_revoked' END,
                claimed_at=NULL,updated_at=timezone('UTC',clock_timestamp())
            FROM candidates c WHERE d.id=c.id AND d.household_id=$1 AND d.status IN ('pending','sending')
            RETURNING d.id,d.status,d.outcome
        ) INSERT INTO security_audit_events(household_id,event_type,metadata,audit_context,created_at,updated_at)
          SELECT $1,'browser_push.reminder.' || status,
                 jsonb_build_object('delivery_id',id,'reason',outcome),jsonb_build_object('household_id',$1),
                 timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()) FROM updated",
        [household_id.into(),account_id.into(),(now-chrono::Duration::seconds(120)).naive_utc().into()]
    )).await?;
    transaction.commit().await?;
    Ok(())
}
