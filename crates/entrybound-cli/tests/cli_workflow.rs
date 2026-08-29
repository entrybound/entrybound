use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[test]
fn all_five_native_commands_operate_on_real_archives() {
    let fixture = Fixture::new();
    let source = fixture.path.join("source");
    let archive = fixture.path.join("fixture.eb");
    let restored = fixture.path.join("restored");
    fs::create_dir(&source).unwrap();
    fs::create_dir(source.join("nested")).unwrap();
    fs::create_dir(source.join("empty-dir")).unwrap();
    fs::write(source.join("nested/hello.txt"), b"hello from the CLI\n").unwrap();
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
    assert!(inspection.contains("planner: bootstrap-store-v1"));
    assert!(inspection.contains("index: present and valid"));

    let unpack = command(["unpack", path(&archive), path(&restored)]);
    assert_success(&unpack);
    assert_eq!(
        fs::read(restored.join("nested/hello.txt")).unwrap(),
        b"hello from the CLI\n"
    );
    assert!(restored.join("empty-dir").is_dir());
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

fn command<const N: usize>(arguments: [&str; N]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_entrybound"))
        .args(arguments)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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
