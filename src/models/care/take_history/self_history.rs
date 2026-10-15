use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseTransaction, DbBackend, FromQueryResult, Statement};

use super::{Projection, RecordedTake};
use crate::models::{errors::OperationError, ids::HistoryEntryId};

const MIN_POSTGRES_UNIX_MICROS: i64 = -210_866_803_200_000_000;

pub(crate) struct Query {
    after: Option<Cursor>,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit: u32,
}

impl Query {
    pub(crate) fn continue_after(&mut self, cursor: Cursor) {
        self.after = Some(cursor);
    }

    pub(crate) fn parse(
        after: Option<&str>,
        start: Option<&str>,
        end: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Self, OperationError> {
        let invalid = || {
            crate::models::profile::validation(
                "history",
                "requires a positive cursor, a limit of 1 to 100, and ordered RFC3339 date bounds",
            )
        };
        let after = after
            .map(|value| {
                let (instant, id) = value.split_once('.').ok_or_else(invalid)?;
                let micros = instant.parse::<i64>().map_err(|_| invalid())?;
                let occurred_at = DateTime::from_timestamp_micros(micros).ok_or_else(invalid)?;
                let id = id.parse::<i64>().map_err(|_| invalid())?;
                if id <= 0 || !storage_instant(occurred_at) {
                    return Err(invalid());
                }
                Ok(Cursor {
                    occurred_at,
                    id: HistoryEntryId::from_database(id),
                })
            })
            .transpose()?;
        let timestamp = |value: &str| {
            let time = DateTime::parse_from_rfc3339(value)
                .map_err(|_| invalid())?
                .with_timezone(&Utc);
            if !storage_instant(time) {
                return Err(invalid());
            }
            Ok(time)
        };
        let start = start.map(timestamp).transpose()?;
        let end = end.map(timestamp).transpose()?;
        let limit = limit.unwrap_or(50);
        if !(1..=100).contains(&limit) || start.zip(end).is_some_and(|(start, end)| start >= end) {
            return Err(invalid());
        }
        Ok(Self {
            after,
            start,
            end,
            limit,
        })
    }
}

fn storage_instant(time: DateTime<Utc>) -> bool {
    time.timestamp_micros() >= MIN_POSTGRES_UNIX_MICROS
        && time.timestamp_subsec_nanos() < 1_000_000_000
        && time.timestamp_subsec_nanos().is_multiple_of(1_000)
}

pub(crate) struct Cursor {
    occurred_at: DateTime<Utc>,
    id: HistoryEntryId,
}

impl std::fmt::Display for Cursor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}.{}",
            self.occurred_at.timestamp_micros(),
            self.id
        )
    }
}

pub(crate) struct Entry {
    occurred_at: DateTime<Utc>,
    pub history_entry_id: HistoryEntryId,
    pub recorded_at: chrono::NaiveDateTime,
    pub stock_name: Option<String>,
    pub take: RecordedTake,
}

pub(crate) struct Page {
    pub entries: Vec<Entry>,
    pub next_cursor: Option<Cursor>,
}

pub(crate) async fn read(
    transaction: &DatabaseTransaction,
    query: &Query,
) -> Result<Page, OperationError> {
    let requested = i32::try_from(query.limit + 1).map_err(|_| OperationError::Unavailable)?;
    let rows = transaction
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.self_medication_history($1,$2,$3,$4,$5)",
            [
                query.after.as_ref().map(|cursor| cursor.occurred_at).into(),
                query.after.as_ref().map(|cursor| cursor.id.get()).into(),
                query.start.into(),
                query.end.into(),
                requested.into(),
            ],
        ))
        .await?;
    let limit = usize::try_from(query.limit).map_err(|_| OperationError::Unavailable)?;
    let more = rows.len() > limit;
    let mut entries = Vec::with_capacity(rows.len().min(limit));
    for row in rows.into_iter().take(limit) {
        entries.push(Entry {
            occurred_at: row.try_get("", "occurred_at")?,
            history_entry_id: HistoryEntryId::from_database(row.try_get("", "history_entry_id")?),
            recorded_at: row.try_get("", "recorded_at")?,
            stock_name: row.try_get("", "stock_name")?,
            take: RecordedTake::try_from(Projection::from_query_result(&row, "")?)?,
        });
    }
    let next_cursor = if more {
        entries.last().map(|entry| Cursor {
            occurred_at: entry.occurred_at,
            id: entry.history_entry_id,
        })
    } else {
        None
    };
    Ok(Page {
        entries,
        next_cursor,
    })
}
