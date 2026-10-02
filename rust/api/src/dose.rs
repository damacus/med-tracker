mod access;
mod audit;
mod history;
mod input;
mod locking;
mod persistence;
mod replay;
mod responses;
mod source;
mod stock;
mod timing;
mod writing;

use crate::dosage_options::{parse_decimal, valid_identifier};
use crate::entities::{
    dosage, grant, location, medication, medication_take, person_medication, schedule,
    security_audit_event,
};
use crate::sync_events::{record_change, SyncRecord};
use crate::{authenticate, decimal_string, scope, ApiError, AppState, AuthContext, CredentialKind};
use axum::extract::{
    rejection::{JsonRejection, QueryRejection},
    Path, Query, State,
};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseTransaction, DbBackend,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait, Set, Statement,
    TransactionTrait,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use uuid::Uuid;

use access::allowed_person;
pub(super) use access::authorize_sync_replay;
use audit::audit;
use audit::record_domain_audit;
use audit::stock_version;
use audit::StockVersionChange;
pub use history::index;
pub(super) use history::serialize;
pub(super) use history::take_etag;
use input::decimal_from_json;
use input::parse_input_time;
use input::prepare;
#[cfg(test)]
use input::valid_numeric_10_2;
use input::ProposedTake;
use locking::lock_row;
use persistence::insert_take;
use replay::lock_client_uuid;
use replay::replay_matches;
use responses::database_error;
use responses::error;
use responses::request_error_response;
use responses::success_response;
use responses::take_error_response;
pub(super) use responses::TakeFailure;
use responses::TakeFailureCause;
use source::app_zone;
pub(crate) use source::config_decimal;
pub(crate) use source::config_value;
use source::effective_source;
pub(super) use source::local_date;
use source::local_date_in_zone;
use source::source;
use source::source_from_assignment;
use source::source_from_schedule;
use source::Source;
use stock::decrement_stock;
pub(super) use stock::same_stock_signature;
use timing::applies_on;
#[cfg(test)]
use timing::cycle_bounds_in_zone;
use timing::timing_allowed;
pub use writing::create;
pub(super) use writing::create_in_transaction;
pub(super) use writing::create_with_failure;

#[derive(Deserialize)]
pub struct Pagination {
    page: Option<i64>,
    per_page: Option<i64>,
    updated_since: Option<String>,
}

pub(super) fn quantity(amount: Decimal, unit: &str) -> Decimal {
    if matches!(
        unit,
        "tablet" | "capsule" | "gummy" | "sachet" | "spray" | "drop" | "pad" | "ml"
    ) {
        amount
    } else {
        Decimal::ONE
    }
}

pub(super) fn same_dosage_signature(left: &dosage::Model, right: &dosage::Model) -> bool {
    left.amount == right.amount
        && left.unit == right.unit
        && left.frequency == right.frequency
        && left.description.as_deref().unwrap_or("") == right.description.as_deref().unwrap_or("")
        && left.default_for_adults == right.default_for_adults
        && left.default_for_children == right.default_for_children
        && left.default_max_daily_doses == right.default_max_daily_doses
        && left.default_min_hours_between_doses == right.default_min_hours_between_doses
        && left.default_dose_cycle == right.default_dose_cycle
}

pub(super) fn sufficient_stock(supply: Option<Decimal>, amount: Decimal, unit: &str) -> bool {
    supply.is_none_or(|value| value >= quantity(amount, unit))
}

pub(super) fn selected_tracked_dosage<'a>(
    tracked: &'a [dosage::Model],
    inventory_id: i64,
    source_option: Option<&dosage::Model>,
    source_amount: Option<Decimal>,
    source_unit: Option<&str>,
) -> Option<&'a dosage::Model> {
    let mut matches = tracked.iter().filter(|option| {
        if let Some(source_option) = source_option {
            if source_option.medication_id == inventory_id {
                option.id == source_option.id
            } else {
                same_dosage_signature(option, source_option)
            }
        } else {
            Some(option.amount) == source_amount && Some(option.unit.as_str()) == source_unit
        }
    });
    let selected = matches.next()?;
    matches.next().is_none().then_some(selected)
}

#[cfg(test)]
mod tests {
    use super::{cycle_bounds_in_zone, parse_input_time, valid_numeric_10_2};
    use chrono::NaiveDate;
    use sea_orm::prelude::Decimal;
    use std::str::FromStr;

    #[test]
    fn numeric_10_2_accepts_the_full_column_range_without_rounding() {
        assert!(valid_numeric_10_2(
            Decimal::from_str("99999999.99").unwrap()
        ));
        assert!(!valid_numeric_10_2(
            Decimal::from_str("100000000.00").unwrap()
        ));
        assert!(!valid_numeric_10_2(Decimal::from_str("1.001").unwrap()));
    }

    #[test]
    fn taken_at_replay_uses_postgresql_microsecond_precision() {
        let input = serde_json::json!("2026-09-28T10:11:12.123456789Z");
        let expected = NaiveDate::from_ymd_opt(2026, 9, 28)
            .unwrap()
            .and_hms_micro_opt(10, 11, 12, 123456)
            .unwrap();
        assert_eq!(parse_input_time(Some(&input)).unwrap(), expected);
    }

    #[test]
    fn london_monthly_cycle_uses_local_month_boundaries() {
        let time = NaiveDate::from_ymd_opt(2026, 9, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let (start, end) = cycle_bounds_in_zone(time, Some(2), chrono_tz::Europe::London);
        assert_eq!(
            start,
            NaiveDate::from_ymd_opt(2026, 8, 31)
                .unwrap()
                .and_hms_opt(23, 0, 0)
                .unwrap()
        );
        assert_eq!(
            end,
            NaiveDate::from_ymd_opt(2026, 9, 30)
                .unwrap()
                .and_hms_opt(23, 0, 0)
                .unwrap()
        );
    }

    #[test]
    fn london_daily_cycle_observes_spring_dst_shift() {
        let time = NaiveDate::from_ymd_opt(2026, 3, 29)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let (start, end) = cycle_bounds_in_zone(time, Some(0), chrono_tz::Europe::London);
        assert_eq!(
            start,
            NaiveDate::from_ymd_opt(2026, 3, 29)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
        );
        assert_eq!(
            end,
            NaiveDate::from_ymd_opt(2026, 3, 29)
                .unwrap()
                .and_hms_opt(23, 0, 0)
                .unwrap()
        );
    }
}
