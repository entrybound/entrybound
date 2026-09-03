use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use entrybound::archive::{PackOptions, plan_directory};
use entrybound::crypto::{
    EncryptedOpenOptions, EncryptedWriteOptions, Unlock, XWingIdentity, encrypt_archive,
    inspect_indexed_random_encrypted_public, open_indexed_random_encrypted,
};
use entrybound::eam::LogicalPath;
use entrybound::ecf::{WriteOptions, encode, open_indexed_random};
use entrybound::planner::CompressionProfile;
use entrybound::random_access::{HttpRangeSource, RandomAccessPolicy};
use flate2::{Compression, write::GzEncoder};

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "entrybound-http-range-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct RangeServer {
    url: String,
    stop: Arc<AtomicBool>,
    requests: Arc<AtomicU64>,
    transferred: Arc<AtomicU64>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl RangeServer {
    fn start(bytes: Vec<u8>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let bytes: Arc<[u8]> = bytes.into();
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(AtomicU64::new(0));
        let transferred = Arc::new(AtomicU64::new(0));
        let thread_stop = Arc::clone(&stop);
        let thread_requests = Arc::clone(&requests);
        let thread_transferred = Arc::clone(&transferred);
        let thread = std::thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                let (mut stream, _) = match listener.accept() {
                    Ok(value) => value,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("range server accept failed: {error}"),
                };
                stream.set_nonblocking(false).unwrap();
                let mut request = Vec::new();
                let mut buffer = [0_u8; 2048];
                loop {
                    let count = stream.read(&mut buffer).unwrap();
                    if count == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..count]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                thread_requests.fetch_add(1, Ordering::Relaxed);
                let request = String::from_utf8(request).unwrap();
                if request.starts_with("HEAD ") {
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nETag: \"fixture-v1\"\r\nConnection: close\r\n\r\n",
                        bytes.len()
                    )
                    .unwrap();
                } else {
                    assert!(request.contains("if-match: \"fixture-v1\""));
                    assert!(request.contains("accept-encoding: identity"));
                    let range = request
                        .lines()
                        .find_map(|line| line.strip_prefix("range: bytes="))
                        .unwrap();
                    let (start, end) = range.split_once('-').unwrap();
                    let start = start.parse::<usize>().unwrap();
                    let end = end.parse::<usize>().unwrap();
                    let body = &bytes[start..=end];
                    thread_transferred.fetch_add(body.len() as u64, Ordering::Relaxed);
                    write!(
                        stream,
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {start}-{end}/{}\r\nETag: \"fixture-v1\"\r\nConnection: close\r\n\r\n",
                        body.len(),
                        bytes.len()
                    )
                    .unwrap();
                    stream.write_all(body).unwrap();
                }
                stream.flush().unwrap();
            }
        });
        Self {
            url: format!("http://{address}/archive.eb"),
            stop,
            requests,
            transferred,
            thread: Some(thread),
        }
    }
}

impl Drop for RangeServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

fn planned_fixture() -> (TestDir, entrybound::eam::Archive) {
    let directory = TestDir::new();
    std::fs::write(
        directory.path().join("wanted.txt"),
        b"verified HTTP range entry",
    )
    .unwrap();
    let mut state = 0x1234_5678_u32;
    let unrelated = (0..2 * 1024 * 1024)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state as u8
        })
        .collect::<Vec<_>>();
    std::fs::write(directory.path().join("unrelated.bin"), unrelated).unwrap();
    let archive = plan_directory(directory.path(), PackOptions::default()).unwrap();
    (directory, archive)
}

fn write_similar_files(directory: &Path, count: usize, len: usize) {
    let mut state = 0xbb67_ae85_84ca_a73b_u64;
    let base = (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect::<Vec<_>>();
    for index in 0..count {
        let mut bytes = base.clone();
        let first = 128 + (index * 977) % (len - 512);
        for (offset, byte) in bytes[first..first + 256].iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(31).wrapping_add(offset as u8);
        }
        std::fs::write(directory.join(format!("sample-{index:03}.bin")), bytes).unwrap();
    }
}

