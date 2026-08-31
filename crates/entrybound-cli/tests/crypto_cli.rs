use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use entrybound::crypto::XWingIdentity;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "entrybound-crypto-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
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

    let private_inspect = run(&[
        "inspect",
        archive.to_str().unwrap(),
        "--crypto",
        "--identity",
        identity_path.to_str().unwrap(),
    ]);
    assert!(
        private_inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&private_inspect.stderr)
    );
    let private_inspect = String::from_utf8(private_inspect.stdout).unwrap();
    assert!(private_inspect.contains("encrypted Descriptor record version: 2"));
    assert!(private_inspect.contains("producer resource declaration: present"));
    assert!(private_inspect.contains("matches authenticated archive reality"));
    assert!(private_inspect.contains("declared resources:"));
    assert!(private_inspect.contains("declared decode:"));

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

    let signer = fixture.0.join("signer.key");
    let generated = run(&["key", "generate-signing", signer.to_str().unwrap()]);
    assert!(generated.status.success());
    let embedded = run(&[
        "sign",
        archive.to_str().unwrap(),
        "--signing-key",
        signer.to_str().unwrap(),
        "--embed",
        "--identity",
        identity_path.to_str().unwrap(),
    ]);
    assert!(
        embedded.status.success(),
        "{}",
        String::from_utf8_lossy(&embedded.stderr)
    );
    let signed = run(&[
        "verify",
        archive.to_str().unwrap(),
        "--identity",
        identity_path.to_str().unwrap(),
        "--signatures",
        "--require-addressing-signature",
    ]);
    assert!(signed.status.success());
    assert!(
        String::from_utf8(signed.stdout)
            .unwrap()
            .contains("addressing=VALID")
    );

    let (retained_identity, retained_recipient) = XWingIdentity::generate().unwrap();
    let retained_identity_path = fixture.0.join("retained-identity.ebk");
    let retained_recipient_path = fixture.0.join("retained-recipient.ebk");
    std::fs::write(
        &retained_identity_path,
        retained_identity.encode_file().unwrap(),
    )
    .unwrap();
    std::fs::write(
        &retained_recipient_path,
        retained_recipient.encode_file().unwrap(),
    )
    .unwrap();
    let added = run(&[
        "key",
        "add",
        archive.to_str().unwrap(),
        "--identity",
        identity_path.to_str().unwrap(),
        "--recipient",
        retained_recipient_path.to_str().unwrap(),
    ]);
    assert!(
        added.status.success(),
        "{}",
        String::from_utf8_lossy(&added.stderr)
    );
    let stale = run(&[
        "verify",
        archive.to_str().unwrap(),
        "--identity",
        retained_identity_path.to_str().unwrap(),
        "--signatures",
    ]);
    assert!(stale.status.success());
    let stale = String::from_utf8(stale.stdout).unwrap();
    assert!(stale.contains("content=VALID"));
    assert!(stale.contains("physical=VALID"));
    assert!(stale.contains("addressing=STALE"));

    let listed = run(&[
        "key",
        "list",
        archive.to_str().unwrap(),
        "--identity",
        retained_identity_path.to_str().unwrap(),
    ]);
    assert!(listed.status.success());
    assert_eq!(
        String::from_utf8(listed.stdout)
            .unwrap()
            .matches("fingerprint=")
            .count(),
        2
    );

    let removed = run(&[
        "key",
        "remove",
        archive.to_str().unwrap(),
        "--identity",
        identity_path.to_str().unwrap(),
        "--retain",
        retained_recipient_path.to_str().unwrap(),
    ]);
    assert!(
        removed.status.success(),
        "{}",
        String::from_utf8_lossy(&removed.stderr)
    );
    assert!(
        !run(&[
            "verify",
            archive.to_str().unwrap(),
            "--identity",
            identity_path.to_str().unwrap(),
        ])
        .status
        .success()
    );
    assert!(
        run(&[
            "verify",
            archive.to_str().unwrap(),
            "--identity",
            retained_identity_path.to_str().unwrap(),
        ])
        .status
        .success()
    );
}

#[test]
fn detached_signing_cli_enforces_requested_bindings() {
    let fixture = Fixture::new();
    let archive = fixture.0.join("plain.eb");
    let signer = fixture.0.join("signer.key");
    let signature = fixture.0.join("plain.ebsig");
    assert!(
        run(&[
            "pack",
            fixture.0.join("source").to_str().unwrap(),
            archive.to_str().unwrap(),
        ])
        .status
        .success()
    );
    assert!(
        run(&["key", "generate-signing", signer.to_str().unwrap(),])
            .status
            .success()
    );
    assert!(
        run(&[
            "sign",
            archive.to_str().unwrap(),
            "--signing-key",
            signer.to_str().unwrap(),
            "--detached",
            signature.to_str().unwrap(),
        ])
        .status
        .success()
    );
    let verified = run(&[
        "verify",
        archive.to_str().unwrap(),
        "--signature",
        signature.to_str().unwrap(),
        "--require-content-signature",
        "--require-physical-signature",
    ]);
    assert!(
        verified.status.success(),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );
}

fn run(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ebound"))
        .args(arguments)
        .output()
        .unwrap()
}
