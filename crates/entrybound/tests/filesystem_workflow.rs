use std::fs;
use std::fs::FileTimes;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, UNIX_EPOCH};

use entrybound::archive::{
    CollisionPolicy, ExtractionPolicy, PackOptions, bootstrap_resource_policy, inspect, list,
    pack_directory, unpack,
};
use entrybound::chunker::BALANCED_V2;
use entrybound::diagnostics::{OutcomeClass, ReasonCode};
use entrybound::eam::{ContentRef, EntryData, EntryKind, ResourceBudget};
use entrybound::ecf::{FOOTER_LEN, PREAMBLE_LEN, SECTION_HEADER_LEN, open, verify};
use entrybound::identity::{BOOTSTRAP_CHUNK_SIZE, sha256_exact};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[test]
fn filesystem_round_trip_is_deterministic_and_complete() {
    let fixture = Fixture::new("round-trip");
    let source = fixture.path.join("source");
    create_full_source(&source);

    let first = pack_directory(&source, PackOptions::default()).unwrap();
    let second = pack_directory(&source, PackOptions::default()).unwrap();
    assert_eq!(first.bytes, second.bytes);
    assert_eq!(first.identities, second.identities);

    let opened = open(&first.bytes).unwrap();
    assert_eq!(opened.report.identities, first.identities);
    assert!(verify(&first.bytes).unwrap().pci_computed);
    let listed = list(&opened.archive).unwrap();
    assert!(
        listed.iter().any(|entry| {
            entry.path == "nested/empty-dir" && entry.kind == EntryKind::Directory
        })
    );
    assert!(listed.iter().any(|entry| {
        entry.path == "large.bin"
            && entry.kind == EntryKind::File
            && entry.logical_bytes > BOOTSTRAP_CHUNK_SIZE as u64
    }));
    let view = inspect(&opened).unwrap();
    assert_eq!(view.entry_count as usize, listed.len());
    assert_eq!(view.planner_id, "balanced-v6");
    assert_eq!(
        view.chunker_id,
        "gear-norm-v1/min-131072/target-524288/max-2097152"
    );
    assert!(view.plans.iter().any(|plan| plan.codec == "store/v1"));
    assert!(view.plans.iter().any(|plan| plan.codec == "zstandard/v1"));

    let duplicate_files = opened
        .archive
        .entry_set
        .entries()
        .iter()
        .filter(|entry| matches!(entry.data(), EntryData::File { .. }))
        .count();
    assert!(opened.archive.content_store.objects.len() < duplicate_files);
    let large = opened
        .archive
        .entry_set
        .entries()
        .iter()
        .find(|entry| entry.path().to_string() == "large.bin")
        .unwrap();
    let EntryData::File {
        content: ContentRef::Internal(digest),
    } = large.data()
    else {
        panic!("large.bin was not a file")
    };
    assert!(opened.archive.content_store.objects[&digest].chunks.len() > 1);

    let destination = fixture.path.join("restored");
    let report = unpack(&first.bytes, &destination, ExtractionPolicy::default()).unwrap();
    assert_eq!(report.entries_created as usize, listed.len());
    assert_trees_equal(&source, &destination);
    assert!(destination.join("nested/empty-dir").is_dir());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_ne!(
            fs::metadata(destination.join("run.sh"))
                .unwrap()
                .permissions()
                .mode()
                & 0o111,
            0
        );
    }
    assert_eq!(
        fs::metadata(source.join("hello.txt"))
            .unwrap()
            .modified()
            .unwrap(),
        fs::metadata(destination.join("hello.txt"))
            .unwrap()
            .modified()
            .unwrap()
    );
}

#[test]
fn filesystem_creation_order_cannot_change_cdc_output() {
    let fixture = Fixture::new("enumeration-order");
    let first_source = fixture.path.join("first");
    let second_source = fixture.path.join("second");
    fs::create_dir(&first_source).unwrap();
    fs::create_dir(&second_source).unwrap();
    let names = ["zeta", "alpha", "middle", "omega", "beta"];
    for name in names {
        fs::write(first_source.join(name), format!("payload for {name}\n")).unwrap();
    }
    for name in names.into_iter().rev() {
        fs::write(second_source.join(name), format!("payload for {name}\n")).unwrap();
    }
    let fixed_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    for source in [&first_source, &second_source] {
        for name in names {
            let file = fs::OpenOptions::new()
                .write(true)
                .open(source.join(name))
                .unwrap();
            file.set_times(FileTimes::new().set_modified(fixed_time))
                .unwrap();
        }
    }

    let first = pack_directory(&first_source, PackOptions::default()).unwrap();
    let second = pack_directory(&second_source, PackOptions::default()).unwrap();
    assert_eq!(first.bytes, second.bytes);
    assert_eq!(first.identities, second.identities);
}

