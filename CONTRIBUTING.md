# Contributing to Entrybound

Entrybound is at the experimental format-bootstrap stage. Changes to native
semantics or wire encoding must be checked against the Product Architecture
Specification and accompanied by focused conformance tests.

## Development

Rust is Entrybound's primary implementation language. The stable
toolchain is declared in `rust-toolchain.toml` and includes rustfmt and Clippy.

```sh
cargo fmt --all --check
cargo check -p entrybound -p entrybound-cli
cargo clippy -p entrybound -p entrybound-cli --all-targets -- -D warnings
cargo test -p entrybound
```

The core crate uses RustCrypto's `sha2` for SHA-256 and `cap-std` for
capability-relative filesystem traversal. Additional dependencies require a
concrete format or implementation need. Keep the CLI crate thin: semantic
rules belong in `entrybound::eam`, format rules in `entrybound::ecf`, identity
rules in `entrybound::identity`, and filesystem workflows and caller-owned
resource policy in `entrybound::archive`.

Filesystem tests must use generated trees. Extraction changes need collision,
containment, preflight-integrity, and resource-policy tests; do not add opaque
archive fixtures or path-string shortcuts.

Generated conformance inputs belong in tests as format-building code, not as
opaque binary fixtures. Each negative case must assert a stable reason code.
