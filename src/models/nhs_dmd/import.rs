use super::extract;
use crate::models::entities::{
    nhs_dmd_amp_trade_family, nhs_dmd_ampp_relationship, nhs_dmd_barcode, nhs_dmd_import,
    nhs_dmd_supplementary_release, nhs_dmd_trade_family, nhs_dmd_trade_family_group,
};
use chrono::{NaiveDate, Utc};
use quick_xml::Reader;
use quick_xml::events::Event;
use regex::Regex;
use sea_orm::prelude::Date;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction,
    DbBackend, EntityTrait, QueryFilter, QuerySelect, Set, Statement, TransactionTrait,
};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::Instant;
use tokio::sync::mpsc;

const PROGRESS_BATCH: i32 = 250;
const INSERT_BATCH: usize = 1000;
static TRAILING_PARENS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*\([^)]*\)\z").expect("valid trailing parentheses expression"));

struct ImportPhaseTimer {
    run_id: i64,
    phase: &'static str,
    started: Instant,
    succeeded: bool,
}

impl ImportPhaseTimer {
    fn new(run_id: i64, phase: &'static str) -> Self {
        Self {
            run_id,
            phase,
            started: Instant::now(),
            succeeded: false,
        }
    }

    fn complete(mut self) {
        self.succeeded = true;
    }
}

impl Drop for ImportPhaseTimer {
    fn drop(&mut self) {
        let duration_ms = self.started.elapsed().as_secs_f64() * 1000.0;
        let outcome = if self.succeeded { "success" } else { "failure" };
        tracing::info!(
            run_id = self.run_id,
            phase = self.phase,
            duration_ms,
            outcome,
            "nhs_dmd_import_timing"
        );
    }
}

#[derive(Default, Clone)]
pub(crate) struct Counts {
    pub processed: i32,
    pub ampp_total: i32,
    pub gtin_total: i32,
    pub created: i32,
    pub updated: i32,
    pub unchanged: i32,
    pub skipped_expired: i32,
    pub skipped_missing_name: i32,
    pub skipped_invalid: i32,
}

impl Counts {
    fn total(&self) -> i32 {
        self.ampp_total + self.gtin_total
    }
    fn imported(&self) -> i32 {
        self.created + self.updated
    }
    fn skipped(&self) -> i32 {
        self.skipped_expired + self.skipped_missing_name + self.skipped_invalid
    }
}

struct AmppName {
    name: String,
    amp_code: Option<String>,
}

enum GtinParsed {
    SkipExpired,
    SkipMissingName,
    SkipInvalid,
    Record {
        gtin: String,
        code: String,
        amp_code: Option<String>,
        display: String,
        vmp_name: Option<String>,
    },
}

pub(crate) fn normalize_gtin(value: &str) -> String {
    value.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn glob_paths(dir: &Path, pattern: &str) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut matches: Vec<PathBuf> = read
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| extract::wildcard_match(pattern, name))
        })
        .collect();
    matches.sort();
    matches
}

fn glob_one(dir: &Path, pattern: &str) -> Result<PathBuf, String> {
    glob_paths(dir, pattern)
        .into_iter()
        .next()
        .ok_or_else(|| format!("No file matching {pattern} in {}", dir.display()))
}

fn count_elements(
    path: &Path,
    element: &str,
    mut progress: impl FnMut() -> Result<(), String>,
) -> Result<i32, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    let mut buffer = Vec::new();
    let mut count = 0_i32;
    let mut last_position = 0;
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event) | Event::Empty(event))
                if event.name().as_ref() == element.as_bytes() =>
            {
                count += 1;
            }
            Ok(Event::Eof) => return Ok(count),
            Ok(_) => {}
            Err(error) => return Err(format!("XML parse failed: {error}")),
        }
        let position = reader.buffer_position();
        if position.saturating_sub(last_position) >= 1024 * 1024 {
            progress()?;
            last_position = position;
        }
        buffer.clear();
    }
}

fn text_fragment(event: Event<'_>) -> Result<String, String> {
    match event {
        Event::Text(event) => {
            let raw = event.xml10_content().map_err(|error| error.to_string())?;
            quick_xml::escape::unescape(&raw)
                .map(|text| text.into_owned())
                .map_err(|error| error.to_string())
        }
        Event::CData(event) => event
            .xml10_content()
            .map(|text| text.into_owned())
            .map_err(|error| error.to_string()),
        Event::GeneralRef(event) => {
            let name = event.decode().map_err(|error| error.to_string())?;
            quick_xml::escape::unescape(&format!("&{name};"))
                .map(|text| text.into_owned())
                .map_err(|error| error.to_string())
        }
        _ => unreachable!(),
    }
}

