# Reproducible crypto conformance

Run the frozen vectors and generated integration cases from a clean checkout:

```sh
cargo test --offline --locked -p entrybound --test crypto_primitives
cargo test --offline --locked -p entrybound --test crypto_configuration_matrix -- --nocapture --test-threads=1
cargo test --offline --locked -p entrybound -p entrybound-cli
cargo test --offline --locked -p entrybound -p entrybound-cli --features entrybound/research-internals
python tools/qualification/identity_reference.py
```

The configuration matrix enumerates all 96 combinations of four creation
profiles, three padding modes, two encrypted boundary modes, index present or
absent, and hybrid or password protection. Each cell reconstructs fourteen
files of lengths 0, 1, 15, 16, 17, 255, 256, 257, 4095, 4096, 4097, 65535,
65536 and 65537. Byte `i` is `(i * 73 + i / 11 + 19) mod 256` using integer
division. It checks exact reconstructed bytes, captured metadata and identities,
restored modification times, Unix permission bits, incorrect credentials and
truncation refusal. All 256 padding and boundary discriminants are checked
against their closed registries.

The empty-archive identity goldens are calculated by a separate Python program
using SHA-256 and explicit bootstrap transcript lengths from
[format-v0.md](format-v0.md). Its input is a Complete, empty archive with
`fixed-1mib/v1`, platform `test`, empty fidelity lists and no conversion record.
It imports no Entrybound implementation. The Rust test checks LAI, PCR and AUX
against those fixed goldens and checks deterministic plaintext serialization
both with and without an index. Optional file arguments calculate exact-byte
PCI through Python's SHA-256 backend.

For an exact-byte PCI cross-check, set `ENTRYBOUND_RETAIN_MATRIX_FIXTURES=1`
while running the matrix with `--nocapture`. The test retains its generated
fixtures under the current temporary directory and prints `MATRIX_PCI` rows
containing the actual archive path and reported PCI. Hash those paths using the
reference script and compare each value. Clean up only those generated fixture
directories after retaining the full results. Failed fixtures are retained even
without that variable. Test identity and signing seeds are public test inputs.

## Coverage beyond round trips

The normal workspace suite retains independent RFC 8452, RFC 5869, RFC 4231,
RFC 9106, X-Wing draft-10 and RFC 8032 expected values; frozen V1-V7 transcript
vectors; authentication and ciphertext/envelope/commitment/footer tampering;
unknown required features; malformed canonical records; caller resource limits;
signature freshness versus cryptographic validity; offline timestamp trust;
source containment and races; exact conversion/reversion; representation repack;
and plaintext INDEXED/STREAM equivalence. The crypto random-reader unit suite
checks exact budget boundaries, charged ownership lifetimes, authenticated
record framing, wipe ordering, partial verification claims and failed reads.

Encrypted STREAM and encrypted repack remain unsupported and must refuse.
Changing readers to accept those operations is a separate implementation task.

## Qualification limits

A finite configuration matrix covers the enumerated option combinations and
chosen boundaries. It does not exhaust all possible files, keys, integer
values, schedules, codecs' internal states or operating systems. Round trips
cannot independently establish a cryptographic construction; the separate
canonical vectors and negative checks provide additional evidence. These tests
do not prove whole-process memory bounds, resistance to every side channel or
external cryptographic assurance. Native Linux filesystem behavior must be
tested with unprivileged processes; Windows results must be labeled separately.
Unavailable platform and release assurance requirements retain their own gates.

Retain the exact commit/tree, source hashes, compiler, commands, input generators,
expected membership, actual logs, errors, fixture and binary hashes, and the
machine-readable coverage report. When verifying a retained archive, reproduce its recorded input hashes,
metadata comparisons and exact PCI. Fresh encryption generates new keys,
archive IDs, salts and padding, so PCI changes. Keyed chunking can also change
PCR. LAI is stable for equal entry semantics; AUX is stable only when captured
auxiliary metadata is unchanged. Freshly generated fixture timestamps can
change AUX. Deterministic plaintext serialization and the frozen cryptographic
vectors are checked separately from randomized archive creation.
