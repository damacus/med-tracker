use super::import::{ImportFailure, ImportFailureCategory, ImportStage};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

struct SourceReader<R> {
    inner: R,
    failed: Arc<AtomicBool>,
}

impl<R: Read> Read for SourceReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buffer).inspect_err(|error| {
            if error.kind() != std::io::ErrorKind::Interrupted {
                self.failed.store(true, Ordering::Relaxed);
            }
        })
    }
}

impl<R: Seek> Seek for SourceReader<R> {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(position).inspect_err(|error| {
            if error.kind() != std::io::ErrorKind::Interrupted {
                self.failed.store(true, Ordering::Relaxed);
            }
        })
    }
}

fn archive_error(_: impl std::fmt::Display) -> ImportFailure {
    ImportFailure::new(ImportStage::Extract, ImportFailureCategory::ArchiveInvalid)
}

fn storage_error(_: impl std::fmt::Display) -> ImportFailure {
    ImportFailure::new(ImportStage::Extract, ImportFailureCategory::Storage)
}

fn read_error(error: std::io::Error, source_failed: &AtomicBool) -> ImportFailure {
    if source_failed.swap(false, Ordering::Relaxed) {
        return storage_error(error);
    }
    match error.kind() {
        std::io::ErrorKind::InvalidData
        | std::io::ErrorKind::InvalidInput
        | std::io::ErrorKind::UnexpectedEof => archive_error(error),
        _ => storage_error(error),
    }
}

fn zip_error(error: zip::result::ZipError, source_failed: &AtomicBool) -> ImportFailure {
    match error {
        zip::result::ZipError::Io(error) => read_error(error, source_failed),
        other if source_failed.swap(false, Ordering::Relaxed) => storage_error(other),
        other => archive_error(other),
    }
}

pub(crate) const MAX_ENTRIES: usize = 200;
pub(crate) const MAX_ENTRY_BYTES: u64 = 150 * 1024 * 1024;
pub(crate) const MAX_TOTAL_BYTES: u64 = 500 * 1024 * 1024;
const SYMLINK_MODE: u32 = 0o120000;
const MODE_MASK: u32 = 0o170000;

#[cfg(test)]
pub(crate) fn extract(
    zip_path: &Path,
    destination: &Path,
    pattern: Option<&str>,
) -> Result<Vec<PathBuf>, ImportFailure> {
    extract_with_progress(zip_path, destination, pattern, || Ok(()))
}

pub(crate) fn extract_with_progress(
    zip_path: &Path,
    destination: &Path,
    pattern: Option<&str>,
    mut progress: impl FnMut() -> Result<(), ImportFailure>,
) -> Result<Vec<PathBuf>, ImportFailure> {
    let file = File::open(zip_path).map_err(storage_error)?;
    let source_failed = Arc::new(AtomicBool::new(false));
    let reader = SourceReader {
        inner: file,
        failed: source_failed.clone(),
    };
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|error| zip_error(error, &source_failed))?;
    source_failed.store(false, Ordering::Relaxed);
    if archive.len() > MAX_ENTRIES {
        return Err(archive_error("too many entries"));
    }
    let mut entries = Vec::new();
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| zip_error(error, &source_failed))?;
        source_failed.store(false, Ordering::Relaxed);
        if let Some(pattern) = pattern
            && !wildcard_match(pattern, &entry.name().map_err(archive_error)?)
        {
            continue;
        }
        validate_entry(&entry)?;
        total += entry.size();
        if total > MAX_TOTAL_BYTES {
            return Err(archive_error("expanded size limit"));
        }
        entries.push(index);
    }
    std::fs::create_dir_all(destination).map_err(storage_error)?;
    let mut extracted = Vec::with_capacity(entries.len());
    let mut emitted = 0_u64;
    for index in entries {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| zip_error(error, &source_failed))?;
        source_failed.store(false, Ordering::Relaxed);
        let target = destination.join(entry.name().map_err(archive_error)?.as_ref());
        if entry.is_dir() {
            std::fs::create_dir_all(&target).map_err(storage_error)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(storage_error)?;
        }
        let mut output = File::create(&target).map_err(storage_error)?;
        let limit = MAX_ENTRY_BYTES.min(MAX_TOTAL_BYTES - emitted);
        let mut bounded = entry.by_ref().take(limit);
        let mut block = vec![0_u8; 1024 * 1024];
        loop {
            let read = bounded
                .read(&mut block)
                .map_err(|error| read_error(error, &source_failed))?;
            source_failed.store(false, Ordering::Relaxed);
            if read == 0 {
                break;
            }
            output.write_all(&block[..read]).map_err(storage_error)?;
            emitted += read as u64;
            progress()?;
        }
        let mut overflow = [0_u8; 1];
        if entry
            .read(&mut overflow)
            .map_err(|error| read_error(error, &source_failed))?
            != 0
        {
            return Err(archive_error("expanded size limit"));
        }
        extracted.push(target);
    }
    Ok(extracted)
}