fn path_containing_chunk(
    archive: &entrybound::eam::Archive,
    chunk_id: entrybound::eam::Digest,
) -> LogicalPath {
    archive
        .entry_set
        .entries()
        .iter()
        .find_map(|entry| match entry.data() {
            entrybound::eam::EntryData::File {
                content: entrybound::eam::ContentRef::Internal(content),
            } if archive.content_store.objects[&content]
                .chunks
                .iter()
                .any(|reference| reference.chunk_id == chunk_id) =>
            {
                Some(entry.path().clone())
            }
            _ => None,
        })
        .unwrap()
}

#[test]
fn http_indexed_read_fetches_only_metadata_and_requested_closure() {
    let (_directory, archive) = planned_fixture();
    let encoded = encode(&archive, WriteOptions::default()).unwrap();
    let server = RangeServer::start(encoded.bytes);
    let source = HttpRangeSource::open(&server.url).unwrap();
    let mut opened = open_indexed_random(source, RandomAccessPolicy::default()).unwrap();
    let read = opened
        .read_entry(&LogicalPath::from_utf8(["wanted.txt"]).unwrap())
        .unwrap();
    assert_eq!(&*read.bytes, b"verified HTTP range entry");
    assert!(read.report.bytes_fetched < opened.metadata().source_length / 2);
    assert_eq!(
        read.report.bytes_fetched,
        server.transferred.load(Ordering::Relaxed)
    );
    assert!(server.requests.load(Ordering::Relaxed) > read.report.range_request_count);
    assert!(!read.report.whole_archive_verified);
}

