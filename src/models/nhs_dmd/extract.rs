use std::fs::File;
use std::io::{self, Read};
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
    if archive.len() > MAX_ENTRIES {
        return Err("ZIP extraction contains too many entries.".to_owned());
    }
    let mut entries = Vec::new();
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| format!("ZIP extraction failed: {error}"))?;
        if let Some(pattern) = pattern
            && !wildcard_match(
                pattern,
                &entry
                    .name()
                    .map_err(|error| format!("ZIP extraction failed: {error}"))?,
            )
        {
            continue;
        }
        validate_entry(&entry)?;
        total += entry.size();
        if total > MAX_TOTAL_BYTES {
            return Err("ZIP extraction would exceed expanded size limit.".to_owned());
        }
        entries.push(index);
    }
    std::fs::create_dir_all(destination)
        .map_err(|error| format!("ZIP extraction failed: {error}"))?;
    let mut extracted = Vec::with_capacity(entries.len());
    let mut emitted = 0_u64;
    for index in entries {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("ZIP extraction failed: {error}"))?;
        let target = destination.join(
            entry
                .name()
                .map_err(|error| format!("ZIP extraction failed: {error}"))?
                .as_ref(),
        );
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
        let limit = MAX_ENTRY_BYTES.min(MAX_TOTAL_BYTES - emitted);
        let written = io::copy(&mut entry.by_ref().take(limit), &mut output)
            .map_err(|error| format!("ZIP extraction failed: {error}"))?;
        emitted += written;
        let mut overflow = [0_u8; 1];
        if entry
            .read(&mut overflow)
            .map_err(|error| format!("ZIP extraction failed: {error}"))?
            != 0
        {
            return Err("ZIP extraction would exceed expanded size limit.".to_owned());
        }
        extracted.push(target);
    }
    Ok(extracted)
}

fn validate_entry(entry: &zip::read::ZipFile<File>) -> Result<(), String> {
    let name = entry
        .name()
        .map_err(|error| format!("ZIP extraction failed: {error}"))?;
    if unsafe_name(&name)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejects_excess_archive_entries_even_when_pattern_excludes_them() {
        let directory = tempfile::tempdir().unwrap();
        let archive_path = directory.path().join("release.zip");
        let mut writer = zip::ZipWriter::new(File::create(&archive_path).unwrap());
        for index in 0..=MAX_ENTRIES {
            writer
                .start_file(
                    format!("ignored-{index}.txt"),
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
            writer.write_all(b"ignored").unwrap();
        }
        writer.finish().unwrap();
        let result = extract(
            &archive_path,
            &directory.path().join("output"),
            Some("*.xml"),
        );
        assert!(
            result.is_err(),
            "All archive entries must count towards the extraction limit"
        );
    }

    #[test]
    fn limits_written_bytes_when_expanded_size_metadata_is_false() {
        let directory = tempfile::tempdir().unwrap();
        let archive_path = directory.path().join("release.zip");
        let mut writer = zip::ZipWriter::new(File::create(&archive_path).unwrap());
        writer
            .start_file(
                "release.xml",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
        let block = [0_u8; 1024 * 1024];
        for _ in 0..151 {
            writer.write_all(&block).unwrap();
        }
        writer.finish().unwrap();
        let mut bytes = std::fs::read(&archive_path).unwrap();
        let central = bytes
            .windows(4)
            .position(|value| value == b"PK\x01\x02")
            .unwrap();
        bytes[central + 24..central + 28].copy_from_slice(&1_u32.to_le_bytes());
        bytes[22..26].copy_from_slice(&1_u32.to_le_bytes());
        std::fs::write(&archive_path, bytes).unwrap();
        let destination = directory.path().join("output");
        let result = extract(&archive_path, &destination, None);
        let written = std::fs::metadata(destination.join("release.xml"))
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        assert!(
            written <= MAX_ENTRY_BYTES,
            "Extraction wrote {written} bytes beyond its limit"
        );
        assert!(result.is_err());
    }
}
