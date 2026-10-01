//! Cross-reader extraction policy and verification boundary regressions.
//!
//! These fixtures use the real INDEXED, STREAM, encrypted INDEXED, and range
//! readers. Range reads return verified bytes but never materialize a tree.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use entrybound::archive::{
    CollisionPolicy, ExtractionPolicy, PackOptions, bootstrap_resource_policy, plan_directory,
    unpack, unpack_opened, unpack_stream,
};
use entrybound::crypto::{
    EncryptedOpenOptions, EncryptedWriteOptions, Unlock, XWingIdentity, encrypt_archive,
    open_encrypted, open_indexed_random_encrypted,
};
use entrybound::diagnostics::{OutcomeClass, ReasonCode};
use entrybound::eam::{DecodeRequirements, LogicalPath, ResourceBudget};
use entrybound::ecf::{
    StreamWriteOptions, WriteOptions, bootstrap_sequential_limits, encode, encode_stream, open,
    open_indexed_random,
};
use entrybound::random_access::{MemoryRandomReadSource, RandomAccessPolicy};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    source: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        #[cfg(windows)]
        let base = repo.join("target/extraction-policy-consistency");
        #[cfg(not(windows))]
        let base = std::env::temp_dir().join("entrybound-extraction-policy-consistency");
        #[cfg(not(windows))]
        let _ = repo;
        fs::create_dir_all(&base).unwrap();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = base.join(format!(
            "{name}-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let source = root.join("source");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("file.bin"), vec![b'x'; 128 * 1024]).unwrap();
        Self { root, source }
    }

    fn output(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn refusing_budget() -> ResourceBudget {
    ResourceBudget {
        entry_count: 0,
        ..bootstrap_resource_policy()
    }
}

#[test]
fn stream_explicit_limits_cannot_be_widened_by_extraction_policy() {
    let fixture = Fixture::new("stream-explicit-limits");
    let archive = plan_directory(&fixture.source, PackOptions::default()).unwrap();
    let mut stream = Vec::new();
    encode_stream(&archive, StreamWriteOptions::default(), &mut stream).unwrap();
    for decode_case in [false, true] {
        let mut limits = bootstrap_sequential_limits();
        if decode_case {
            limits.decode.window_bytes = 0;
        } else {
            limits.budget.entry_count = 0;
        }
        let destination = fixture.output(if decode_case {
            "explicit-decode-refused"
        } else {
            "explicit-budget-refused"
        });
        let error = unpack_stream(
            Cursor::new(&stream),
            &destination,
            ExtractionPolicy::default(),
            limits,
        )
        .unwrap_err();
        assert_eq!(error.class(), OutcomeClass::PolicyRefused);
        assert_eq!(error.code(), ReasonCode::ResourceLimit);
        assert!(!destination.exists());
    }
}

#[test]
fn every_full_reader_refuses_caller_budget_before_final_output() {
    let fixture = Fixture::new("budget");
    let archive = plan_directory(&fixture.source, PackOptions::default()).unwrap();
    let indexed = encode(&archive, WriteOptions::default()).unwrap();
    let mut stream = Vec::new();
    encode_stream(&archive, StreamWriteOptions::default(), &mut stream).unwrap();
    let (identity, recipient) = XWingIdentity::generate().unwrap();
    let encrypted = encrypt_archive(
        &archive,
        EncryptedWriteOptions {
            recipients: std::slice::from_ref(&recipient),
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap();
    let budget = refusing_budget();
    let policy = ExtractionPolicy::new(CollisionPolicy::Refuse, budget);

    let destination = fixture.output("indexed-refused");
    let error = unpack(&indexed.bytes, &destination, policy).unwrap_err();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);
    assert!(!destination.exists());

    let destination = fixture.output("stream-refused");
    let error = unpack_stream(
        Cursor::new(&stream),
        &destination,
        policy,
        bootstrap_sequential_limits(),
    )
    .unwrap_err();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);
    assert!(!destination.exists());

    // An opened archive may have been verified under different, broader limits.
    // The extraction policy is still caller-owned at the materialization API.
    let opened = open(&indexed.bytes).unwrap();
    let destination = fixture.output("opened-refused");
    let error = unpack_opened(&opened, &destination, policy).unwrap_err();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);
    assert!(!destination.exists());

    let mut crypto_options = EncryptedOpenOptions::new(Some(Unlock::Identity(&identity)));
    crypto_options.resource_policy = budget;
    let destination = fixture.output("encrypted-refused");
    let error = open_encrypted(&encrypted.bytes, crypto_options).unwrap_err();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);
    assert!(!destination.exists());

    let range_policy = RandomAccessPolicy {
        resource_policy: budget,
        ..RandomAccessPolicy::default()
    };
    let error = open_indexed_random(
        MemoryRandomReadSource::new(indexed.bytes.clone()),
        range_policy.clone(),
    )
    .err()
    .unwrap();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);
    let error = open_indexed_random_encrypted(
        MemoryRandomReadSource::new(encrypted.bytes.clone()),
        range_policy,
        EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
    )
    .err()
    .unwrap();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);
}

