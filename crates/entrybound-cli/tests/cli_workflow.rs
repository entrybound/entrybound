use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
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
    assert!(inspection.contains("planner: balanced-v3"));
    assert!(inspection.contains("chunker: gear-norm-v1/"));
    assert!(inspection.contains("chunks: unique="));
    assert!(inspection.contains("codec usage: store/v1"));
    assert!(inspection.contains("codec usage: zstandard/v1"));
    assert!(inspection.contains("index: present and valid"));

    let explain = command(["explain", path(&archive)]);
    assert_success(&explain);
    let explanation = String::from_utf8_lossy(&explain.stdout);
    assert!(explanation.contains("planner: balanced-v3"));
    assert!(explanation.contains("exact deduplication:"));
    assert!(explanation.contains("logical Chunk references:"));
    assert!(explanation.contains("Zstandard: chunks="));
    assert!(explanation.contains("total Chunk-payload compression savings:"));
    assert!(explanation.contains("shared-dictionary payload savings:"));
    assert!(explanation.contains("bounded-lookback payload savings:"));
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
    assert!(String::from_utf8_lossy(&output.stdout).contains("planner dense-v3"));

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
    let mut state = 0xbb67_ae85_84ca_a73b_u64;
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