fn parse_ampp_names(
    path: &Path,
    counts: &mut Counts,
    progress: &mpsc::UnboundedSender<Counts>,
) -> Result<HashMap<String, AmppName>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut names = HashMap::new();
    let mut in_ampp = false;
    let mut ampp_depth = 0;
    let mut current_tag: Option<String> = None;
    let mut nested_depth = 0;
    let mut appid = String::new();
    let mut name = String::new();
    let mut apid = String::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).into_owned();
                if tag == "AMPP" {
                    if in_ampp {
                        return Err("Nested AMPP record in release XML".to_owned());
                    }
                    in_ampp = true;
                    ampp_depth = 1;
                    current_tag = None;
                    nested_depth = 0;
                    appid.clear();
                    name.clear();
                    apid.clear();
                } else if in_ampp {
                    if current_tag.is_some() {
                        nested_depth += 1;
                    } else if ampp_depth == 1 && matches!(tag.as_str(), "APPID" | "NM" | "APID") {
                        current_tag = Some(tag);
                    }
                    ampp_depth += 1;
                }
            }
            Ok(event @ (Event::Text(_) | Event::CData(_) | Event::GeneralRef(_))) => {
                if let Some(tag) = &current_tag {
                    let text = text_fragment(event)?;
                    match tag.as_str() {
                        "APPID" => appid.push_str(&text),
                        "NM" => name.push_str(&text),
                        "APID" => apid.push_str(&text),
                        _ => {}
                    }
                }
            }
            Ok(Event::Empty(event)) if event.name().as_ref() == b"AMPP" => {
                counts.processed += 1;
                emit_progress(progress, counts, false);
            }
            Ok(Event::End(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).into_owned();
                if tag == "AMPP" {
                    in_ampp = false;
                    ampp_depth = 0;
                    counts.processed += 1;
                    if !appid.trim().is_empty() && !name.trim().is_empty() {
                        names.insert(
                            appid.clone(),
                            AmppName {
                                name: name.clone(),
                                amp_code: (!apid.is_empty()).then(|| apid.clone()),
                            },
                        );
                    }
                    emit_progress(progress, counts, false);
                } else if in_ampp {
                    ampp_depth -= 1;
                    if current_tag.is_some() {
                        if nested_depth == 0 {
                            current_tag = None;
                        } else {
                            nested_depth -= 1;
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(format!("AMPP parse failed: {error}")),
        }
        buffer.clear();
    }
    Ok(names)
}

fn parse_gtins(
    path: &Path,
    names: &HashMap<String, AmppName>,
    today: NaiveDate,
    counts: &mut Counts,
    progress: &mpsc::UnboundedSender<Counts>,
) -> Result<Vec<GtinParsed>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut records = Vec::new();
    let mut in_ampp = false;
    let mut in_gtindata = false;
    let mut ampp_depth = 0;
    let mut gtindata_depth = 0;
    let mut current_tag: Option<String> = None;
    let mut nested_depth = 0;
    let mut amppid = String::new();
    let mut gtin = String::new();
    let mut enddt = String::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).into_owned();
                if tag == "AMPP" {
                    if in_ampp {
                        return Err("Nested AMPP record in GTIN XML".to_owned());
                    }
                    in_ampp = true;
                    ampp_depth = 1;
                    in_gtindata = false;
                    current_tag = None;
                    nested_depth = 0;
                    amppid.clear();
                } else if tag == "GTINDATA" && in_ampp {
                    if in_gtindata {
                        return Err("Nested GTINDATA record in GTIN XML".to_owned());
                    }
                    in_gtindata = true;
                    ampp_depth += 1;
                    gtindata_depth = 1;
                    current_tag = None;
                    nested_depth = 0;
                    gtin.clear();
                    enddt.clear();
                } else if in_ampp {
                    if current_tag.is_some() {
                        nested_depth += 1;
                    } else if (!in_gtindata && ampp_depth == 1 && tag == "AMPPID")
                        || (in_gtindata
                            && gtindata_depth == 1
                            && matches!(tag.as_str(), "GTIN" | "ENDDT"))
                    {
                        current_tag = Some(tag);
                    }
                    ampp_depth += 1;
                    if in_gtindata {
                        gtindata_depth += 1;
                    }
                }
            }
            Ok(event @ (Event::Text(_) | Event::CData(_) | Event::GeneralRef(_))) => {
                if let Some(tag) = &current_tag {
                    let text = text_fragment(event)?;
                    match tag.as_str() {
                        "AMPPID" if !in_gtindata => amppid.push_str(&text),
                        "GTIN" if in_gtindata => gtin.push_str(&text),
                        "ENDDT" if in_gtindata => enddt.push_str(&text),
                        _ => {}
                    }
                }
            }
            Ok(Event::Empty(event)) if event.name().as_ref() == b"GTINDATA" && in_ampp => {
                counts.processed += 1;
                records.push(GtinParsed::SkipInvalid);
                emit_progress(progress, counts, false);
            }
            Ok(Event::End(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).into_owned();
                if tag == "AMPP" {
                    in_ampp = false;
                    ampp_depth = 0;
                } else if tag == "GTINDATA" && in_gtindata {
                    in_gtindata = false;
                    ampp_depth -= 1;
                    gtindata_depth = 0;
                    counts.processed += 1;
                    records.push(classify_gtin(&amppid, &gtin, &enddt, names, today)?);
                    emit_progress(progress, counts, false);
                } else if in_ampp {
                    ampp_depth -= 1;
                    if in_gtindata {
                        gtindata_depth -= 1;
                    }
                    if current_tag.is_some() {
                        if nested_depth == 0 {
                            current_tag = None;
                        } else {
                            nested_depth -= 1;
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(format!("GTIN parse failed: {error}")),
        }
        buffer.clear();
    }
    Ok(records)
}

fn classify_gtin(
    amppid: &str,
    gtin: &str,
    enddt: &str,
    names: &HashMap<String, AmppName>,
    today: NaiveDate,
) -> Result<GtinParsed, String> {
    if gtin.trim().is_empty() {
        return Ok(GtinParsed::SkipInvalid);
    }
    let enddt = enddt.trim();
    if !enddt.is_empty() {
        let date = NaiveDate::parse_from_str(enddt, "%Y-%m-%d")
            .map_err(|error| format!("Invalid GTIN ENDDT: {error}"))?;
        if date <= today {
            return Ok(GtinParsed::SkipExpired);
        }
    }
    if amppid.is_empty() {
        return Ok(GtinParsed::SkipMissingName);
    }
    let Some(ampp) = names.get(amppid) else {
        return Ok(GtinParsed::SkipMissingName);
    };
    let display = ampp.name.clone();
    let stripped = strip_trailing_parens(&display);
    Ok(GtinParsed::Record {
        gtin: normalize_gtin(gtin),
        code: amppid.to_owned(),
        amp_code: ampp.amp_code.clone(),
        vmp_name: (stripped != display).then_some(stripped),
        display,
    })
}

fn strip_trailing_parens(display: &str) -> String {
    TRAILING_PARENS.replace(display, "").trim().to_owned()
}

fn emit_progress(progress: &mpsc::UnboundedSender<Counts>, counts: &Counts, force: bool) {
    if force || (counts.processed > 0 && counts.processed % PROGRESS_BATCH == 0) {
        let _ = progress.send(counts.clone());
    }
}

struct SupplementaryFiles {
    groups: PathBuf,
    families: PathBuf,
    memberships: PathBuf,
}

fn supplementary_files(dir: &Path) -> Option<SupplementaryFiles> {
    let groups = optional_glob(dir, "f_trade_family_group2_0*.xml")?;
    let families = optional_glob(dir, "f_trade_family2_0*.xml")?;
    let memberships = optional_glob(dir, "f_amp_trade_family2_0*.xml")?;
    Some(SupplementaryFiles {
        groups,
        families,
        memberships,
    })
}

fn optional_glob(dir: &Path, pattern: &str) -> Option<PathBuf> {
    let matches = glob_paths(dir, pattern);
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

fn release_date_from_filename(path: &Path) -> Option<Date> {
    let name = path.file_name()?.to_str()?;
    let stem = name.strip_suffix(".xml")?;
    let suffix = &stem[stem.len().checked_sub(6)?..];
    if !suffix.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let day: u32 = suffix[0..2].parse().ok()?;
    let month: u32 = suffix[2..4].parse().ok()?;
    let year: i32 = 2000 + suffix[4..6].parse::<i32>().ok()?;
    NaiveDate::from_ymd_opt(year, month, day)
}

struct TradeFamilyData {
    groups: Vec<(String, String)>,
    families: Vec<(String, String, Option<String>)>,
    memberships: Vec<(String, String)>,
    released_on: Option<Date>,
}

fn parse_supplementary(files: &SupplementaryFiles) -> Result<Option<TradeFamilyData>, String> {
    let groups = parse_code_names(&files.groups, "TRADE_FAMILY_GROUP", "TFGID")?;
    let families = parse_families(&files.families)?;
    let memberships = parse_memberships(&files.memberships)?;
    if groups.is_empty() || families.is_empty() {
        return Ok(None);
    }
    let group_codes: HashSet<&str> = groups.iter().map(|(code, _)| code.as_str()).collect();
    let family_codes: HashSet<&str> = families.iter().map(|(code, _, _)| code.as_str()).collect();
    let membership_codes: HashSet<&str> = memberships.iter().map(|(amp, _)| amp.as_str()).collect();
    if group_codes.len() != groups.len()
        || family_codes.len() != families.len()
        || membership_codes.len() != memberships.len()
        || families.iter().any(|(_, _, group)| {
            group
                .as_deref()
                .is_some_and(|code| !group_codes.contains(code))
        })
        || memberships
            .iter()
            .any(|(_, family)| !family_codes.contains(family.as_str()))
    {
        return Ok(None);
    }
    let released_on = [&files.groups, &files.families, &files.memberships]
        .into_iter()
        .filter_map(|file| release_date_from_filename(file))
        .max();
    Ok(Some(TradeFamilyData {
        groups,
        families,
        memberships,
        released_on,
    }))
}

fn parse_optional_supplementary(files: &SupplementaryFiles) -> Option<TradeFamilyData> {
    match parse_supplementary(files) {
        Ok(data) => data,
        Err(error) => {
            tracing::warn!(error = %error, "NHS dm+d supplementary metadata could not be read");
            None
        }
    }
}

fn parse_code_names(
    path: &Path,
    element: &str,
    code_tag: &str,
) -> Result<Vec<(String, String)>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut rows = Vec::new();
    let mut in_element = false;
    let mut current_tag: Option<String> = None;
    let mut code = String::new();
    let mut name = String::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).into_owned();
                if tag == element {
                    in_element = true;
                    code.clear();
                    name.clear();
                } else if in_element {
                    current_tag = Some(tag);
                }
            }
            Ok(event @ (Event::Text(_) | Event::CData(_) | Event::GeneralRef(_))) => {
                if let Some(tag) = &current_tag {
                    let text = text_fragment(event)?;
                    if tag.as_str() == code_tag {
                        code.push_str(&text);
                    } else if tag == "NM" {
                        name.push_str(&text);
                    }
                }
            }
            Ok(Event::End(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).into_owned();
                if tag == element {
                    in_element = false;
                    if !code.trim().is_empty() && !name.trim().is_empty() {
                        rows.push((code.clone(), name.clone()));
                    }
                } else if current_tag.as_deref() == Some(tag.as_str()) {
                    current_tag = None;
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(format!("supplementary parse failed: {error}")),
        }
        buffer.clear();
    }
    Ok(rows)
}

