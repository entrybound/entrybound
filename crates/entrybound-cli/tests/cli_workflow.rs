use std::fs::{self, File};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[test]
fn all_native_commands_operate_on_real_archives() {
    let fixture = Fixture::new();
    let source = fixture.path.join("source");
    let archive = fixture.path.join("fixture.eb");
    let restored = fixture.path.join("restored");
    fs::create_dir(&source).unwrap();
    fs::create_dir(source.join("nested")).unwrap();
    fs::create_dir(source.join("empty-dir")).unwrap();
    fs::write(source.join("nested/hello.txt"), b"hello from the CLI\n").unwrap();
    fs::write(source.join("compressible.bin"), vec![b'x'; 128 * 1024]).unwrap();
    fs::write(
        source.join("incompressible.bin"),
        deterministic_noise(128 * 1024),
    )
    .unwrap();
    fs::write(source.join("empty"), []).unwrap();

    let pack = command(["pack", path(&source), path(&archive)]);
    assert_success(&pack);
    assert!(String::from_utf8_lossy(&pack.stdout).contains("OK packed"));

    let verify = command(["verify", path(&archive)]);
    assert_success(&verify);
    assert!(String::from_utf8_lossy(&verify.stdout).contains("exact-byte PCI"));

    let list = command(["list", path(&archive)]);
    assert_success(&list);
    let listing = String::from_utf8_lossy(&list.stdout);
    assert!(listing.contains("directory\tnested"));
    assert!(listing.contains("file\tnested/hello.txt"));

    let inspect = command(["inspect", path(&archive)]);
    assert_success(&inspect);
    let inspection = String::from_utf8_lossy(&inspect.stdout);
    assert!(inspection.contains("format: ecf/bootstrap-v1"));
    assert!(inspection.contains("planner: balanced-v6"));
    assert!(inspection.contains("chunker: gear-norm-v1/"));
    assert!(inspection.contains("chunks: unique="));
    assert!(inspection.contains("codec usage: store/v1"));
    assert!(inspection.contains("codec usage: zstandard/v1"));
    assert!(inspection.contains("whole-object reconstruction: feature=true"));
    assert!(inspection.contains("index: present and valid"));

    let explain = command(["explain", path(&archive)]);
    assert_success(&explain);
    let explanation = String::from_utf8_lossy(&explain.stdout);
    assert!(explanation.contains("planner: balanced-v6"));
    assert!(explanation.contains("exact deduplication:"));
    assert!(explanation.contains("logical Chunk references:"));
    assert!(explanation.contains("Zstandard: chunks="));
    assert!(explanation.contains("total Chunk-payload compression savings:"));
    assert!(explanation.contains("shared-dictionary payload savings:"));
    assert!(explanation.contains("bounded-lookback payload savings:"));
    assert!(explanation.contains("JPEG reconstruction:"));
    assert!(explanation.contains("similarity cohorts:"));

    let unpack = command(["unpack", path(&archive), path(&restored)]);
    assert_success(&unpack);
    assert_eq!(
        fs::read(restored.join("nested/hello.txt")).unwrap(),
        b"hello from the CLI\n"
    );
    assert!(restored.join("empty-dir").is_dir());
}

#[test]
fn pack_help_and_profile_option_are_available_only_at_creation() {
    let help = command(["pack", "--help"]);
    assert_success(&help);
    assert!(String::from_utf8_lossy(&help.stdout).contains("--profile"));

    let fixture = Fixture::new();
    let source = fixture.path.join("source");
    let archive = fixture.path.join("dense.eb");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("data"), vec![0_u8; 4096]).unwrap();
    let output = command(["pack", path(&source), path(&archive), "--profile", "dense"]);
    assert_success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("planner dense-v6"));

    let error = command(["verify", path(&archive), "--profile", "fast"]);
    assert!(!error.status.success());
    assert!(String::from_utf8_lossy(&error.stderr).contains("EB_CLI_USAGE"));
}

#[test]
fn malformed_archive_prints_stable_diagnostic_and_fails() {
    let fixture = Fixture::new();
    let archive = fixture.path.join("bad.eb");
    fs::write(&archive, b"not entrybound").unwrap();
    let output = command(["verify", path(&archive)]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("NONCONFORMING EB_ECF_BAD_MAGIC"));
    assert!(!stderr.contains("Diagnostic {"));
}

