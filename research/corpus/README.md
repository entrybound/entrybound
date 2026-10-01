# Corpus tools

Run the tools inside WSL from the repository root. The source definitions and
fingerprint records are described in [methodology.md](methodology.md).

## Metadata commands

```sh
research/corpus/tools/run.sh provision --check
research/corpus/tools/run.sh provision --list --family F04
research/corpus/tools/run.sh provision --list --group example-group --split validation
```

`--check` validates every source definition and reports only item, source-file,
warning and error counts. It reads no per-item fingerprint records or payload
directories. Selection filters do not narrow validation. If combined with
`--list`, `--check` takes precedence.

`--list` validates the definitions, then applies the family, group, item and split
filters before checking fingerprint record files. Filters may be repeated or comma
separated. The default splits are `tuning` and `validation`; selecting an item
does not override that default. Listing does not include dependencies implicitly.
`recorded` and `not-recorded` describe the presence of a regular metadata record
file only; they do not parse the record or verify payload presence or integrity.
The selected file check does not follow symlinks. Nonregular or inaccessible
selected record entries cause an aggregate refusal with exit status 2. No payload
directories or orphan directories are inspected.

Both commands report source diagnostics as counts because detailed diagnostics
can disclose unrelated held-out identities and upstreams. Authorized curator work
can use the existing `corpuslib.load_sources()` diagnostic results under the
applicable custody rules. Source errors still cause exit status 2.

An explicit `--split heldout` requests selected held-out metadata and remains
subject to the existing identity exposure and custody rules. It grants no content
access and makes no change to the held-out lock or unlock procedure in
[methodology.md, section 9](methodology.md#9-held-out-access-control).

For help (`-h` or `--help`) and metadata commands, `run.sh` uses an existing
research virtual environment or falls back to `python3`. It does not create the
virtual environment, create data directories or change held-out permissions.
Direct invocation is also supported:

```sh
python3 -B research/corpus/tools/provision.py --check
```

## Materialization

```sh
research/corpus/tools/run.sh provision --family F04 --split tuning --offline
```

Materialization retains its existing data-root setup, dependency inclusion,
source validation, cache verification and offline behavior. Invalid definition
types fail validation before dependency/group/upstream graph processing. Read the
methodology before provisioning or inspecting held-out content.
