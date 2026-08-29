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
cargo run -p entrybound-cli --bin ebound -- --help
```

The core crate uses RustCrypto's `sha2` for SHA-256, `cap-std` for
capability-relative filesystem traversal, and `zstd` 0.13.3 for the operational
Zstandard codec. The `zstd` crate is MIT licensed; its bindings use the
redistributable upstream Zstandard implementation, and default optional
features are disabled. Additional dependencies require a concrete format or
implementation need. Keep the CLI crate thin: semantic rules belong in
`entrybound::eam`, format rules in `entrybound::ecf`, identity rules in
`entrybound::identity`, creation-time codec selection in
`entrybound::planner`, operational codec dispatch below ECF, and filesystem
workflows and caller-owned resource policy in `entrybound::archive`.

The primary executable is `ebound`. The `entrybound` executable is a thin
compatibility alias; both call the shared implementation in the
`entrybound-cli` package library. Do not duplicate command logic between them.

Filesystem tests must use generated trees. Extraction changes need collision,
containment, preflight-integrity, and resource-policy tests; do not add opaque
archive fixtures or path-string shortcuts.

Compression planner behavior is frozen by planner ID. Do not change the
candidate levels, probe, cost rule, or parameter encoding of a `*-v1` policy;
introduce a new planner version. Decoder behavior must depend only on recorded
TransformPlans, never on the creation profile.

Generated conformance inputs belong in tests as format-building code, not as
opaque binary fixtures. Each negative case must assert a stable reason code.