#[test]
fn entrybound_alias_uses_the_same_cli_implementation() {
    let primary = command_with(env!("CARGO_BIN_EXE_ebound"), ["--help"]);
    let alias = command_with(env!("CARGO_BIN_EXE_entrybound"), ["--help"]);
    assert_success(&primary);
    assert_success(&alias);
    assert_eq!(primary.stdout, alias.stdout);
    assert_eq!(primary.stderr, alias.stderr);
    assert!(String::from_utf8_lossy(&primary.stdout).contains("ebound pack"));
}

#[test]
fn a_real_pipe_carries_a_stream_archive_from_pack_to_verify() {
    let fixture = Fixture::new();
    let source = fixture.path.join("source");
    write_pipe_source(&source);

    // `ebound pack ./tree - --layout stream` writes archive bytes to standard
    // output, so every status line must go to standard error instead.
    let mut pack = Command::new(env!("CARGO_BIN_EXE_ebound"))
        .args(["pack", path(&source), "-", "--layout", "stream"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let verify = Command::new(env!("CARGO_BIN_EXE_ebound"))
        .args(["verify", "-"])
        .stdin(Stdio::from(pack.stdout.take().unwrap()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let pack = pack.wait_with_output().unwrap();
    assert_success(&pack);
    assert_success(&verify);

    let pack_status = String::from_utf8_lossy(&pack.stderr);
    assert!(pack_status.contains("OK packed"));
    assert!(pack_status.contains("layout STREAM"));
    assert!(pack_status.contains("stream dedup window 0"));
    assert!(pack_status.contains("PCI "));
    assert!(
        pack.stdout.is_empty(),
        "status output must not share the byte stream"
    );

    let verified = String::from_utf8_lossy(&verify.stdout);
    assert!(verified.contains("exact-byte PCI"));
    assert!(verified.contains("stream dedup-window constraints"));
    assert!(verified.contains("layout: STREAM"));
    assert!(verified.contains("not applicable; STREAM layout carries no Index by design"));
}

#[test]
fn stream_archives_round_trip_through_standard_input_and_output() {
    let fixture = Fixture::new();
    let source = fixture.path.join("source");
    let archive = fixture.path.join("piped.eb");
    let restored = fixture.path.join("restored");
    write_pipe_source(&source);

    let pack = Command::new(env!("CARGO_BIN_EXE_ebound"))
        .args(["pack", path(&source), "-", "--layout", "stream"])
        .stdout(Stdio::from(File::create(&archive).unwrap()))
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_success(&pack);
    let bytes = fs::read(&archive).unwrap();
    assert_eq!(&bytes[..8], &[0x8e, b'E', b'B', b'1', 13, 10, 26, 10]);
    assert_eq!(bytes[72], 2, "the preamble must declare STREAM layout");

    for (arguments, expected) in [
        (vec!["list", "-"], "file\tnested/hello.txt"),
        (vec!["inspect", "-"], "layout: STREAM"),
        (vec!["explain", "-"], "exact deduplication:"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_ebound"))
            .args(&arguments)
            .stdin(Stdio::from(File::open(&archive).unwrap()))
            .output()
            .unwrap();
        assert_success(&output);
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(expected),
            "{arguments:?} did not report {expected}"
        );
    }

    let inspect = Command::new(env!("CARGO_BIN_EXE_ebound"))
        .args(["inspect", "-"])
        .stdin(Stdio::from(File::open(&archive).unwrap()))
        .output()
        .unwrap();
    let inspection = String::from_utf8_lossy(&inspect.stdout);
    assert!(inspection.contains("stream dedup window: 0"));
    assert!(inspection.contains("producer budget declaration: declared before the payload"));
    assert!(inspection.contains("random entry lookup: unavailable"));
    assert!(inspection.contains("index: not applicable; STREAM layout carries no Index by design"));
    assert!(inspection.contains("stream access: random-entry-lookup=false"));

    let unpack = Command::new(env!("CARGO_BIN_EXE_ebound"))
        .args(["unpack", "-", path(&restored)])
        .stdin(Stdio::from(File::open(&archive).unwrap()))
        .output()
        .unwrap();
    assert_success(&unpack);
    assert!(String::from_utf8_lossy(&unpack.stdout).contains("layout: STREAM"));
    assert_eq!(
        fs::read(restored.join("nested/hello.txt")).unwrap(),
        b"hello from the CLI\n"
    );
    assert!(restored.join("empty-dir").is_dir());
}

#[test]
fn a_stream_archive_in_a_file_is_never_reported_as_random_access() {
    let fixture = Fixture::new();
    let source = fixture.path.join("source");
    let archive = fixture.path.join("seekable.eb");
    write_pipe_source(&source);

    let pack = command(["pack", path(&source), path(&archive), "--layout", "stream"]);
    assert_success(&pack);
    assert!(String::from_utf8_lossy(&pack.stdout).contains("layout STREAM"));

    let inspect = command(["inspect", path(&archive)]);
    assert_success(&inspect);
    let inspection = String::from_utf8_lossy(&inspect.stdout);
    assert!(inspection.contains("layout: STREAM"));
    assert!(inspection.contains("random entry lookup: unavailable"));

    let list = command(["list", path(&archive)]);
    assert_success(&list);
    assert!(
        String::from_utf8_lossy(&list.stderr).contains("required a complete sequential pass"),
        "listing a STREAM archive must say what it cost"
    );
}

#[test]
fn a_truncated_stream_reports_truncation_rather_than_generic_corruption() {
    let fixture = Fixture::new();
    let source = fixture.path.join("source");
    let archive = fixture.path.join("whole.eb");
    let cut = fixture.path.join("cut.eb");
    write_pipe_source(&source);
    assert_success(&command([
        "pack",
        path(&source),
        path(&archive),
        "--layout",
        "stream",
    ]));

    let bytes = fs::read(&archive).unwrap();
    fs::write(&cut, &bytes[..bytes.len() - 200]).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ebound"))
        .args(["verify", "-"])
        .stdin(Stdio::from(File::open(&cut).unwrap()))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(3), "TRUNCATED exit class");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("TRUNCATED EB_ECF_TRUNCATED_STREAM"),
        "{stderr}"
    );
}

#[test]
fn a_window_that_cannot_be_met_fails_with_a_typed_diagnostic() {
    let fixture = Fixture::new();
    let source = fixture.path.join("source");
    let archive = fixture.path.join("shared.eb");
    // Each file owns content the other does not, so both contribute Chunks and
    // the second record read necessarily reaches back to the first file's run.
    fs::create_dir(&source).unwrap();
    let shared = structured(1024 * 1024);
    let mut leading = seeded_noise(128 * 1024, 0x0bad_cafe_0bad_cafe);
    leading.extend_from_slice(&shared);
    let mut trailing = shared;
    trailing.extend_from_slice(&seeded_noise(128 * 1024, 0x0fee_1900_0fee_1900));
    fs::write(source.join("unique-head-then-shared.bin"), &leading).unwrap();
    fs::write(source.join("shared-then-unique-tail.bin"), &trailing).unwrap();

    let refused = command(["pack", path(&source), path(&archive), "--layout", "stream"]);
    assert!(!refused.status.success());
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("EB_ECF_STREAM_WINDOW_EXCEEDED"), "{stderr}");
    assert!(stderr.contains("automatic window"), "{stderr}");
    assert!(
        !archive.exists(),
        "a refused plan must not leave an output file behind"
    );

    let automatic = fixture.path.join("auto.eb");
    let packed = command([
        "pack",
        path(&source),
        path(&automatic),
        "--layout",
        "stream",
        "--stream-window",
        "auto",
    ]);
    assert_success(&packed);
    let stdout = String::from_utf8_lossy(&packed.stdout);
    assert!(stdout.contains("stream dedup window "));
    assert!(!stdout.contains("stream dedup window 0"));
    assert_success(&command(["verify", path(&automatic)]));
}

fn write_pipe_source(source: &std::path::Path) {
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::create_dir(source.join("empty-dir")).unwrap();
    fs::write(source.join("nested/hello.txt"), b"hello from the CLI\n").unwrap();
    fs::write(source.join("compressible.bin"), vec![b'x'; 128 * 1024]).unwrap();
    fs::write(
        source.join("incompressible.bin"),
        deterministic_noise(128 * 1024),
    )
    .unwrap();
    fs::write(source.join("empty"), []).unwrap();
}

fn structured(len: usize) -> Vec<u8> {
    (0..len)
        .map(|index| ((index % 251) as u8).wrapping_add((index / 251) as u8))
        .collect()
}

fn command<const N: usize>(arguments: [&str; N]) -> Output {
    command_with(env!("CARGO_BIN_EXE_ebound"), arguments)
}

fn command_with<const N: usize>(executable: &str, arguments: [&str; N]) -> Output {
    Command::new(executable).args(arguments).output().unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn deterministic_noise(len: usize) -> Vec<u8> {
    seeded_noise(len, 0xbb67_ae85_84ca_a73b)
}

fn seeded_noise(len: usize, seed: u64) -> Vec<u8> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

fn path(value: &std::path::Path) -> &str {
    value.to_str().unwrap()
}

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "entrybound-cli-{}-{}",
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
