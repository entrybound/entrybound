# Original Cargo archive collector

`collect_cargo_archives.py` collects original crates.io archives from an existing
Cargo registry cache. It uses only Python 3.11+ standard-library code on Linux/WSL.
It does not invoke Cargo, download packages, run lifecycle scripts or extract
archive payloads.

The current intended source is the existing tuning ripgrep 14.1.1 `Cargo.lock`,
from `f01-tuning-ripgrep-14-1-1-git`, pinned at upstream commit
`4649aa9700619f94cf9c66876e9549d83420e16c`. Its SHA256 is
`1a99f2f04d96411238379485e36623a19b0158d9446796e6d85d58746023f441`.
All 51 registry packages are selected by the lock, independently of experiment
results. Their original compressed bytes total 15,055,640 bytes in the retained
offline preparation. This tool has been qualified with synthetic inputs only.

## Interface

Supply these arguments through an authorized unrestricted recipe:

```text
--lock <pinned existing Cargo.lock>
--lock-sha256 <64 lowercase hex>
--cache <Cargo registry/cache directory containing registry shard directories>
--scratch-root <caller-owned private D-backed WSL scratch root>
--out <absent direct child of the private scratch root>
--source-commit <40 lowercase hex>
--source-split tuning
--expected-count 51
--max-bytes 67108864
```

The scratch root must already exist, belong to the current UID and have mode 0700.
The caller must exclusively control this D-backed WSL scratch during the whole
operation and exclude concurrent same-UID directory changes. Runtime ownership
and mode checks are tripwires; they cannot prove that another process with the
same UID will respect this contract. Source split and `EB_SPLIT`, when present,
must be tuning or validation. Paths containing a `heldout` component or `..`,
symlink ancestors, selected archive symlinks and nonregular archive files are
refused. This interface grants no held-out access.

The output is `packages/<name>-<version>.crate`, the unchanged `Cargo.lock`, and
`provenance-notices.json`. The manifest contains source, lock, tool and archive
pins; original registry URLs; package metadata checks; and embedded license and
notice paths, byte counts and hashes. Cache paths, output paths and run timestamps
are omitted. Outer files use mode 0644, directories 0755 and mtime 1767225600;
ownership is that of the caller. Archive bytes and internal metadata are unchanged.

## Refusals and bounds

The collector validates the complete lock population before publication. It
refuses unsupported registries, count mismatches, missing or duplicate cache
archives, checksum failures, malformed gzip/tar/TOML, duplicate or unsafe archive
members (including repeated slashes and file trailing slashes), file/directory
hierarchy conflicts, links/special files, package name/version mismatches, and absent or
unsupported license evidence. No package-manager or network fallback exists.

License expressions are limited to the eight reviewed declarations in the pinned
population. Explicit `AND` clauses require every operand; explicit `OR` permits a
fully evidenced alternative; legacy slash declarations conservatively require
both texts. Required text evidence comes from top-level license/notice files.
Every nested standalone notice is also indexed, and every other embedded notice
is preserved through original-archive byte equality. The manifest records text
evidence and makes no redistribution or corpus acceptance claim.

The 64 MiB ceiling includes all output file bytes: original archives, lock and
manifest. `--max-bytes` may lower it. Input lock/Cargo.toml reads are bounded at
1 MiB, a notice at 2 MiB, collection notice bytes at 8 MiB, each expanded gzip tar
at 128 MiB and members per archive at 100,000. Verified compressed originals are
buffered before publication; expanded data is inspected one archive at a time.
These memory limits are separate from the output-file byte ceiling.

After validation, exclusive `mkdir` creates the absent output as an owned private
stage under the pinned scratch descriptor. The no-follow stage identity is
captured before its first open. Every existing output is refused,
including an empty directory. Validation failures leave no output. Write-failure
cleanup checks the created directory's device, inode and UID against the current
no-follow entry before removing it. An empty owned stage is removed without
opening it, including when its first open fails under a restrictive umask. A
moved/replaced entry causes cleanup refusal;
the replacement and relocated directory are left untouched for owner recovery.

No-follow descriptors reject links. They do not prevent ordinary-directory
replacement or relocation by a concurrent process with the same UID. This tool
requires trusted scratch and does not close the product section 11.6 directory
concurrency gate. The caller must run the binding disk guard before an authorized
actual run.

## Corpus scope

This is tooling for a capture extension of `f03-tuning-ripgrep-cargo-vendor`, using
the same tuning ripgrep lineage. `build_tree.py` can invoke the collector as a
command with `{python}`, `{repo}`, `{src}/Cargo.lock` and
`{cache}/cargo-home/registry/cache`, using a separately created private scratch
root for the collector output, then capture that directory alongside `vendor`.
No source definition, corpus membership, item ID, existing data, pin, manifest or
held-out lock is changed by adding this tool. Actual recipe changes and
materialization need a separate assignment.

A successful collection retains byte-identical original archives and their notice
evidence. Corpus admission, sample independence, and coverage of other source
families require separate evidence.