#[test]
fn every_full_reader_refuses_caller_decode_limit_before_final_output() {
    let fixture = Fixture::new("decode");
    let archive = plan_directory(&fixture.source, PackOptions::default()).unwrap();
    let indexed = encode(&archive, WriteOptions::default()).unwrap();
    assert!(indexed.archive.descriptor.decode.window_bytes > 0);
    let mut stream = Vec::new();
    encode_stream(&archive, StreamWriteOptions::default(), &mut stream).unwrap();
    let (identity, recipient) = XWingIdentity::generate().unwrap();
    let encrypted = encrypt_archive(
        &archive,
        EncryptedWriteOptions {
            recipients: std::slice::from_ref(&recipient),
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap();
    let decode = DecodeRequirements::default();
    let policy = ExtractionPolicy::new_with_decode(
        CollisionPolicy::Refuse,
        bootstrap_resource_policy(),
        decode,
    );

    let destination = fixture.output("indexed-decode-refused");
    let error = unpack(&indexed.bytes, &destination, policy).unwrap_err();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);
    assert!(!destination.exists());

    let destination = fixture.output("stream-decode-refused");
    let error = unpack_stream(
        Cursor::new(&stream),
        &destination,
        policy,
        bootstrap_sequential_limits(),
    )
    .unwrap_err();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);
    assert!(!destination.exists());

    let opened = open(&indexed.bytes).unwrap();
    let destination = fixture.output("opened-decode-refused");
    let error = unpack_opened(&opened, &destination, policy).unwrap_err();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);
    assert!(!destination.exists());

    let mut options = EncryptedOpenOptions::new(Some(Unlock::Identity(&identity)));
    options.decode_policy = decode;
    let error = open_encrypted(&encrypted.bytes, options).unwrap_err();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);

    let range_policy = RandomAccessPolicy {
        decode_policy: decode,
        ..RandomAccessPolicy::default()
    };
    let error = open_indexed_random(
        MemoryRandomReadSource::new(indexed.bytes),
        range_policy.clone(),
    )
    .err()
    .unwrap();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);
    let error = open_indexed_random_encrypted(
        MemoryRandomReadSource::new(encrypted.bytes),
        range_policy,
        EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
    )
    .err()
    .unwrap();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);
}