fn parse_families(path: &Path) -> Result<Vec<(String, String, Option<String>)>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut rows = Vec::new();
    let mut in_element = false;
    let mut current_tag: Option<String> = None;
    let mut code = String::new();
    let mut name = String::new();
    let mut group = String::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).into_owned();
                if tag == "TRADE_FAMILY" {
                    in_element = true;
                    code.clear();
                    name.clear();
                    group.clear();
                } else if in_element {
                    current_tag = Some(tag);
                }
            }
            Ok(event @ (Event::Text(_) | Event::CData(_) | Event::GeneralRef(_))) => {
                if let Some(tag) = &current_tag {
                    let text = text_fragment(event)?;
                    match tag.as_str() {
                        "TFID" => code.push_str(&text),
                        "NM" => name.push_str(&text),
                        "TFGID" => group.push_str(&text),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).into_owned();
                if tag == "TRADE_FAMILY" {
                    in_element = false;
                    if !code.trim().is_empty() && !name.trim().is_empty() {
                        rows.push((
                            code.clone(),
                            name.clone(),
                            (!group.trim().is_empty()).then(|| group.clone()),
                        ));
                    }
                } else if current_tag.as_deref() == Some(tag.as_str()) {
                    current_tag = None;
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(format!("trade family parse failed: {error}")),
        }
        buffer.clear();
    }
    Ok(rows)
}

fn parse_memberships(path: &Path) -> Result<Vec<(String, String)>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut rows = Vec::new();
    let mut in_element = false;
    let mut current_tag: Option<String> = None;
    let mut amp = String::new();
    let mut family = String::new();
    let mut startdt = String::new();
    let mut enddt = String::new();
    let today = Utc::now().date_naive();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).into_owned();
                if tag == "AMP_TRADE_FAMILY" {
                    in_element = true;
                    amp.clear();
                    family.clear();
                    startdt.clear();
                    enddt.clear();
                } else if in_element {
                    current_tag = Some(tag);
                }
            }
            Ok(event @ (Event::Text(_) | Event::CData(_) | Event::GeneralRef(_))) => {
                if let Some(tag) = &current_tag {
                    let text = text_fragment(event)?;
                    match tag.as_str() {
                        "APID" => amp.push_str(&text),
                        "TFID" => family.push_str(&text),
                        "STARTDT" => startdt.push_str(&text),
                        "ENDDT" => enddt.push_str(&text),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).into_owned();
                if tag == "AMP_TRADE_FAMILY" {
                    in_element = false;
                    if membership_active(&startdt, &enddt, today)
                        && !amp.trim().is_empty()
                        && !family.trim().is_empty()
                    {
                        rows.push((amp.clone(), family.clone()));
                    }
                } else if current_tag.as_deref() == Some(tag.as_str()) {
                    current_tag = None;
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(format!("AMP trade family parse failed: {error}")),
        }
        buffer.clear();
    }
    Ok(rows)
}

fn membership_active(startdt: &str, enddt: &str, today: NaiveDate) -> bool {
    let parse = |value: &str| NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").ok();
    if !startdt.is_empty() && parse(startdt).is_none() {
        return false;
    }
    if !enddt.is_empty() && parse(enddt).is_none() {
        return false;
    }
    parse(startdt).is_none_or(|date| date <= today) && parse(enddt).is_none_or(|date| date > today)
}

async fn drain_activity(
    db: &DatabaseConnection,
    run_id: i64,
    mut receiver: mpsc::UnboundedReceiver<()>,
) -> Result<(), String> {
    while receiver.recv().await.is_some() {
        touch_run(db, run_id).await?;
    }
    Ok(())
}

async fn blocking_with_activity<T, F>(
    db: &DatabaseConnection,
    run_id: i64,
    operation: F,
) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(mpsc::UnboundedSender<()>) -> Result<T, String> + Send + 'static,
{
    let (sender, receiver) = mpsc::unbounded_channel();
    let work = tokio::task::spawn_blocking(move || operation(sender));
    let activity = drain_activity(db, run_id, receiver);
    finish_work(run_id, "activity", work, activity).await
}

async fn finish_work<T>(
    run_id: i64,
    phase: &'static str,
    work: tokio::task::JoinHandle<Result<T, String>>,
    activity: impl std::future::Future<Output = Result<(), String>>,
) -> Result<T, String> {
    let (result, liveness) = tokio::join!(work, activity);
    let result = result.unwrap_or_else(|error| {
        Err(if error.is_panic() {
            "Import worker panicked".to_owned()
        } else {
            "Import worker was cancelled".to_owned()
        })
    });
    if result.is_err() && liveness.is_err() {
        tracing::warn!(
            run_id,
            phase,
            secondary_failure = "activity_reporting",
            "NHS import activity reporting also failed"
        );
    }
    let value = result?;
    liveness?;
    Ok(value)
}

