use super::extract;
use crate::entities::{
    nhs_dmd_amp_trade_family, nhs_dmd_ampp_relationship, nhs_dmd_barcode,
    nhs_dmd_supplementary_release, nhs_dmd_trade_family, nhs_dmd_trade_family_group,
};
use chrono::{NaiveDate, Utc};
use quick_xml::events::Event;
use quick_xml::Reader;
use sea_orm::prelude::Date;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, Set, Statement, TransactionTrait,
};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

const PROGRESS_BATCH: i32 = 250;
const INSERT_BATCH: usize = 1000;

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

fn count_elements(path: &Path, element: &str) -> Result<i32, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    let mut buffer = Vec::new();
    let mut count = 0_i32;
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) if event.name().as_ref() == element => count += 1,
            Ok(Event::Eof) => return Ok(count),
            Ok(_) => {}
            Err(error) => return Err(format!("XML parse failed: {error}")),
        }
        buffer.clear();
    }
}

fn parse_ampp_names(
    path: &Path,
    counts: &mut Counts,
    progress: &mpsc::UnboundedSender<Counts>,
) -> Result<HashMap<String, AmppName>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut names = HashMap::new();
    let mut in_ampp = false;
    let mut current_tag: Option<String> = None;
    let mut appid = String::new();
    let mut name = String::new();
    let mut apid = String::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                let tag = event.name().as_ref().to_owned();
                if tag == "AMPP" {
                    in_ampp = true;
                    appid.clear();
                    name.clear();
                    apid.clear();
                } else if in_ampp {
                    current_tag = Some(tag);
                }
            }
            Ok(Event::Text(event)) => {
                if let Some(tag) = &current_tag {
                    let raw = event.xml10_content();
                    let text = quick_xml::escape::unescape(&raw)
                        .map_err(|error| error.to_string())?
                        .into_owned();
                    match tag.as_str() {
                        "APPID" => appid.push_str(&text),
                        "NM" => name.push_str(&text),
                        "APID" => apid.push_str(&text),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(event)) => {
                let tag = event.name().as_ref().to_owned();
                if tag == "AMPP" {
                    in_ampp = false;
                    counts.processed += 1;
                    if !appid.is_empty() && !name.is_empty() {
                        names.insert(
                            appid.clone(),
                            AmppName {
                                name: name.clone(),
                                amp_code: (!apid.is_empty()).then(|| apid.clone()),
                            },
                        );
                    }
                    emit_progress(progress, counts, false);
                } else if current_tag.as_deref() == Some(tag.as_str()) {
                    current_tag = None;
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
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut records = Vec::new();
    let mut in_ampp = false;
    let mut in_gtindata = false;
    let mut current_tag: Option<String> = None;
    let mut amppid = String::new();
    let mut gtin = String::new();
    let mut enddt = String::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                let tag = event.name().as_ref().to_owned();
                if tag == "AMPP" {
                    in_ampp = true;
                    amppid.clear();
                } else if tag == "GTINDATA" && in_ampp {
                    in_gtindata = true;
                    gtin.clear();
                    enddt.clear();
                } else if in_ampp {
                    current_tag = Some(tag);
                }
            }
            Ok(Event::Text(event)) => {
                if let Some(tag) = &current_tag {
                    let raw = event.xml10_content();
                    let text = quick_xml::escape::unescape(&raw)
                        .map_err(|error| error.to_string())?
                        .into_owned();
                    match tag.as_str() {
                        "AMPPID" if !in_gtindata => amppid.push_str(&text),
                        "GTIN" if in_gtindata => gtin.push_str(&text),
                        "ENDDT" if in_gtindata => enddt.push_str(&text),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(event)) => {
                let tag = event.name().as_ref().to_owned();
                if tag == "AMPP" {
                    in_ampp = false;
                } else if tag == "GTINDATA" && in_gtindata {
                    in_gtindata = false;
                    counts.processed += 1;
                    records.push(classify_gtin(&amppid, &gtin, &enddt, names, today));
                    emit_progress(progress, counts, false);
                } else if current_tag.as_deref() == Some(tag.as_str()) {
                    current_tag = None;
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
) -> GtinParsed {
    if gtin.trim().is_empty() {
        return GtinParsed::SkipInvalid;
    }
    if !enddt.is_empty()
        && NaiveDate::parse_from_str(enddt, "%Y-%m-%d").is_ok_and(|date| date <= today)
    {
        return GtinParsed::SkipExpired;
    }
    if amppid.is_empty() {
        return GtinParsed::SkipMissingName;
    }
    let Some(ampp) = names.get(amppid) else {
        return GtinParsed::SkipMissingName;
    };
    let display = ampp.name.clone();
    let stripped = strip_trailing_parens(&display);
    GtinParsed::Record {
        gtin: normalize_gtin(gtin),
        code: amppid.to_owned(),
        amp_code: ampp.amp_code.clone(),
        vmp_name: (stripped != display).then_some(stripped),
        display,
    }
}

fn strip_trailing_parens(display: &str) -> String {
    let trimmed = display.trim_end();
    let Some(start) = trimmed.strip_suffix(')').and_then(|_| trimmed.rfind('(')) else {
        return display.trim().to_owned();
    };
    trimmed[..start].trim().to_owned()
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

fn parse_code_names(
    path: &Path,
    element: &str,
    code_tag: &str,
) -> Result<Vec<(String, String)>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut rows = Vec::new();
    let mut in_element = false;
    let mut current_tag: Option<String> = None;
    let mut code = String::new();
    let mut name = String::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                let tag = event.name().as_ref().to_owned();
                if tag == element {
                    in_element = true;
                    code.clear();
                    name.clear();
                } else if in_element {
                    current_tag = Some(tag);
                }
            }
            Ok(Event::Text(event)) => {
                if let Some(tag) = &current_tag {
                    let raw = event.xml10_content();
                    let text = quick_xml::escape::unescape(&raw)
                        .map_err(|error| error.to_string())?
                        .into_owned();
                    if tag.as_str() == code_tag {
                        code.push_str(&text);
                    } else if tag == "NM" {
                        name.push_str(&text);
                    }
                }
            }
            Ok(Event::End(event)) => {
                let tag = event.name().as_ref().to_owned();
                if tag == element {
                    in_element = false;
                    if !code.is_empty() && !name.is_empty() {
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
    reader.config_mut().trim_text(true);
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
                let tag = event.name().as_ref().to_owned();
                if tag == "TRADE_FAMILY" {
                    in_element = true;
                    code.clear();
                    name.clear();
                    group.clear();
                } else if in_element {
                    current_tag = Some(tag);
                }
            }
            Ok(Event::Text(event)) => {
                if let Some(tag) = &current_tag {
                    let raw = event.xml10_content();
                    let text = quick_xml::escape::unescape(&raw)
                        .map_err(|error| error.to_string())?
                        .into_owned();
                    match tag.as_str() {
                        "TFID" => code.push_str(&text),
                        "NM" => name.push_str(&text),
                        "TFGID" => group.push_str(&text),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(event)) => {
                let tag = event.name().as_ref().to_owned();
                if tag == "TRADE_FAMILY" {
                    in_element = false;
                    if !code.is_empty() && !name.is_empty() {
                        rows.push((
                            code.clone(),
                            name.clone(),
                            (!group.is_empty()).then(|| group.clone()),
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
    reader.config_mut().trim_text(true);
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
                let tag = event.name().as_ref().to_owned();
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
            Ok(Event::Text(event)) => {
                if let Some(tag) = &current_tag {
                    let raw = event.xml10_content();
                    let text = quick_xml::escape::unescape(&raw)
                        .map_err(|error| error.to_string())?
                        .into_owned();
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
                let tag = event.name().as_ref().to_owned();
                if tag == "AMP_TRADE_FAMILY" {
                    in_element = false;
                    if membership_active(&startdt, &enddt, today)
                        && !amp.is_empty()
                        && !family.is_empty()
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
    let parse = |value: &str| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok();
    if !startdt.is_empty() && parse(startdt).is_none() {
        return false;
    }
    if !enddt.is_empty() && parse(enddt).is_none() {
        return false;
    }
    parse(startdt).is_none_or(|date| date <= today) && parse(enddt).is_none_or(|date| date > today)
}

pub(crate) async fn run_import(
    db: DatabaseConnection,
    run_id: i64,
    zip_path: PathBuf,
) -> Result<(), String> {
    set_status(&db, run_id, 1, "Extracting release archive").await?;
    let zip_for_task = zip_path.clone();
    let extracted = tokio::task::spawn_blocking(move || {
        let dir = std::env::temp_dir().join(format!("nhs-dmd-release-{run_id}"));
        extract::extract(&zip_for_task, &dir, None).map(|_| dir)
    })
    .await
    .map_err(|error| error.to_string())??;
    let result = import_extracted(&db, run_id, &extracted).await;
    let _ = std::fs::remove_dir_all(&extracted);
    let _ = std::fs::remove_file(&zip_path);
    result
}

async fn set_status(
    db: &DatabaseConnection,
    run_id: i64,
    status: i32,
    message: &str,
) -> Result<(), String> {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE nhs_dmd_imports SET status = $1, started_at = COALESCE(started_at, now()), log = COALESCE(log, '') || $2 || E'\\n', updated_at = now() WHERE id = $3",
        [status.into(), message.into(), run_id.into()],
    ))
    .await
    .map_err(|error| error.to_string())?;
    Ok(())
}

async fn apply_progress(
    db: &DatabaseConnection,
    run_id: i64,
    counts: &Counts,
    status: i32,
) -> Result<(), String> {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE nhs_dmd_imports SET status = $1, total_records = $2, processed_records = $3, imported_count = $4, skipped_count = $5, created_count = $6, updated_count = $7, unchanged_count = $8, skipped_expired_count = $9, skipped_missing_name_count = $10, skipped_invalid_count = $11, updated_at = now() WHERE id = $12",
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
    Ok(())
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
    let ampp_file = glob_one(dir, "f_ampp2_3*.xml")?;
    let gtin_file = find_gtin_file(dir)?;
    let ampp_count = ampp_file.clone();
    let gtin_count = gtin_file.clone();
    let (ampp_total, gtin_total) = tokio::task::spawn_blocking(move || {
        let ampp = count_elements(&ampp_count, "AMPP")?;
        let gtin = count_elements(&gtin_count, "GTINDATA")?;
        Ok::<_, String>((ampp, gtin))
    })
    .await
    .map_err(|error| error.to_string())??;
    let counts = Counts {
        ampp_total,
        gtin_total,
        ..Default::default()
    };
    apply_progress(db, run_id, &counts, 2).await?;
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
    let (names_result, drain_result) = tokio::join!(names_task, drain);
    drain_result?;
    let (names, counts) = names_result.map_err(|error| error.to_string())??;
    apply_progress(db, run_id, &counts, 3).await?;
    persist_relationships(db, &names).await?;
    if let Some(files) = supplementary_files(dir) {
        let supplementary = tokio::task::spawn_blocking(move || parse_supplementary(&files))
            .await
            .map_err(|error| error.to_string())??;
        if let Some(data) = supplementary {
            persist_supplementary(db, &data).await?;
        }
    }
    let (gtin_tx, gtin_rx) = mpsc::unbounded_channel::<Counts>();
    let gtin_parse = gtin_file.clone();
    let gtin_task = tokio::task::spawn_blocking(move || {
        let today = Utc::now().date_naive();
        let mut counts = counts;
        let records = parse_gtins(&gtin_parse, &names, today, &mut counts, &gtin_tx)?;
        emit_progress(&gtin_tx, &counts, true);
        Ok::<_, String>((records, counts))
    });
    let drain = drain_progress(db, run_id, gtin_rx);
    let (gtin_result, drain_result) = tokio::join!(gtin_task, drain);
    drain_result?;
    let (records, mut counts) = gtin_result.map_err(|error| error.to_string())??;
    persist_gtins(db, run_id, &records, &mut counts).await?;
    finalize(db, run_id, &counts).await
}

fn find_gtin_file(dir: &Path) -> Result<PathBuf, String> {
    if let Some(first) = glob_paths(dir, "f_gtin2_0*.xml").into_iter().next() {
        return Ok(first);
    }
    let Some(zip) = glob_paths(dir, "*GTIN.zip").into_iter().next() else {
        return Err(format!("No GTIN XML or ZIP found in {}", dir.display()));
    };
    let dest = std::env::temp_dir().join(format!("dmd-gtin-{}", uuid::Uuid::new_v4()));
    extract::extract(&zip, &dest, Some("f_gtin2_0*.xml"))?;
    glob_paths(&dest, "f_gtin2_0*.xml")
        .into_iter()
        .next()
        .ok_or_else(|| "No GTIN XML found in nested archive".to_owned())
}

async fn persist_relationships(
    db: &DatabaseConnection,
    names: &HashMap<String, AmppName>,
) -> Result<(), String> {
    let transaction = db.begin().await.map_err(|error| error.to_string())?;
    nhs_dmd_ampp_relationship::Entity::delete_many()
        .exec(&transaction)
        .await
        .map_err(|error| error.to_string())?;
    let now = Utc::now().naive_utc();
    let rows: Vec<nhs_dmd_ampp_relationship::ActiveModel> = names
        .iter()
        .filter_map(|(ampp_code, entry)| {
            entry
                .amp_code
                .as_ref()
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
            .exec(&transaction)
            .await
            .map_err(|error| error.to_string())?;
    }
    transaction
        .commit()
        .await
        .map_err(|error| error.to_string())
}

async fn persist_gtins(
    db: &DatabaseConnection,
    run_id: i64,
    records: &[GtinParsed],
    counts: &mut Counts,
) -> Result<(), String> {
    let existing: Vec<nhs_dmd_barcode::Model> = nhs_dmd_barcode::Entity::find()
        .all(db)
        .await
        .map_err(|error| error.to_string())?;
    let mut by_gtin: HashMap<String, nhs_dmd_barcode::Model> = existing
        .into_iter()
        .map(|row| (row.gtin.clone(), row))
        .collect();
    let now = Utc::now().naive_utc();
    let mut inserts = Vec::new();
    let mut updates: Vec<nhs_dmd_barcode::ActiveModel> = Vec::new();
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
            } => match by_gtin.remove(gtin) {
                None => {
                    counts.created += 1;
                    inserts.push(nhs_dmd_barcode::ActiveModel {
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
                Some(row) => {
                    let identical = row.code == *code
                        && row.display == *display
                        && row.amp_code == *amp_code
                        && row.vmp_name == *vmp_name
                        && row.system == "https://dmd.nhs.uk"
                        && row.concept_class.as_deref() == Some("AMPP");
                    if identical {
                        counts.unchanged += 1;
                    } else {
                        counts.updated += 1;
                        let mut active: nhs_dmd_barcode::ActiveModel = row.into();
                        active.amp_code = Set(amp_code.clone());
                        active.code = Set(code.clone());
                        active.display = Set(display.clone());
                        active.vmp_name = Set(vmp_name.clone());
                        active.system = Set("https://dmd.nhs.uk".to_owned());
                        active.concept_class = Set(Some("AMPP".to_owned()));
                        active.updated_at = Set(now);
                        updates.push(active);
                    }
                }
            },
        }
        let handled = counts.created
            + counts.updated
            + counts.unchanged
            + counts.skipped_expired
            + counts.skipped_missing_name
            + counts.skipped_invalid;
        if handled > 0 && handled % PROGRESS_BATCH == 0 {
            apply_progress(db, run_id, counts, 3).await?;
        }
    }
    for chunk in inserts.chunks(INSERT_BATCH) {
        nhs_dmd_barcode::Entity::insert_many(chunk.to_vec())
            .exec(db)
            .await
            .map_err(|error| error.to_string())?;
    }
    for active in updates {
        active.update(db).await.map_err(|error| error.to_string())?;
    }
    Ok(())
}

async fn persist_supplementary(
    db: &DatabaseConnection,
    data: &TradeFamilyData,
) -> Result<(), String> {
    let transaction = db.begin().await.map_err(|error| error.to_string())?;
    let now = Utc::now().naive_utc();
    let mut group_ids = HashMap::new();
    for (code, name) in &data.groups {
        let existing = nhs_dmd_trade_family_group::Entity::find()
            .filter(nhs_dmd_trade_family_group::Column::Code.eq(code))
            .one(&transaction)
            .await
            .map_err(|error| error.to_string())?;
        let row = match existing {
            Some(existing) if existing.name == *name => existing,
            Some(existing) => {
                let mut active: nhs_dmd_trade_family_group::ActiveModel = existing.into();
                active.name = Set(name.clone());
                active.updated_at = Set(now);
                active
                    .update(&transaction)
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
            .insert(&transaction)
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
            .one(&transaction)
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
                    .update(&transaction)
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
            .insert(&transaction)
            .await
            .map_err(|error| error.to_string())?,
        };
        family_ids.insert(code.clone(), row.id);
    }
    nhs_dmd_amp_trade_family::Entity::delete_many()
        .exec(&transaction)
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
        .insert(&transaction)
        .await
        .map_err(|error| error.to_string())?;
    }
    if let Some(released_on) = data.released_on {
        let exists = nhs_dmd_supplementary_release::Entity::find()
            .filter(nhs_dmd_supplementary_release::Column::ReleasedOn.eq(released_on))
            .one(&transaction)
            .await
            .map_err(|error| error.to_string())?;
        if exists.is_none() {
            nhs_dmd_supplementary_release::ActiveModel {
                released_on: Set(released_on),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&transaction)
            .await
            .map_err(|error| error.to_string())?;
        }
    }
    transaction
        .commit()
        .await
        .map_err(|error| error.to_string())
}

async fn finalize(db: &DatabaseConnection, run_id: i64, counts: &Counts) -> Result<(), String> {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE nhs_dmd_imports SET status = 4, processed_records = $1, imported_count = $2, skipped_count = $3, created_count = $4, updated_count = $5, unchanged_count = $6, skipped_expired_count = $7, skipped_missing_name_count = $8, skipped_invalid_count = $9, completed_at = now(), error_message = NULL, updated_at = now() WHERE id = $10",
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
    Ok(())
}

pub(crate) async fn fail_run(db: &DatabaseConnection, run_id: i64, message: &str) {
    let _ = db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE nhs_dmd_imports SET status = 5, completed_at = now(), error_message = $1, log = COALESCE(log, '') || $1 || E'\\n', updated_at = now() WHERE id = $2",
            [message.into(), run_id.into()],
        ))
        .await;
}