#[test]
fn full_readers_materialize_the_same_verified_tree() {
    let fixture = Fixture::new("accepted");
    let archive = plan_directory(&fixture.source, PackOptions::default()).unwrap();
    let indexed = encode(&archive, WriteOptions::default()).unwrap();
    let mut stream = Vec::new();
    encode_stream(&archive, StreamWriteOptions::default(), &mut stream).unwrap();
    let (identity, recipient) = XWingIdentity::generate().unwrap();
    let encrypted = encrypt_archive(
        &archive,
        EncryptedWriteOptions {
            recipients: std::slice::from_ref(&recipient),
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap();

    let indexed_destination = fixture.output("indexed-accepted");
    let indexed_report = unpack(
        &indexed.bytes,
        &indexed_destination,
        ExtractionPolicy::default(),
    )
    .unwrap();
    let stream_destination = fixture.output("stream-accepted");
    let (stream_report, _) = unpack_stream(
        Cursor::new(&stream),
        &stream_destination,
        ExtractionPolicy::default(),
        bootstrap_sequential_limits(),
    )
    .unwrap();
    let encrypted_destination = fixture.output("encrypted-accepted");
    let opened = open_encrypted(
        &encrypted.bytes,
        EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
    )
    .unwrap();
    let encrypted_report =
        unpack_opened(&opened, &encrypted_destination, ExtractionPolicy::default()).unwrap();
    assert_eq!(
        indexed_report.entries_created,
        stream_report.entries_created
    );
    assert_eq!(
        indexed_report.entries_created,
        encrypted_report.entries_created
    );
    let original = fs::read(fixture.source.join("file.bin")).unwrap();
    for destination in [
        indexed_destination,
        stream_destination,
        encrypted_destination,
    ] {
        assert_eq!(fs::read(destination.join("file.bin")).unwrap(), original);
    }
}

#[test]
fn opened_archive_inspection_cannot_authorize_unverified_models() {
    let fixture = Fixture::new("opened-mutation");
    let original = fs::read(fixture.source.join("file.bin")).unwrap();
    let archive = plan_directory(&fixture.source, PackOptions::default()).unwrap();
    let indexed = encode(&archive, WriteOptions::default()).unwrap();

    let opened = open(&indexed.bytes).unwrap();
    let mut edit = opened.archive.clone().into_inner();
    edit.descriptor.budget.entry_count = 0;
    let first = edit
        .content_store
        .chunks
        .values_mut()
        .find(|chunk| !chunk.plaintext.is_empty())
        .unwrap();
    first.plaintext[0] ^= 1;
    assert!(encode(&edit, WriteOptions::default()).is_err());
    // Editing consumes an unverified copy; it cannot mutate the read-only
    // reader model or be assigned back as a VerifiedArchive.
    assert_ne!(edit.descriptor.budget, opened.archive.descriptor.budget);

    fs::write(fixture.source.join("file.bin"), b"another verified archive").unwrap();
    let replacement = plan_directory(&fixture.source, PackOptions::default()).unwrap();
    let replacement = encode(&replacement, WriteOptions::default()).unwrap();
    let mut replaced_model = opened.clone();
    replaced_model.archive = open(&replacement.bytes).unwrap().archive;
    let destination = fixture.output("replaced-model");
    assert!(unpack_opened(&replaced_model, &destination, ExtractionPolicy::default()).is_err());
    assert!(!destination.exists());

    // The public report is an inspection result. Flipping its booleans cannot
    // change the reader-owned extraction authority or the verified EAM.
    let mut report_only = opened;
    report_only.report.semantic_invariants = false;
    report_only.report.chunk_integrity = false;
    let destination = fixture.output("altered-report-only");
    unpack_opened(&report_only, &destination, ExtractionPolicy::default()).unwrap();
    assert_eq!(fs::read(destination.join("file.bin")).unwrap(), original);
}

#[test]
fn stream_without_producer_budget_uses_caller_budget() {
    let fixture = Fixture::new("stream-no-declaration");
    let archive = plan_directory(&fixture.source, PackOptions::default()).unwrap();
    let mut stream = Vec::new();
    encode_stream(
        &archive,
        StreamWriteOptions {
            budget_declared: false,
            ..StreamWriteOptions::default()
        },
        &mut stream,
    )
    .unwrap();

    let destination = fixture.output("caller-refused");
    let policy = ExtractionPolicy::new(CollisionPolicy::Refuse, refusing_budget());
    let error = unpack_stream(
        Cursor::new(&stream),
        &destination,
        policy,
        bootstrap_sequential_limits(),
    )
    .unwrap_err();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);
    assert!(!destination.exists());

    let destination = fixture.output("caller-accepted");
    let (_, report) = unpack_stream(
        Cursor::new(&stream),
        &destination,
        ExtractionPolicy::default(),
        bootstrap_sequential_limits(),
    )
    .unwrap();
    assert!(!report.budget_declared);
    assert_eq!(
        fs::read(destination.join("file.bin")).unwrap(),
        fs::read(fixture.source.join("file.bin")).unwrap()
    );
}

#[test]
fn corrupt_or_unauthenticated_inputs_never_create_final_objects() {
    let fixture = Fixture::new("integrity");
    let archive = plan_directory(&fixture.source, PackOptions::default()).unwrap();
    let indexed = encode(&archive, WriteOptions::default()).unwrap();
    let mut stream = Vec::new();
    encode_stream(&archive, StreamWriteOptions::default(), &mut stream).unwrap();
    let (identity, recipient) = XWingIdentity::generate().unwrap();
    let encrypted = encrypt_archive(
        &archive,
        EncryptedWriteOptions {
            recipients: std::slice::from_ref(&recipient),
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap();

    let mut invalid_indexed = indexed.bytes.clone();
    invalid_indexed[0] ^= 1; // canonical magic
    let destination = fixture.output("invalid-indexed");
    assert!(unpack(&invalid_indexed, &destination, ExtractionPolicy::default()).is_err());
    assert!(!destination.exists());

    let truncated_stream = &stream[..stream.len() - 1];
    let destination = fixture.output("truncated-stream");
    assert!(
        unpack_stream(
            Cursor::new(truncated_stream),
            &destination,
            ExtractionPolicy::default(),
            bootstrap_sequential_limits()
        )
        .is_err()
    );
    assert!(!destination.exists());

    let mut altered_encrypted = encrypted.bytes.clone();
    altered_encrypted[0] ^= 1;
    let destination = fixture.output("invalid-encrypted");
    assert!(
        open_encrypted(
            &altered_encrypted,
            EncryptedOpenOptions::new(Some(Unlock::Identity(&identity)))
        )
        .is_err()
    );
    assert!(!destination.exists());
    let (wrong_identity, _) = XWingIdentity::generate().unwrap();
    assert!(
        open_encrypted(
            &encrypted.bytes,
            EncryptedOpenOptions::new(Some(Unlock::Identity(&wrong_identity)))
        )
        .is_err()
    );
    assert!(!destination.exists());

    // Range APIs intentionally expose only the requested, verified file and
    // must label the remaining archive as not fully verified.
    let path = LogicalPath::from_utf8(["file.bin"]).unwrap();
    let mut indexed_range = open_indexed_random(
        MemoryRandomReadSource::new(indexed.bytes.clone()),
        RandomAccessPolicy::default(),
    )
    .unwrap();
    let read = indexed_range.read_entry(&path).unwrap();
    assert_eq!(&*read.bytes, vec![b'x'; 128 * 1024].as_slice());
    assert!(!read.report.whole_archive_verified);
    let mut encrypted_range = open_indexed_random_encrypted(
        MemoryRandomReadSource::new(encrypted.bytes),
        RandomAccessPolicy::default(),
        EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
    )
    .unwrap();
    let read = encrypted_range.read_entry(&path).unwrap();
    assert_eq!(&*read.bytes, vec![b'x'; 128 * 1024].as_slice());
    assert!(!read.report.whole_archive_verified);
}