pub(crate) async fn run_import(
    db: DatabaseConnection,
    run_id: i64,
    zip_path: PathBuf,
) -> Result<(), String> {
    let total = ImportPhaseTimer::new(run_id, "total");
    let result = async {
        let extraction = ImportPhaseTimer::new(run_id, "extract");
        set_status(&db, run_id, 1, "Extracting release archive").await?;
        let zip_for_task = zip_path.clone();
        let extracted = blocking_with_activity(&db, run_id, move |activity| {
            let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
            extract::extract_with_progress(&zip_for_task, dir.path(), None, || {
                activity
                    .send(())
                    .map_err(|_| "Import activity could not be reported".to_owned())
            })
            .map(|_| dir)
        })
        .await?;
        extraction.complete();
        import_extracted(&db, run_id, extracted.path()).await
    }
    .await;
    if result.is_ok() {
        total.complete();
    }
    result
}

async fn set_status(
    db: &DatabaseConnection,
    run_id: i64,
    status: i32,
    message: &str,
) -> Result<(), String> {
    let changed = db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE nhs_dmd_imports SET status = $1, started_at = COALESCE(started_at, now()), log = COALESCE(log, '') || $2 || E'\\n', updated_at = now() WHERE id = $3 AND status < 4",
        [status.into(), message.into(), run_id.into()],
    ))
    .await
    .map_err(|error| error.to_string())?;
    if changed.rows_affected() == 1 {
        Ok(())
    } else {
        Err("Import is no longer active".into())
    }
}

async fn apply_progress(
    db: &DatabaseConnection,
    run_id: i64,
    counts: &Counts,
    status: i32,
) -> Result<(), String> {
    let changed = db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE nhs_dmd_imports SET status = $1, total_records = $2, processed_records = $3, imported_count = $4, skipped_count = $5, created_count = $6, updated_count = $7, unchanged_count = $8, skipped_expired_count = $9, skipped_missing_name_count = $10, skipped_invalid_count = $11, updated_at = now() WHERE id = $12 AND status < 4",
        [
            status.into(),
            counts.total().into(),
            counts.processed.into(),
            counts.imported().into(),
            counts.skipped().into(),
            counts.created.into(),
            counts.updated.into(),
            counts.unchanged.into(),
            counts.skipped_expired.into(),
            counts.skipped_missing_name.into(),
            counts.skipped_invalid.into(),
            run_id.into(),
        ],
    ))
    .await
    .map_err(|error| error.to_string())?;
    if changed.rows_affected() == 1 {
        Ok(())
    } else {
        Err("Import is no longer active".into())
    }
}

async fn touch_run(db: &impl ConnectionTrait, run_id: i64) -> Result<(), String> {
    let changed = db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE nhs_dmd_imports SET updated_at = now() WHERE id = $1 AND status < 4",
            [run_id.into()],
        ))
        .await
        .map_err(|error| error.to_string())?;
    if changed.rows_affected() == 1 {
        Ok(())
    } else {
        Err("Import is no longer active".into())
    }
}

async fn drain_progress(
    db: &DatabaseConnection,
    run_id: i64,
    mut receiver: mpsc::UnboundedReceiver<Counts>,
) -> Result<(), String> {
    while let Some(snapshot) = receiver.recv().await {
        apply_progress(db, run_id, &snapshot, 3).await?;
    }
    Ok(())
}

async fn import_extracted(db: &DatabaseConnection, run_id: i64, dir: &Path) -> Result<(), String> {
    let counting = ImportPhaseTimer::new(run_id, "count");
    set_status(db, run_id, 2, "Counting release records").await?;
    let ampp_file = glob_one(dir, "f_ampp2_3*.xml")?;
    let gtin_dir = dir.to_path_buf();
    let gtin_file = blocking_with_activity(db, run_id, move |activity| {
        find_gtin_file(&gtin_dir, || {
            activity
                .send(())
                .map_err(|_| "Import activity could not be reported".to_owned())
        })
    })
    .await?;
    let ampp_count = ampp_file.clone();
    let gtin_count = gtin_file.clone();
    let (ampp_total, gtin_total) = blocking_with_activity(db, run_id, move |activity| {
        let mut signal = || {
            activity
                .send(())
                .map_err(|_| "Import activity could not be reported".to_owned())
        };
        let ampp = count_elements(&ampp_count, "AMPP", &mut signal)?;
        let gtin = count_elements(&gtin_count, "GTINDATA", &mut signal)?;
        Ok::<_, String>((ampp, gtin))
    })
    .await?;
    let counts = Counts {
        ampp_total,
        gtin_total,
        ..Default::default()
    };
    apply_progress(db, run_id, &counts, 2).await?;
    counting.complete();
    let parsing_ampp = ImportPhaseTimer::new(run_id, "parse_ampp");
    let (progress_tx, progress_rx) = mpsc::unbounded_channel::<Counts>();
    let ampp_parse = ampp_file.clone();
    let names_task = tokio::task::spawn_blocking(move || {
        let mut counts = Counts {
            ampp_total,
            gtin_total,
            ..Default::default()
        };
        let names = parse_ampp_names(&ampp_parse, &mut counts, &progress_tx)?;
        emit_progress(&progress_tx, &counts, true);
        Ok::<_, String>((names, counts))
    });
    let drain = drain_progress(db, run_id, progress_rx);
    let (names, counts) = finish_work(run_id, "parse_ampp", names_task, drain).await?;
    let names = Arc::new(names);
    apply_progress(db, run_id, &counts, 3).await?;
    parsing_ampp.complete();
    let parsing_supplementary = ImportPhaseTimer::new(run_id, "parse_supplementary");
    let supplementary = if let Some(files) = supplementary_files(dir) {
        tokio::task::spawn_blocking(move || parse_optional_supplementary(&files))
            .await
            .map_err(|error| error.to_string())?
    } else {
        None
    };
    parsing_supplementary.complete();
    let parsing_gtin = ImportPhaseTimer::new(run_id, "parse_gtin");
    let (gtin_tx, gtin_rx) = mpsc::unbounded_channel::<Counts>();
    let gtin_parse = gtin_file.clone();
    let names_for_parse = Arc::clone(&names);
    let gtin_task = tokio::task::spawn_blocking(move || {
        let today = Utc::now().date_naive();
        let mut counts = counts;
        let records = parse_gtins(&gtin_parse, &names_for_parse, today, &mut counts, &gtin_tx)?;
        emit_progress(&gtin_tx, &counts, true);
        Ok::<_, String>((records, counts))
    });
    let drain = drain_progress(db, run_id, gtin_rx);
    let (records, mut counts) = finish_work(run_id, "parse_gtin", gtin_task, drain).await?;
    parsing_gtin.complete();
    let persisting = ImportPhaseTimer::new(run_id, "persist");
    let transaction = db.begin().await.map_err(|error| error.to_string())?;
    touch_run(&transaction, run_id).await?;
    persist_relationships(&transaction, &names).await?;
    if let Some(data) = supplementary {
        persist_supplementary(&transaction, &data).await?;
    }
    persist_gtins(&transaction, &records, &mut counts).await?;
    finalize(&transaction, run_id, &counts).await?;
    persisting.complete();
    let committing = ImportPhaseTimer::new(run_id, "commit");
    transaction
        .commit()
        .await
        .map_err(|error| error.to_string())?;
    committing.complete();
    Ok(())
}