#[test]
fn extraction_refuses_file_and_directory_collisions() {
    let fixture = Fixture::new("collisions");
    let source = fixture.path.join("source");
    fs::create_dir(&source).unwrap();
    fs::create_dir(source.join("nested")).unwrap();
    fs::write(source.join("nested/file"), b"archive").unwrap();
    fs::write(source.join("plain"), b"archive").unwrap();
    let bytes = pack_directory(&source, PackOptions::default())
        .unwrap()
        .bytes;

    let file_collision = fixture.path.join("file-collision");
    fs::create_dir(&file_collision).unwrap();
    fs::write(file_collision.join("plain"), b"preexisting").unwrap();
    let error = unpack(&bytes, &file_collision, ExtractionPolicy::default()).unwrap_err();
    assert_eq!(error.code(), ReasonCode::ExtractionCollision);
    assert_eq!(
        fs::read(file_collision.join("plain")).unwrap(),
        b"preexisting"
    );

    let directory_collision = fixture.path.join("directory-collision");
    fs::create_dir(&directory_collision).unwrap();
    fs::write(directory_collision.join("nested"), b"preexisting").unwrap();
    let error = unpack(&bytes, &directory_collision, ExtractionPolicy::default()).unwrap_err();
    assert_eq!(error.code(), ReasonCode::ExtractionCollision);
    assert_eq!(
        fs::read(directory_collision.join("nested")).unwrap(),
        b"preexisting"
    );

    let directory_at_file = fixture.path.join("directory-at-file");
    fs::create_dir(&directory_at_file).unwrap();
    fs::create_dir(directory_at_file.join("plain")).unwrap();
    let error = unpack(&bytes, &directory_at_file, ExtractionPolicy::default()).unwrap_err();
    assert_eq!(error.code(), ReasonCode::ExtractionCollision);
    assert!(directory_at_file.join("plain").is_dir());
}

#[test]
fn corruption_and_policy_refusal_happen_before_materialization() {
    let fixture = Fixture::new("preflight");
    let source = fixture.path.join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("file"), b"content").unwrap();
    let encoded = pack_directory(&source, PackOptions::default()).unwrap();

    let mut corrupt = encoded.bytes.clone();
    let (_, chunk_payload) = locate_section(&corrupt, 7);
    corrupt[chunk_payload.end - 1] ^= 1;
    let corrupt_destination = fixture.path.join("corrupt-output");
    let error = unpack(&corrupt, &corrupt_destination, ExtractionPolicy::default()).unwrap_err();
    assert_eq!(error.code(), ReasonCode::SectionDigestMismatch);
    assert!(!corrupt_destination.exists());

    let restrictive = ResourceBudget {
        entry_count: 0,
        ..bootstrap_resource_policy()
    };
    let policy_destination = fixture.path.join("policy-output");
    let error = unpack(
        &encoded.bytes,
        &policy_destination,
        ExtractionPolicy::new(CollisionPolicy::Refuse, restrictive),
    )
    .unwrap_err();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);
    assert!(!policy_destination.exists());

    let mut underdeclared = encoded.bytes.clone();
    underdeclared[96..104].copy_from_slice(&0_u64.to_be_bytes());
    let preamble_digest = sha256_exact(&underdeclared[..PREAMBLE_LEN as usize]);
    let footer = underdeclared.len() - FOOTER_LEN as usize;
    underdeclared[footer + 64..footer + 96].copy_from_slice(preamble_digest.as_bytes());
    let error = open(&underdeclared).unwrap_err();
    assert_eq!(error.class(), OutcomeClass::Corrupt);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);
}

#[test]
fn invalid_logical_path_is_rejected_before_destination_creation() {
    let fixture = Fixture::new("invalid-path");
    let source = fixture.path.join("source");
    fs::create_dir(&source).unwrap();
    fs::create_dir(source.join("x")).unwrap();
    let mut bytes = pack_directory(&source, PackOptions::default())
        .unwrap()
        .bytes;
    mutate_first_component_to_dot(&mut bytes);
    let destination = fixture.path.join("destination");
    let error = unpack(&bytes, &destination, ExtractionPolicy::default()).unwrap_err();
    assert_eq!(error.code(), ReasonCode::DotComponent);
    assert!(!destination.exists());
}

#[test]
fn partial_failure_never_overwrites_preexisting_files() {
    let fixture = Fixture::new("partial");
    let source = fixture.path.join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("a"), b"new a").unwrap();
    fs::write(source.join("zz"), b"new zz").unwrap();
    let bytes = pack_directory(&source, PackOptions::default())
        .unwrap()
        .bytes;
    let destination = fixture.path.join("destination");
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join("zz"), b"keep me").unwrap();

    let error = unpack(&bytes, &destination, ExtractionPolicy::default()).unwrap_err();
    assert_eq!(error.code(), ReasonCode::ExtractionCollision);
    assert_eq!(fs::read(destination.join("zz")).unwrap(), b"keep me");
    assert_eq!(fs::read(destination.join("a")).unwrap(), b"new a");
}

