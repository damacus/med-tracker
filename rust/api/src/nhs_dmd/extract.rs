use std::fs::File;
use std::io;
use std::path::{Component, Path, PathBuf};

pub(crate) const MAX_ENTRIES: usize = 200;
pub(crate) const MAX_ENTRY_BYTES: u64 = 150 * 1024 * 1024;
pub(crate) const MAX_TOTAL_BYTES: u64 = 500 * 1024 * 1024;
const SYMLINK_MODE: u32 = 0o120000;
const MODE_MASK: u32 = 0o170000;

pub(crate) fn extract(
    zip_path: &Path,
    destination: &Path,
    pattern: Option<&str>,
) -> Result<Vec<PathBuf>, String> {
    let file = File::open(zip_path).map_err(|error| format!("ZIP extraction failed: {error}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| format!("ZIP extraction failed: {error}"))?;
    let mut entries = Vec::new();
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| format!("ZIP extraction failed: {error}"))?;
        if let Some(pattern) = pattern {
            if !wildcard_match(pattern, entry.name()) {
                continue;
            }
        }
        validate_entry(&entry)?;
        total += entry.size();
        if total > MAX_TOTAL_BYTES {
            return Err("ZIP extraction would exceed expanded size limit.".to_owned());
        }
        entries.push(index);
        if entries.len() > MAX_ENTRIES {
            return Err("ZIP extraction contains too many entries.".to_owned());
        }
    }
    std::fs::create_dir_all(destination)
        .map_err(|error| format!("ZIP extraction failed: {error}"))?;
    let mut extracted = Vec::with_capacity(entries.len());
    for index in entries {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("ZIP extraction failed: {error}"))?;
        let target = destination.join(entry.name());
        if entry.is_dir() {
            std::fs::create_dir_all(&target)
                .map_err(|error| format!("ZIP extraction failed: {error}"))?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("ZIP extraction failed: {error}"))?;
        }
        let mut output =
            File::create(&target).map_err(|error| format!("ZIP extraction failed: {error}"))?;
        io::copy(&mut entry, &mut output)
            .map_err(|error| format!("ZIP extraction failed: {error}"))?;
        extracted.push(target);
    }
    Ok(extracted)
}

fn validate_entry(entry: &zip::read::ZipFile<File>) -> Result<(), String> {
    let name = entry.name();
    if unsafe_name(name)
        || entry
            .unix_mode()
            .is_some_and(|mode| mode & MODE_MASK == SYMLINK_MODE)
    {
        return Err(format!("unsafe ZIP entry: {name}"));
    }
    if entry.size() > MAX_ENTRY_BYTES {
        return Err(format!("ZIP entry is too large: {name}"));
    }
    Ok(())
}

fn unsafe_name(name: &str) -> bool {
    let path = Path::new(name);
    path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
}

pub(crate) fn wildcard_match(pattern: &str, name: &str) -> bool {
    let pattern: Vec<&str> = pattern.split('/').collect();
    let parts: Vec<&str> = name.split('/').collect();
    pattern.len() == parts.len()
        && pattern
            .iter()
            .zip(parts.iter())
            .all(|(pattern, part)| segment_match(pattern.as_bytes(), part.as_bytes()))
}

fn segment_match(pattern: &[u8], name: &[u8]) -> bool {
    match pattern.split_first() {
        None => name.is_empty(),
        Some((b'*', rest)) => (0..=name.len()).any(|skip| segment_match(rest, &name[skip..])),
        Some((&expected, rest)) => {
            name.first() == Some(&expected) && segment_match(rest, &name[1..])
        }
    }
}
