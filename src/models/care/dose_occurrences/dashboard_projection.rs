use super::*;

#[derive(Default)]
pub(super) struct DosePreviews {
    by_date: BTreeMap<NaiveDate, doses::BrowserDosePreview>,
}

impl DosePreviews {
    pub(super) fn for_source(
        &mut self,
        source: &Source,
        day: NaiveDate,
    ) -> Option<doses::BrowserDosePreview> {
        match source {
            Source::Schedule(source) => {
                Some(self.for_date(day, || doses::browser_schedule_preview(source.clone(), day)))
            }
            Source::Assignment(_) => None,
        }
    }

    fn for_date(
        &mut self,
        day: NaiveDate,
        preview: impl FnOnce() -> doses::BrowserDosePreview,
    ) -> doses::BrowserDosePreview {
        self.by_date.entry(day).or_insert_with(preview).clone()
    }
}

pub(crate) struct DashboardOccurrence {
    pub person_id: i64,
    pub medication_id: i64,
    pub source_id: i64,
    pub source_type: &'static str,
    pub window_start: NaiveDate,
    pub window_end: NaiveDate,
    pub scheduled_at: Option<NaiveDateTime>,
    pub position: i32,
    pub expected: bool,
    pub actionable: bool,
    pub outcome: String,
    pub occurrence_key: String,
    pub occurrence_etag: Option<String>,
    pub medication_take: Option<medication_take::Model>,
    pub dose: Option<doses::BrowserDosePreview>,
}

fn viewer_interval(
    start: NaiveDate,
    end: NaiveDate,
    settings: Option<&Value>,
) -> (NaiveDateTime, NaiveDateTime) {
    (
        local_midnight(start, settings),
        local_midnight(end + Duration::days(1), settings),
    )
}

pub(super) fn scan_bounds(
    source: &Source,
    start: NaiveDate,
    end: NaiveDate,
    dashboard: bool,
    settings: Option<&Value>,
) -> (NaiveDate, NaiveDate) {
    if !dashboard {
        return projection::scan_bounds(source, start, end);
    }
    let (first, finish) = viewer_interval(start, end, settings);
    let source_settings = source.calendar_settings(settings);
    projection::scan_bounds(
        source,
        local_date(first, source_settings.as_ref()),
        local_date(finish - Duration::microseconds(1), source_settings.as_ref()),
    )
}

pub(super) fn includes(
    source: &Source,
    occurrence: &Occurrence,
    start: NaiveDate,
    end: NaiveDate,
    settings: Option<&Value>,
) -> bool {
    let (first, finish) = viewer_interval(start, end, settings);
    if let Some(scheduled_at) = occurrence.scheduled_at {
        return first <= scheduled_at && scheduled_at < finish;
    }
    let source_settings = source.calendar_settings(settings);
    local_midnight(occurrence.window_start, source_settings.as_ref()) < finish
        && local_midnight(
            occurrence.window_end + Duration::days(1),
            source_settings.as_ref(),
        ) > first
}

pub(super) fn project(
    report: &ReportOccurrence,
    source: &Source,
    occurrence: &Occurrence,
    takes: &std::collections::HashMap<i64, &medication_take::Model>,
    secret: &Arc<[u8]>,
    settings: Option<&Value>,
    dose: Option<doses::BrowserDosePreview>,
) -> DashboardOccurrence {
    let take_id = occurrence
        .record
        .as_ref()
        .and_then(|record| record.medication_take_id)
        .or(occurrence.legacy_take_id);
    DashboardOccurrence {
        person_id: report.person_id,
        medication_id: report.medication_id,
        source_id: report.source_id,
        source_type: report.source_type,
        window_start: report.window_start,
        window_end: report.window_end,
        scheduled_at: report.scheduled_at,
        position: occurrence.position,
        expected: occurrence.expected,
        actionable: actionable(source, occurrence, settings),
        outcome: report.outcome.clone(),
        occurrence_key: key(secret, source, occurrence.window_start, occurrence.position),
        occurrence_etag: occurrence.record.as_ref().map(record_etag),
        medication_take: take_id.and_then(|id| takes.get(&id).copied()).cloned(),
        dose,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dense_occurrences_share_one_dose_preview_per_source_date() {
        let mut cache = DosePreviews::default();
        let calls = std::cell::Cell::new(0);
        let day: NaiveDate = "2026-11-16".parse().unwrap();
        let config = json!({"times":vec!["08:00";10_000]});
        for _ in 0..10_000 {
            let preview = cache.for_date(day, || {
                calls.set(calls.get() + 1);
                assert_eq!(config.clone()["times"].as_array().unwrap().len(), 10_000);
                doses::BrowserDosePreview {
                    available: true,
                    amount: Some("1".into()),
                    unit: Some("tablet".into()),
                }
            });
            assert_eq!(preview.amount.as_deref(), Some("1"));
        }
        let next = cache.for_date(day + Duration::days(1), || {
            calls.set(calls.get() + 1);
            doses::BrowserDosePreview {
                available: true,
                amount: Some("2".into()),
                unit: Some("tablet".into()),
            }
        });
        assert_eq!(next.amount.as_deref(), Some("2"));
        assert_eq!(calls.get(), 2);
    }

    #[tokio::test]
    async fn viewer_midnight_interval_retains_dst_day_lengths_and_source_dates() {
        for (date, hours) in [("2026-03-29", 23), ("2026-10-25", 25)] {
            with_dashboard_timezone(chrono_tz::Europe::London, async {
                let day = date.parse().unwrap();
                let (start, end) = viewer_interval(day, day, None);
                assert_eq!((end - start).num_hours(), hours);
                let source = json!({"schedule_origin_time_zone":"America/New_York"});
                assert_eq!(local_date(start, Some(&source)), day - Duration::days(1));
                assert_eq!(
                    local_date(end - Duration::microseconds(1), Some(&source)),
                    day
                );
            })
            .await;
        }
    }
}