fn validate_entry(entry: &zip::read::ZipFile<SourceReader<File>>) -> Result<(), ImportFailure> {
    let name = entry.name().map_err(archive_error)?;
    if unsafe_name(&name)
        || entry
            .unix_mode()
            .is_some_and(|mode| mode & MODE_MASK == SYMLINK_MODE)
    {
        return Err(archive_error("unsafe entry"));
    }
    if entry.size() > MAX_ENTRY_BYTES {
        return Err(archive_error("entry size limit"));
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
    use proptest::prelude::*;
    use std::io::Write;

    #[test]
    fn interrupted_source_read_then_invalid_archive_is_not_storage() {
        struct InterruptedOnce {
            data: std::io::Cursor<Vec<u8>>,
            interrupted: bool,
        }
        impl Read for InterruptedOnce {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                if !self.interrupted {
                    self.interrupted = true;
                    return Err(std::io::ErrorKind::Interrupted.into());
                }
                self.data.read(buffer)
            }
        }
        impl Seek for InterruptedOnce {
            fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
                self.data.seek(position)
            }
        }
        let failed = Arc::new(AtomicBool::new(false));
        let reader = SourceReader {
            inner: InterruptedOnce {
                data: std::io::Cursor::new(vec![0; 128]),
                interrupted: false,
            },
            failed: failed.clone(),
        };
        let error = zip::ZipArchive::new(reader).err().expect("invalid archive");
        assert_eq!(zip_error(error, &failed).category.name(), "archive_invalid");
    }

    #[test]
    fn source_read_failure_is_storage_but_truncated_archive_is_invalid() {
        struct FaultyReader;
        impl std::io::Read for FaultyReader {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("injected read failure"))
            }
        }
        impl std::io::Seek for FaultyReader {
            fn seek(&mut self, _: std::io::SeekFrom) -> std::io::Result<u64> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "injected seek failure",
                ))
            }
        }
        let failed = Arc::new(AtomicBool::new(false));
        let error = zip::ZipArchive::new(SourceReader {
            inner: FaultyReader,
            failed: failed.clone(),
        })
        .err()
        .expect("source read failure");
        assert!(matches!(error, zip::result::ZipError::Io(_)));
        assert_eq!(zip_error(error, &failed).category.name(), "storage");
        failed.store(true, Ordering::Relaxed);
        assert_eq!(
            read_error(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "injected entry read failure"
                ),
                &failed
            )
            .category
            .name(),
            "storage"
        );
        assert_eq!(
            read_error(
                std::io::Error::new(std::io::ErrorKind::InvalidData, "checksum"),
                &failed
            )
            .category
            .name(),
            "archive_invalid"
        );

        let directory = tempfile::tempdir().unwrap();
        for (name, contents) in [
            ("malformed.zip", &b"not a zip"[..]),
            ("truncated.zip", &b"PK\x03\x04\x14\x00"[..]),
        ] {
            let path = directory.path().join(name);
            std::fs::write(&path, contents).unwrap();
            let failure = extract(&path, &directory.path().join("out"), None).unwrap_err();
            assert_eq!(failure.category.name(), "archive_invalid", "{name}");
        }
    }

    #[test]
    fn corrupt_deflate_stream_is_an_invalid_archive() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("corrupt.zip");
        let mut writer = zip::ZipWriter::new(File::create(&path).unwrap());
        writer
            .start_file(
                "release.xml",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(b"release contents").unwrap();
        writer.finish().unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        let name_length = usize::from(u16::from_le_bytes([bytes[26], bytes[27]]));
        let extra_length = usize::from(u16::from_le_bytes([bytes[28], bytes[29]]));
        let data_start = 30 + name_length + extra_length;
        bytes[data_start] = 0xff;
        std::fs::write(&path, bytes).unwrap();
        let failure = extract(&path, &directory.path().join("out"), None).unwrap_err();
        assert_eq!(failure.category.name(), "archive_invalid");
    }

    #[test]
    fn reports_extraction_activity_while_a_large_entry_is_written() {
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
        let block = vec![0_u8; 1024 * 1024];
        for _ in 0..4 {
            writer.write_all(&block).unwrap();
        }
        writer.finish().unwrap();
        let mut activity = 0;
        extract_with_progress(
            &archive_path,
            &directory.path().join("output"),
            None,
            || {
                activity += 1;
                Ok(())
            },
        )
        .unwrap();
        assert!(activity >= 4);
    }

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

    proptest::proptest! {
        #[test]
        fn wildcard_match_never_panics(
            pattern in "[a-z0-9*/._-]{0,32}",
            name in "[a-z0-9*/._-]{0,32}",
        ) {
            let _ = wildcard_match(&pattern, &name);
        }

        #[test]
        fn wildcard_match_without_star_is_exact_equality(
            pattern in "[a-z0-9._/-]{0,32}",
            name in "[a-z0-9._/-]{0,32}",
        ) {
            prop_assert_eq!(wildcard_match(&pattern, &name), pattern == name);
        }

        #[test]
        fn wildcard_star_segment_matches_any_single_segment(
            prefix in "[a-z0-9_-]{1,8}",
            segment in "[a-z0-9_-]{0,16}",
        ) {
            let pattern = format!("{prefix}/*");
            let name = format!("{prefix}/{segment}");
            prop_assert!(wildcard_match(&pattern, &name));
        }

        #[test]
        fn unsafe_name_flags_parent_and_absolute_paths(
            segment in "[a-z0-9_-]{1,8}",
        ) {
            let parent = format!("{segment}/../x");
            let absolute = format!("/{segment}");
            prop_assert!(unsafe_name(&parent));
            prop_assert!(unsafe_name(&absolute));
            prop_assert!(!unsafe_name(&segment));
        }
    }
}
