# Qualification and release evidence

Status: development policy for the experimental implementation. Public format
semantics remain in the versioned specification and format notes; this document
defines how changes and release artifacts are checked.

## Source identity

Every qualification record names the full Git commit ID, working directory,
exact toolchain version, command, exit code, skipped cases, and hashes of
retained artifacts. For checks run before a commit, record the working-tree
diff digest and later verify that the committed tree contains the tested bytes.
Release candidates are checked afresh at their final commit. A successful run
on a previous commit, different branch, or uncommitted binary does not qualify
the new candidate.

## Local checks

Run focused positive and negative checks for each changed behavior, then the
canonical production-workspace checks from the repository root:

```sh
cargo fmt --all --check
cargo check -p entrybound -p entrybound-cli
cargo clippy -p entrybound -p entrybound-cli --all-targets -- -D warnings
cargo test -p entrybound -p entrybound-cli
git diff --check
```

Use the toolchain selected by `rust-toolchain.toml` and record its actual
version. The stable channel is not an immutable version pin. When workspace
packages or features change, the canonical commands must cover the new
production surface. Run applicable source, documentation-link, structured-data,
canonical-vector, historical-reader, dependency-license, and research-workspace
validators for affected files. Test assertions must check the relevant output
fields and failure reasons, not only process exit status.

On the current Windows research host, run the disk guard before long-running
research jobs and between their stages; other hosts need an equivalent storage
preflight. Keep heavy inputs, build output, and evidence on approved D:-backed
paths on that host. Retain raw results, negative findings,
source/configuration hashes, and decision records; remove only identified
reproducible scratch created by the current run. No credentials or private
execution records belong in a product commit or package.

## Research, platform, and security evidence

Decision-bearing research follows preregistered methods and the existing
experiment protocols. Preserve input versions, seeds, raw output, uncertainty,
variance, failed runs, and sensitivity analyses. A prepared protocol or
`READY` label does not mean an experiment ran. Alternative algorithms remain
research-only until their decision and compatibility gates accept them.

Platform claims require execution on the actual named OS, architecture,
filesystem, and privilege level. Record capture, representation, decode, and
restoration separately, including unsupported features and object state before
and after. Emulation and model review do not establish native behavior.

Security qualification includes targeted exploit, race, resource and crypto
composition cases, applicable fuzz campaigns and minimized regressions, and
independent external review where the release contract requires it. A passing
known-answer vector or mathematical signature check does not by itself prove
publisher trust or integrated security.

## Hosted checks and release candidates

The current qualification policy is local-only pending an owner decision on
hosted execution. This document does not enable a hosted runner or authorize
cost. If an owner-approved revision makes hosted jobs mandatory, record the
required provider, workflow revision, branch or pull-request candidate,
matrix, and budget before relying on those jobs. Every named mandatory job
must succeed on the exact resulting commit and required workflow revision.
Missing access, missing jobs, pending, skipped, canceled, failed, or stale-head
runs are not passes. Until activation, record hosted checks as inapplicable
under the current policy, never as successful runs.

A release candidate also needs exact-candidate builds and reproducible inputs,
the required native platform matrix, historical and independent conformance,
installation and uninstallation checks, safe defaults, license and SBOM
review, provenance, checksum and signature verification, complete docs, and
qualified performance claims. Recheck the result after any merge or artifact
change. Public release approval must name the exact commit and artifact
manifest; a prepared release is not a published release.

## Publication and recovery

For a release bundle, finish target preflight, record expected paths and hashes,
and verify staged and published bytes independently. Several files cannot
become visible or durable in one portable filesystem operation. If publication
is interrupted, inspect actual final and temporary paths and their hashes
before cleanup or retry; preserve originals and remove only verified run-owned
scratch. See
[migration workflows](migration-workflows-v1.md#transaction) for the current
multi-target implementation's exact guarantee and recovery procedure.

Release qualification must exercise temporary creation, destination collision
and replacement races, link failures, and cleanup failures. It must establish
that cleanup removes only artifacts created by the invocation and preserves
pre-existing and foreign destinations. Current pathname-based cleanup leaves
these required v1 transaction and cleanup gates unresolved. Controlled-directory
operation and recovery guidance do not satisfy those gates.