fn find_gtin_file(
    dir: &Path,
    progress: impl FnMut() -> Result<(), String>,
) -> Result<PathBuf, String> {
    if let Some(first) = glob_paths(dir, "f_gtin2_0*.xml").into_iter().next() {
        return Ok(first);
    }
    let Some(zip) = glob_paths(dir, "*GTIN.zip").into_iter().next() else {
        return Err(format!("No GTIN XML or ZIP found in {}", dir.display()));
    };
    let dest = dir.join(format!("nested-gtin-{}", uuid::Uuid::new_v4()));
    extract::extract_with_progress(&zip, &dest, Some("f_gtin2_0*.xml"), progress)?;
    glob_paths(&dest, "f_gtin2_0*.xml")
        .into_iter()
        .next()
        .ok_or_else(|| "No GTIN XML found in nested archive".to_owned())
}

async fn persist_relationships(
    transaction: &DatabaseTransaction,
    names: &HashMap<String, AmppName>,
) -> Result<(), String> {
    nhs_dmd_ampp_relationship::Entity::delete_many()
        .exec(transaction)
        .await
        .map_err(|error| error.to_string())?;
    let now = Utc::now().naive_utc();
    let rows: Vec<nhs_dmd_ampp_relationship::ActiveModel> = names
        .iter()
        .filter_map(|(ampp_code, entry)| {
            entry
                .amp_code
                .as_ref()
                .filter(|amp_code| !amp_code.trim().is_empty())
                .map(|amp_code| nhs_dmd_ampp_relationship::ActiveModel {
                    ampp_code: Set(ampp_code.clone()),
                    amp_code: Set(amp_code.clone()),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                })
        })
        .collect();
    for chunk in rows.chunks(INSERT_BATCH) {
        nhs_dmd_ampp_relationship::Entity::insert_many(chunk.to_vec())
            .exec(transaction)
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

async fn persist_gtins(
    transaction: &DatabaseTransaction,
    records: &[GtinParsed],
    counts: &mut Counts,
) -> Result<(), String> {
    let existing: Vec<nhs_dmd_barcode::Model> = nhs_dmd_barcode::Entity::find()
        .all(transaction)
        .await
        .map_err(|error| error.to_string())?;
    let by_gtin: HashMap<String, nhs_dmd_barcode::Model> = existing
        .into_iter()
        .map(|row| (row.gtin.clone(), row))
        .collect();
    let now = Utc::now().naive_utc();
    let mut seen = HashSet::new();
    let mut upserts = Vec::new();
    for record in records {
        match record {
            GtinParsed::SkipExpired => counts.skipped_expired += 1,
            GtinParsed::SkipMissingName => counts.skipped_missing_name += 1,
            GtinParsed::SkipInvalid => counts.skipped_invalid += 1,
            GtinParsed::Record {
                gtin,
                code,
                amp_code,
                display,
                vmp_name,
            } => {
                if !seen.insert(gtin.clone()) {
                    counts.skipped_invalid += 1;
                } else {
                    let identical = by_gtin.get(gtin).is_some_and(|row| {
                        row.code == *code
                            && row.display == *display
                            && row.amp_code == *amp_code
                            && row.vmp_name == *vmp_name
                            && row.system == "https://dmd.nhs.uk"
                            && row.concept_class.as_deref() == Some("AMPP")
                    });
                    if by_gtin.contains_key(gtin) && identical {
                        counts.unchanged += 1;
                    } else {
                        if by_gtin.contains_key(gtin) {
                            counts.updated += 1;
                        } else {
                            counts.created += 1;
                        }
                        upserts.push(nhs_dmd_barcode::ActiveModel {
                            amp_code: Set(amp_code.clone()),
                            code: Set(code.clone()),
                            concept_class: Set(Some("AMPP".to_owned())),
                            created_at: Set(now),
                            display: Set(display.clone()),
                            gtin: Set(gtin.clone()),
                            system: Set("https://dmd.nhs.uk".to_owned()),
                            updated_at: Set(now),
                            vmp_name: Set(vmp_name.clone()),
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }
    for chunk in upserts.chunks(INSERT_BATCH) {
        nhs_dmd_barcode::Entity::insert_many(chunk.to_vec())
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(nhs_dmd_barcode::Column::Gtin)
                    .update_columns([
                        nhs_dmd_barcode::Column::AmpCode,
                        nhs_dmd_barcode::Column::Code,
                        nhs_dmd_barcode::Column::ConceptClass,
                        nhs_dmd_barcode::Column::Display,
                        nhs_dmd_barcode::Column::System,
                        nhs_dmd_barcode::Column::UpdatedAt,
                        nhs_dmd_barcode::Column::VmpName,
                    ])
                    .to_owned(),
            )
            .exec_without_returning(transaction)
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

async fn persist_supplementary(
    transaction: &DatabaseTransaction,
    data: &TradeFamilyData,
) -> Result<(), String> {
    let now = Utc::now().naive_utc();
    let mut group_ids = HashMap::new();
    for (code, name) in &data.groups {
        let existing = nhs_dmd_trade_family_group::Entity::find()
            .filter(nhs_dmd_trade_family_group::Column::Code.eq(code))
            .one(transaction)
            .await
            .map_err(|error| error.to_string())?;
        let row = match existing {
            Some(existing) if existing.name == *name => existing,
            Some(existing) => {
                let mut active: nhs_dmd_trade_family_group::ActiveModel = existing.into();
                active.name = Set(name.clone());
                active.updated_at = Set(now);
                active
                    .update(transaction)
                    .await
                    .map_err(|error| error.to_string())?
            }
            None => nhs_dmd_trade_family_group::ActiveModel {
                code: Set(code.clone()),
                name: Set(name.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(transaction)
            .await
            .map_err(|error| error.to_string())?,
        };
        group_ids.insert(code.clone(), row.id);
    }
    let mut family_ids = HashMap::new();
    for (code, name, group_code) in &data.families {
        let group_id = group_code
            .as_ref()
            .and_then(|code| group_ids.get(code))
            .copied();
        let existing = nhs_dmd_trade_family::Entity::find()
            .filter(nhs_dmd_trade_family::Column::Code.eq(code))
            .one(transaction)
            .await
            .map_err(|error| error.to_string())?;
        let row = match existing {
            Some(existing)
                if existing.name == *name && existing.trade_family_group_id == group_id =>
            {
                existing
            }
            Some(existing) => {
                let mut active: nhs_dmd_trade_family::ActiveModel = existing.into();
                active.name = Set(name.clone());
                active.trade_family_group_id = Set(group_id);
                active.updated_at = Set(now);
                active
                    .update(transaction)
                    .await
                    .map_err(|error| error.to_string())?
            }
            None => nhs_dmd_trade_family::ActiveModel {
                code: Set(code.clone()),
                name: Set(name.clone()),
                trade_family_group_id: Set(group_id),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(transaction)
            .await
            .map_err(|error| error.to_string())?,
        };
        family_ids.insert(code.clone(), row.id);
    }
    nhs_dmd_amp_trade_family::Entity::delete_many()
        .exec(transaction)
        .await
        .map_err(|error| error.to_string())?;
    for (amp_code, family_code) in &data.memberships {
        let Some(family_id) = family_ids.get(family_code) else {
            continue;
        };
        nhs_dmd_amp_trade_family::ActiveModel {
            amp_code: Set(amp_code.clone()),
            trade_family_id: Set(*family_id),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(transaction)
        .await
        .map_err(|error| error.to_string())?;
    }
    if let Some(released_on) = data.released_on {
        let exists = nhs_dmd_supplementary_release::Entity::find()
            .filter(nhs_dmd_supplementary_release::Column::ReleasedOn.eq(released_on))
            .one(transaction)
            .await
            .map_err(|error| error.to_string())?;
        if exists.is_none() {
            nhs_dmd_supplementary_release::ActiveModel {
                released_on: Set(released_on),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(transaction)
            .await
            .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

async fn finalize(db: &DatabaseTransaction, run_id: i64, counts: &Counts) -> Result<(), String> {
    let changed = db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE nhs_dmd_imports SET status = 4, processed_records = $1, imported_count = $2, skipped_count = $3, created_count = $4, updated_count = $5, unchanged_count = $6, skipped_expired_count = $7, skipped_missing_name_count = $8, skipped_invalid_count = $9, completed_at = now(), error_message = NULL, updated_at = now() WHERE id = $10 AND status < 4",
        [
            counts.total().into(),
            counts.imported().into(),
            counts.skipped().into(),
            counts.created.into(),
            counts.updated.into(),
            counts.unchanged.into(),
            counts.skipped_expired.into(),
            counts.skipped_missing_name.into(),
            counts.skipped_invalid.into(),
            run_id.into(),
        ],
    ))
    .await
    .map_err(|error| error.to_string())?;
    if changed.rows_affected() == 1 {
        Ok(())
    } else {
        Err("Import is no longer active".into())
    }
}

pub(crate) async fn fail_run(
    db: &DatabaseConnection,
    run_id: i64,
    message: &str,
) -> Result<(), sea_orm::DbErr> {
    db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE nhs_dmd_imports SET status = 5, completed_at = now(), error_message = $1, log = COALESCE(log, '') || $1 || E'\\n', updated_at = now() WHERE id = $2 AND status < 4",
            [message.into(), run_id.into()],
        ))
        .await?;
    Ok(())
}

pub(crate) async fn fail_owned_run(
    db: &DatabaseConnection,
    owned_run: &nhs_dmd_import::Model,
    message: &str,
) -> Result<(), sea_orm::DbErr> {
    const STALE_MESSAGE: &str = "The import stopped making progress. Upload the release again.";
    if owned_run.status >= 4 {
        return Ok(());
    }
    fail_run(db, owned_run.id, message).await?;
    if matches!(
        message,
        "Import is no longer active" | "Import activity could not be reported"
    ) {
        return Ok(());
    }
    let transaction = db.begin().await?;
    let stale = nhs_dmd_import::Entity::find_by_id(owned_run.id)
        .filter(nhs_dmd_import::Column::Status.eq(5))
        .filter(nhs_dmd_import::Column::ErrorMessage.eq(STALE_MESSAGE))
        .filter(match &owned_run.archive_key {
            Some(key) => nhs_dmd_import::Column::ArchiveKey.eq(key.as_str()),
            None => nhs_dmd_import::Column::ArchiveKey.is_null(),
        })
        .filter(match &owned_run.archive_checksum {
            Some(checksum) => nhs_dmd_import::Column::ArchiveChecksum.eq(checksum.as_str()),
            None => nhs_dmd_import::Column::ArchiveChecksum.is_null(),
        })
        .lock_exclusive()
        .one(&transaction)
        .await?;
    if let Some(stale) = stale {
        let log = format!(
            "{}{STALE_MESSAGE}\n{message}\n",
            stale.log.as_deref().unwrap_or_default()
        );
        let mut active: nhs_dmd_import::ActiveModel = stale.into();
        active.error_message = Set(Some(message.to_owned()));
        active.log = Set(Some(log));
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(&transaction).await?;
    }
    transaction.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn blocking_work_failure_survives_activity_failure() {
        let error = blocking_with_activity(&DatabaseConnection::default(), 1, |activity| {
            let _ = activity.send(());
            Err::<(), _>("Specific release parsing failure".to_owned())
        })
        .await
        .unwrap_err();
        assert_eq!(error, "Specific release parsing failure");
    }

    #[tokio::test]
    async fn successful_work_does_not_hide_activity_failure() {
        let error = blocking_with_activity(&DatabaseConnection::default(), 1, |activity| {
            let _ = activity.send(());
            Ok(())
        })
        .await
        .unwrap_err();
        assert!(error.contains("Disconnected"));
    }

    #[tokio::test]
    async fn ampp_and_gtin_failures_survive_progress_failure() {
        for (phase, xml, expected) in [
            (
                "parse_ampp",
                format!(
                    "<AMPPS>{}<AMPP><AMPP></AMPP></AMPP></AMPPS>",
                    "<AMPP/>".repeat(250)
                ),
                "Nested AMPP record in release XML",
            ),
            (
                "parse_gtin",
                format!(
                    "<AMPPS><AMPP>{}<GTINDATA><GTINDATA></GTINDATA></GTINDATA></AMPP></AMPPS>",
                    "<GTINDATA/>".repeat(250)
                ),
                "Nested GTINDATA record in GTIN XML",
            ),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("release.xml");
            std::fs::write(&path, xml).unwrap();
            let (sender, receiver) = mpsc::unbounded_channel();
            let work = tokio::task::spawn_blocking(move || {
                let mut counts = Counts::default();
                if phase == "parse_ampp" {
                    parse_ampp_names(&path, &mut counts, &sender).map(|_| ())
                } else {
                    parse_gtins(
                        &path,
                        &HashMap::new(),
                        NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
                        &mut counts,
                        &sender,
                    )
                    .map(|_| ())
                }
            });
            let drain = async {
                let result = drain_progress(&DatabaseConnection::default(), 1, receiver).await;
                assert!(result.is_err());
                result
            };
            assert_eq!(
                finish_work(1, phase, work, drain).await.unwrap_err(),
                expected
            );
        }
    }

    #[tokio::test]
    async fn blocking_panic_is_not_replaced_by_activity_failure_or_exposed() {
        let error = blocking_with_activity(
            &DatabaseConnection::default(),
            1,
            |activity| -> Result<(), String> {
                let _ = activity.send(());
                panic!("synthetic private panic payload");
            },
        )
        .await
        .unwrap_err();
        assert_eq!(error, "Import worker panicked");
    }

    #[test]
    fn self_nested_release_records_fail_without_publishing_partial_names_or_gtins() {
        let dir = tempfile::tempdir().unwrap();
        let ampp = dir.path().join("ampp.xml");
        std::fs::write(
            &ampp,
            "<AMPPS><AMPP><APPID>777</APPID><NM>Outer</NM><AMPP><APPID>999</APPID><NM>Inner</NM></AMPP></AMPP></AMPPS>",
        )
        .unwrap();
        let (sender, _receiver) = mpsc::unbounded_channel();
        assert!(parse_ampp_names(&ampp, &mut Counts::default(), &sender).is_err());

        let gtin = dir.path().join("gtin.xml");
        std::fs::write(
            &gtin,
            "<AMPPS><AMPP><AMPPID>777</AMPPID><GTINDATA><GTIN>5016298210989</GTIN><GTINDATA><GTIN>5016298210990</GTIN></GTINDATA></GTINDATA></AMPP></AMPPS>",
        )
        .unwrap();
        let names = HashMap::from([(
            "777".into(),
            AmppName {
                name: "Outer".into(),
                amp_code: None,
            },
        )]);
        assert!(
            parse_gtins(
                &gtin,
                &names,
                NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
                &mut Counts::default(),
                &sender,
            )
            .is_err()
        );
    }

    #[test]
    fn self_closing_elements_are_counted_and_invalid_gtins_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let ampp = dir.path().join("ampp.xml");
        std::fs::write(
            &ampp,
            "<AMPPS><AMPP><APPID>777</APPID><NM>Tablets</NM></AMPP><AMPP/></AMPPS>",
        )
        .unwrap();
        assert_eq!(count_elements(&ampp, "AMPP", || Ok(())).unwrap(), 2);
        let (sender, _receiver) = mpsc::unbounded_channel();
        let mut counts = Counts::default();
        let names = parse_ampp_names(&ampp, &mut counts, &sender).unwrap();
        assert_eq!(counts.processed, 2);
        assert_eq!(names.len(), 1);

        let gtin = dir.path().join("gtin.xml");
        std::fs::write(
            &gtin,
            "<AMPPS><AMPP><AMPPID>777</AMPPID><GTINDATA/></AMPP></AMPPS>",
        )
        .unwrap();
        assert_eq!(count_elements(&gtin, "GTINDATA", || Ok(())).unwrap(), 1);
        let mut counts = Counts::default();
        let records = parse_gtins(
            &gtin,
            &names,
            NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            &mut counts,
            &sender,
        )
        .unwrap();
        assert_eq!(counts.processed, 1);
        assert!(matches!(records.as_slice(), [GtinParsed::SkipInvalid]));
    }

    #[test]
    fn malformed_supplementary_data_is_ignored_without_discarding_the_release() {
        let dir = tempfile::tempdir().unwrap();
        let groups = dir.path().join("f_trade_family_group2_0131026.xml");
        let families = dir.path().join("f_trade_family2_0131026.xml");
        let memberships = dir.path().join("f_amp_trade_family2_0131026.xml");
        std::fs::write(
            &groups,
            "<TRADE_FAMILY_GROUPS><TRADE_FAMILY_GROUP></TRADE_FAMILY_GROUPS>",
        )
        .unwrap();
        std::fs::write(&families, "<TRADE_FAMILIES><TRADE_FAMILY><TFID>1</TFID><NM>Family</NM></TRADE_FAMILY></TRADE_FAMILIES>").unwrap();
        std::fs::write(&memberships, "<AMP_TRADE_FAMILIES/>").unwrap();
        let files = SupplementaryFiles {
            groups,
            families,
            memberships,
        };
        assert!(parse_optional_supplementary(&files).is_none());
    }

    #[test]
    fn supplementary_trade_families_are_staged_and_duplicate_codes_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let groups = dir.path().join("f_trade_family_group2_0311026.xml");
        let families = dir.path().join("f_trade_family2_0311026.xml");
        let memberships = dir.path().join("f_amp_trade_family2_0311026.xml");
        std::fs::write(&groups, "<GROUPS><TRADE_FAMILY_GROUP><TFGID>G1</TFGID><NM>Trade &amp; <![CDATA[Co]]></NM></TRADE_FAMILY_GROUP></GROUPS>").unwrap();
        std::fs::write(&families, "<FAMILIES><TRADE_FAMILY><TFID>F1</TFID><NM>Family</NM><TFGID>G1</TFGID></TRADE_FAMILY></FAMILIES>").unwrap();
        std::fs::write(&memberships, "<MEMBERSHIPS><AMP_TRADE_FAMILY><APID>A1</APID><TFID>F1</TFID></AMP_TRADE_FAMILY></MEMBERSHIPS>").unwrap();
        let files = SupplementaryFiles {
            groups,
            families,
            memberships,
        };
        let data = parse_optional_supplementary(&files).unwrap();
        assert_eq!(data.groups, vec![("G1".into(), "Trade & Co".into())]);
        assert_eq!(
            data.families,
            vec![("F1".into(), "Family".into(), Some("G1".into()))]
        );
        assert_eq!(data.memberships, vec![("A1".into(), "F1".into())]);
        std::fs::write(&files.groups, "<GROUPS><TRADE_FAMILY_GROUP><TFGID>G1</TFGID><NM>Group</NM></TRADE_FAMILY_GROUP><TRADE_FAMILY_GROUP><TFGID>G1</TFGID><NM>Duplicate</NM></TRADE_FAMILY_GROUP></GROUPS>").unwrap();
        assert!(parse_optional_supplementary(&files).is_none());
    }

    #[test]
    fn whitespace_only_ampp_identity_and_name_are_not_catalogue_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ampp.xml");
        std::fs::write(&path, "<AMPPS><AMPP><APPID>   </APPID><NM>Missing code</NM></AMPP><AMPP><APPID>777</APPID><NM>   </NM></AMPP></AMPPS>").unwrap();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let mut counts = Counts::default();
        assert!(
            parse_ampp_names(&path, &mut counts, &sender)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn malformed_gtin_end_date_fails_instead_of_publishing_unknown_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gtin.xml");
        std::fs::write(&path, "<AMPPS><AMPP><AMPPID>777</AMPPID><GTINDATA><GTIN>5016298210989</GTIN><ENDDT>not-a-date</ENDDT></GTINDATA></AMPP></AMPPS>").unwrap();
        let names = HashMap::from([(
            "777".into(),
            AmppName {
                name: "Tablets".into(),
                amp_code: None,
            },
        )]);
        let (sender, _receiver) = mpsc::unbounded_channel();
        assert!(
            parse_gtins(
                &path,
                &names,
                NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
                &mut Counts::default(),
                &sender
            )
            .is_err()
        );
    }

    #[test]
    fn padded_gtin_end_date_is_compared_to_today() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gtin.xml");
        std::fs::write(&path, "<AMPPS><AMPP><AMPPID>777</AMPPID><GTINDATA><GTIN>5016298210989</GTIN><ENDDT> 2020-01-01 </ENDDT></GTINDATA></AMPP></AMPPS>").unwrap();
        let names = HashMap::from([(
            "777".into(),
            AmppName {
                name: "Tablets".into(),
                amp_code: None,
            },
        )]);
        let (sender, _receiver) = mpsc::unbounded_channel();
        let records = parse_gtins(
            &path,
            &names,
            NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            &mut Counts::default(),
            &sender,
        )
        .unwrap();
        assert!(matches!(records[0], GtinParsed::SkipExpired));
    }

    #[test]
    fn xml_entity_references_remain_in_ampp_names_and_gtins() {
        let dir = tempfile::tempdir().unwrap();
        let ampp = dir.path().join("ampp.xml");
        std::fs::write(&ampp, "<AMPPS><AMPP><APPID>777</APPID><NM>Thornton &amp; Ross <![CDATA[& Co]]></NM></AMPP></AMPPS>").unwrap();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let mut counts = Counts::default();
        let names = parse_ampp_names(&ampp, &mut counts, &sender).unwrap();
        assert_eq!(names["777"].name, "Thornton & Ross & Co");

        let gtin = dir.path().join("gtin.xml");
        std::fs::write(&gtin, "<AMPPS><AMPP><AMPPID>777</AMPPID><GTINDATA><GTIN>5016&#50;98210989</GTIN></GTINDATA></AMPP></AMPPS>").unwrap();
        let records = parse_gtins(
            &gtin,
            &names,
            NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            &mut counts,
            &sender,
        )
        .unwrap();
        assert!(
            matches!(&records[0], GtinParsed::Record { gtin, display, .. } if gtin == "5016298210989" && display == "Thornton & Ross & Co")
        );
    }

    #[test]
    fn nested_terminal_parentheses_do_not_create_a_shortened_vmp_name() {
        for display in [
            "Day & Night capsules 1 x (2 x (6 caps + 2 caps))",
            "Capsules (manufacturer (pack))",
        ] {
            let names = HashMap::from([(
                "777".into(),
                AmppName {
                    name: display.into(),
                    amp_code: None,
                },
            )]);
            assert!(
                matches!(
                    classify_gtin(
                        "777",
                        "5016298210989",
                        "",
                        &names,
                        NaiveDate::from_ymd_opt(2026, 10, 13).unwrap()
                    )
                    .unwrap(),
                    GtinParsed::Record { vmp_name: None, .. }
                ),
                "{display}"
            );
        }
        let names = HashMap::from([(
            "777".into(),
            AmppName {
                name: "Capsules (manufacturer)  ".into(),
                amp_code: None,
            },
        )]);
        assert!(matches!(
            classify_gtin("777", "5016298210989", "", &names, NaiveDate::from_ymd_opt(2026, 10, 13).unwrap()).unwrap(),
            GtinParsed::Record { vmp_name: Some(name), .. } if name == "Capsules (manufacturer)"
        ));

        let names = HashMap::from([(
            "777".into(),
            AmppName {
                name: "Tablets (box)".into(),
                amp_code: None,
            },
        )]);
        assert!(
            matches!(classify_gtin("777", "5016298210989", "", &names, NaiveDate::from_ymd_opt(2026, 10, 13).unwrap()).unwrap(), GtinParsed::Record { vmp_name: Some(name), .. } if name == "Tablets")
        );
    }

    #[test]
    fn reports_counting_activity_before_the_xml_scan_finishes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("many.xml");
        let mut xml = String::from("<AMPPS>");
        for _ in 0..20_000 {
            xml.push_str("<AMPP><APPID>777</APPID><NM>Scanner imported tablets</NM></AMPP>");
        }
        xml.push_str("</AMPPS>");
        std::fs::write(&path, xml).unwrap();
        let mut activity = 0;
        let total = count_elements(&path, "AMPP", || {
            activity += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(total, 20_000);
        assert!(activity > 0);
    }

    #[test]
    fn unrelated_nested_fields_do_not_replace_direct_ampp_or_gtin_values() {
        let dir = tempfile::tempdir().unwrap();
        let ampp = dir.path().join("ampp.xml");
        std::fs::write(
            &ampp,
            "<AMPPS><AMPP><APPID>777</APPID><NM>Correct tablets</NM><APID>888</APID><DETAILS><APPID>999</APPID><NM>Wrong tablets</NM><APID>000</APID></DETAILS></AMPP></AMPPS>",
        )
        .unwrap();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let mut counts = Counts::default();
        let names = parse_ampp_names(&ampp, &mut counts, &sender).unwrap();
        assert_eq!(names["777"].name, "Correct tablets");
        assert_eq!(names["777"].amp_code.as_deref(), Some("888"));

        let gtin = dir.path().join("gtin.xml");
        std::fs::write(
            &gtin,
            "<AMPPS><AMPP><AMPPID>777</AMPPID><DETAILS><AMPPID>999</AMPPID></DETAILS><GTINDATA><GTIN>5016298210989</GTIN><DETAILS><GTIN>0000000000000</GTIN></DETAILS></GTINDATA></AMPP></AMPPS>",
        )
        .unwrap();
        let records = parse_gtins(
            &gtin,
            &names,
            NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            &mut counts,
            &sender,
        )
        .unwrap();
        assert!(
            matches!(&records[0], GtinParsed::Record { gtin, code, .. } if gtin == "5016298210989" && code == "777")
        );
    }

    #[test]
    fn nested_text_retains_the_outer_ampp_and_gtin_fields() {
        let dir = tempfile::tempdir().unwrap();
        let ampp = dir.path().join("ampp.xml");
        std::fs::write(
            &ampp,
            "<AMPPS><AMPP><APPID>777</APPID><NM>Scanner <span>imported</span> tablets</NM><APID>888</APID></AMPP></AMPPS>",
        )
        .unwrap();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let mut counts = Counts::default();
        let names = parse_ampp_names(&ampp, &mut counts, &sender).unwrap();
        assert_eq!(names["777"].name, "Scanner imported tablets");
        assert_eq!(names["777"].amp_code.as_deref(), Some("888"));

        let gtin = dir.path().join("gtin.xml");
        std::fs::write(
            &gtin,
            "<AMPPS><AMPP><AMPPID>777</AMPPID><GTINDATA><GTIN>5016<span>2982</span>10989</GTIN></GTINDATA></AMPP></AMPPS>",
        )
        .unwrap();
        let records = parse_gtins(
            &gtin,
            &names,
            NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            &mut counts,
            &sender,
        )
        .unwrap();
        assert!(
            matches!(&records[0], GtinParsed::Record { gtin, display, .. } if gtin == "5016298210989" && display == "Scanner imported tablets")
        );
    }
}