#[test]
fn http_encrypted_indexed_read_authenticates_without_full_download() {
    let (_directory, archive) = planned_fixture();
    let (identity, recipient) = XWingIdentity::generate().unwrap();
    let encrypted = encrypt_archive(
        &archive,
        EncryptedWriteOptions {
            recipients: &[recipient],
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap();
    let server = RangeServer::start(encrypted.bytes);
    let public = inspect_indexed_random_encrypted_public(
        HttpRangeSource::open(&server.url).unwrap(),
        RandomAccessPolicy::default(),
        entrybound::crypto::CryptoPolicy::default(),
    )
    .unwrap();
    assert!(public.public.encrypted);
    assert!(!public.whole_archive_verified);
    assert!(public.bytes_fetched < public.public.total_container_bytes);
    let source = HttpRangeSource::open(&server.url).unwrap();
    let mut opened = open_indexed_random_encrypted(
        source,
        RandomAccessPolicy::default(),
        EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
    )
    .unwrap();
    let read = opened
        .read_entry(&LogicalPath::from_utf8(["wanted.txt"]).unwrap())
        .unwrap();
    assert_eq!(&*read.bytes, b"verified HTTP range entry");
    assert!(read.report.bytes_fetched < opened.metadata().source_length);
    assert!(!read.report.whole_archive_verified);
}

#[test]
fn http_password_indexed_read_keeps_kdf_and_decryption_client_side() {
    let (_directory, archive) = planned_fixture();
    let password = b"high entropy HTTP range test password";
    let encrypted = encrypt_archive(
        &archive,
        EncryptedWriteOptions {
            password: Some(password),
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap();
    let server = RangeServer::start(encrypted.bytes);
    let mut opened = open_indexed_random_encrypted(
        HttpRangeSource::open(&server.url).unwrap(),
        RandomAccessPolicy::default(),
        EncryptedOpenOptions::new(Some(Unlock::Password(password))),
    )
    .unwrap();
    let read = opened
        .read_entry(&LogicalPath::from_utf8(["wanted.txt"]).unwrap())
        .unwrap();
    assert_eq!(&*read.bytes, b"verified HTTP range entry");
    assert!(read.report.bytes_fetched < opened.metadata().source_length);
    assert!(!read.report.whole_archive_verified);
}

#[test]
fn random_access_fetches_dictionary_and_bounded_lookback_dependencies() {
    let dictionary_dir = TestDir::new();
    write_similar_files(dictionary_dir.path(), 24, 64 * 1024);
    let dictionary_archive = plan_directory(
        dictionary_dir.path(),
        PackOptions {
            profile: CompressionProfile::Balanced,
            ..PackOptions::default()
        },
    )
    .unwrap();
    let dictionary_plans = dictionary_archive
        .transform_plans
        .iter()
        .filter(|plan| plan.dictionary.is_some())
        .map(|plan| plan.plan_id)
        .collect::<Vec<_>>();
    let dictionary_chunk = dictionary_archive
        .content_store
        .chunks
        .values()
        .find(|chunk| dictionary_plans.contains(&chunk.plan_ref))
        .unwrap()
        .chunk_id;
    let dictionary_path = path_containing_chunk(&dictionary_archive, dictionary_chunk);
    let expected_dictionary =
        std::fs::read(dictionary_dir.path().join(dictionary_path.to_string())).unwrap();
    let dictionary_bytes = encode(&dictionary_archive, WriteOptions::default())
        .unwrap()
        .bytes;
    let mut dictionary_reader = open_indexed_random(
        entrybound::random_access::MemoryRandomReadSource::new(dictionary_bytes),
        RandomAccessPolicy::default(),
    )
    .unwrap();
    let dictionary_read = dictionary_reader.read_entry(&dictionary_path).unwrap();
    assert_eq!(&*dictionary_read.bytes, expected_dictionary);
    assert!(dictionary_read.report.dictionaries_verified);

    let lookback_dir = TestDir::new();
    write_similar_files(lookback_dir.path(), 12, 64 * 1024);
    let lookback_archive = plan_directory(
        lookback_dir.path(),
        PackOptions {
            profile: CompressionProfile::Dense,
            ..PackOptions::default()
        },
    )
    .unwrap();
    let lookback_chunk = lookback_archive
        .content_store
        .physical_order
        .iter()
        .rev()
        .find(|chunk| {
            lookback_archive.content_store.chunks[chunk]
                .group_ref
                .is_some()
        })
        .copied()
        .unwrap();
    let lookback_path = path_containing_chunk(&lookback_archive, lookback_chunk);
    let expected_lookback =
        std::fs::read(lookback_dir.path().join(lookback_path.to_string())).unwrap();
    let lookback_bytes = encode(&lookback_archive, WriteOptions::default())
        .unwrap()
        .bytes;
    let mut lookback_reader = open_indexed_random(
        entrybound::random_access::MemoryRandomReadSource::new(lookback_bytes),
        RandomAccessPolicy::default(),
    )
    .unwrap();
    let lookback_read = lookback_reader.read_entry(&lookback_path).unwrap();
    assert_eq!(&*lookback_read.bytes, expected_lookback);
    assert!(lookback_read.report.groups_verified);
    assert!(lookback_read.report.dependency_chunk_count > 0);
}

#[test]
fn random_access_verifies_deflate_reconstruction_dependencies() {
    let directory = TestDir::new();
    let source = (0..20_000)
        .flat_map(|index| {
            format!(
                "row={index:06};value={:08x};category={}\n",
                index * 17,
                index % 31
            )
            .into_bytes()
        })
        .collect::<Vec<_>>();
    let mut gzip = GzEncoder::new(Vec::new(), Compression::new(6));
    gzip.write_all(&source).unwrap();
    let original = gzip.finish().unwrap();
    std::fs::write(directory.path().join("payload.gz"), &original).unwrap();
    let archive = plan_directory(
        directory.path(),
        PackOptions {
            profile: CompressionProfile::Dense,
            ..PackOptions::default()
        },
    )
    .unwrap();
    assert!(!archive.content_store.reconstruction_data.is_empty());
    let bytes = encode(&archive, WriteOptions::default()).unwrap().bytes;
    let mut reader = open_indexed_random(
        entrybound::random_access::MemoryRandomReadSource::new(bytes),
        RandomAccessPolicy::default(),
    )
    .unwrap();
    let read = reader
        .read_entry(&LogicalPath::from_utf8(["payload.gz"]).unwrap())
        .unwrap();
    assert_eq!(&*read.bytes, original);
    assert!(read.report.reconstruction_verified);
}
