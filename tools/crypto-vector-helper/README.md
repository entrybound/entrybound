# Crypto wire vector helper

This directory contains two non-production reference generators for the
documentation vectors in `docs/crypto-wire-v1.md`:

- `src/main.rs` uses the pinned RustCrypto primitives selected by the review;
- `reference.py` independently constructs every T1, ECF record, private-object
  envelope, and canonical sequence byte string. When its pinned Python
  dependencies are installed, it also independently runs Argon2id and
  AES-256-GCM-SIV.

All keys, nonces, salts, and passwords here are public fixed test inputs. None
of this package is linked into Entrybound, and none of its deterministic
interfaces may be copied into production encryption code.

Run:

```text
cargo run --locked --manifest-path tools/crypto-vector-helper/Cargo.toml
python tools/crypto-vector-helper/reference.py
```

The Python reference environment is pinned in `requirements.txt`. Both
programs print the same `key=value` lines. `reference.py --canonical-only`
requires only Python's standard library and independently reproduces all
transcripts, derivations, hashes, and sequence vectors except Argon2id and the
two final AES-GCM-SIV ciphertexts.

