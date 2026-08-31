use std::path::PathBuf;
use std::process::Command;

use entrybound::crypto::XWingIdentity;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("entrybound-crypto-cli-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(path.join("source/nested")).unwrap();
        std::fs::write(path.join("source/nested/private.txt"), b"secret CLI bytes").unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn hybrid_pack_public_inspect_verify_and_unpack() {
    let fixture = Fixture::new();
    let source = fixture.0.join("source");
    let archive = fixture.0.join("encrypted.eb");
    let destination = fixture.0.join("restored");
    let identity_path = fixture.0.join("identity.ebk");
    let recipient_path = fixture.0.join("recipient.ebk");
    let (identity, recipient) = XWingIdentity::generate().unwrap();
    std::fs::write(&identity_path, identity.encode_file().unwrap()).unwrap();
    std::fs::write(&recipient_path, recipient.encode_file().unwrap()).unwrap();

    let pack = run(&[
        "pack",
        source.to_str().unwrap(),
        archive.to_str().unwrap(),
        "--recipient",
        recipient_path.to_str().unwrap(),
    ]);
    assert!(
        pack.status.success(),
        "{}",
        String::from_utf8_lossy(&pack.stderr)
    );

    let public = run(&["inspect", archive.to_str().unwrap(), "--crypto"]);
    assert!(public.status.success());
    let public = String::from_utf8(public.stdout).unwrap();
    assert!(public.contains("encrypted: yes"));
    assert!(public.contains("private archive metadata: locked"));
    assert!(!public.contains("nested/private.txt"));

    let public_verify = run(&["verify", archive.to_str().unwrap()]);
    assert!(public_verify.status.success());
    let public_verify = String::from_utf8(public_verify.stdout).unwrap();
    assert!(public_verify.contains("PRIVATE CONTENT UNVERIFIED"));
    assert!(!public_verify.contains("OK authenticated and verified"));

    let verify = run(&[
        "verify",
        archive.to_str().unwrap(),
        "--identity",
        identity_path.to_str().unwrap(),
    ]);
    assert!(
        verify.status.success(),
        "{}",
        String::from_utf8_lossy(&verify.stderr)
    );
    assert!(
        String::from_utf8(verify.stdout)
            .unwrap()
            .contains("OK authenticated and verified")
    );

    let unpack = run(&[
        "unpack",
        archive.to_str().unwrap(),
        destination.to_str().unwrap(),
        "--identity",
        identity_path.to_str().unwrap(),
    ]);
    assert!(
        unpack.status.success(),
        "{}",
        String::from_utf8_lossy(&unpack.stderr)
    );
    assert_eq!(
        std::fs::read(destination.join("nested/private.txt")).unwrap(),
        b"secret CLI bytes"
    );

    let tampered_archive = fixture.0.join("tampered.eb");
    let tampered_destination = fixture.0.join("tampered-destination");
    let mut tampered = std::fs::read(&archive).unwrap();
    let footer = tampered.len() - 192;
    let segments =
        u64::from_be_bytes(tampered[footer + 40..footer + 48].try_into().unwrap()) as usize;
    tampered[segments + 64 + 64 + 32] ^= 1;
    std::fs::write(&tampered_archive, tampered).unwrap();
    let failure = run(&[
        "unpack",
        tampered_archive.to_str().unwrap(),
        tampered_destination.to_str().unwrap(),
        "--identity",
        identity_path.to_str().unwrap(),
    ]);
    assert!(!failure.status.success());
    assert!(!tampered_destination.exists());
}

fn run(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ebound"))
        .args(arguments)
        .output()
        .unwrap()
}