#[cfg(unix)]
#[test]
fn pack_rejects_symlinks_without_following_them() {
    let fixture = Fixture::new("symlink");
    let source = fixture.path.join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("target"), b"target").unwrap();
    std::os::unix::fs::symlink("target", source.join("link")).unwrap();
    let error = pack_directory(&source, PackOptions::default()).unwrap_err();
    assert_eq!(error.class(), OutcomeClass::Unsupported);
    assert_eq!(error.code(), ReasonCode::UnsupportedEntryKind);
}

fn create_full_source(source: &Path) {
    fs::create_dir(source).unwrap();
    fs::create_dir(source.join("nested")).unwrap();
    fs::create_dir(source.join("nested/deeper")).unwrap();
    fs::create_dir(source.join("nested/empty-dir")).unwrap();
    fs::write(source.join("empty"), []).unwrap();
    fs::write(source.join("hello.txt"), b"hello, Entrybound\n").unwrap();
    fs::write(source.join("binary.bin"), [0, 255, 19, 0, 128, 64]).unwrap();
    fs::write(source.join("run.sh"), b"#!/bin/sh\nexit 0\n").unwrap();
    fs::write(source.join("nested/duplicate.txt"), b"hello, Entrybound\n").unwrap();
    fs::write(
        source.join("large.bin"),
        vec![0x5a; BALANCED_V2.maximum_size + 17],
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mut permissions = fs::metadata(source.join("run.sh")).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(source.join("run.sh"), permissions).unwrap();
    }
}

fn assert_trees_equal(source: &Path, destination: &Path) {
    let source_entries = tree_entries(source);
    let destination_entries = tree_entries(destination);
    assert_eq!(source_entries, destination_entries);
    for (path, is_dir) in source_entries {
        if !is_dir {
            assert_eq!(
                fs::read(source.join(&path)).unwrap(),
                fs::read(destination.join(&path)).unwrap(),
                "file bytes differ for {}",
                path.display()
            );
        }
    }
}

fn tree_entries(root: &Path) -> Vec<(PathBuf, bool)> {
    fn visit(root: &Path, relative: &Path, output: &mut Vec<(PathBuf, bool)>) {
        let mut entries = fs::read_dir(root.join(relative))
            .unwrap()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let child = relative.join(entry.file_name());
            let is_dir = entry.file_type().unwrap().is_dir();
            output.push((child.clone(), is_dir));
            if is_dir {
                visit(root, &child, output);
            }
        }
    }
    let mut output = Vec::new();
    visit(root, Path::new(""), &mut output);
    output
}

fn mutate_first_component_to_dot(bytes: &mut [u8]) {
    let (_, manifest) = locate_section(bytes, 6);
    let path_sequence = field_value(bytes, manifest.start, 1);
    let component_start = path_sequence.start + 16;
    let component_value = field_value(bytes, component_start, 2);
    assert_eq!(component_value.len(), 1);
    bytes[component_value.start] = b'.';
    rehash_section(bytes, 6);
}

fn locate_section(bytes: &[u8], wanted: u16) -> (Range<usize>, Range<usize>) {
    let mut cursor = PREAMBLE_LEN as usize;
    let footer = bytes.len() - FOOTER_LEN as usize;
    while cursor < footer {
        let kind = u16::from_be_bytes(bytes[cursor + 4..cursor + 6].try_into().unwrap());
        let payload_len = usize::try_from(u64::from_be_bytes(
            bytes[cursor + 16..cursor + 24].try_into().unwrap(),
        ))
        .unwrap();
        let payload_start = cursor + SECTION_HEADER_LEN as usize;
        let payload_end = payload_start + payload_len;
        if kind == wanted {
            return (cursor..payload_start, payload_start..payload_end);
        }
        cursor = payload_end;
    }
    panic!("section not found")
}

fn field_value(bytes: &[u8], record_start: usize, wanted: u16) -> Range<usize> {
    let payload_len = usize::try_from(u64::from_be_bytes(
        bytes[record_start + 8..record_start + 16]
            .try_into()
            .unwrap(),
    ))
    .unwrap();
    let mut cursor = record_start + 16;
    let end = cursor + payload_len;
    while cursor < end {
        let tag = u16::from_be_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
        let len = usize::try_from(u64::from_be_bytes(
            bytes[cursor + 4..cursor + 12].try_into().unwrap(),
        ))
        .unwrap();
        if tag == wanted {
            return cursor + 12..cursor + 12 + len;
        }
        cursor += 12 + len;
    }
    panic!("field not found")
}

fn rehash_section(bytes: &mut [u8], section: u16) {
    let (header, payload) = locate_section(bytes, section);
    let digest = sha256_exact(&bytes[payload]);
    bytes[header.start + 24..header.start + 56].copy_from_slice(digest.as_bytes());
}

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "entrybound-{label}-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self { path }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
