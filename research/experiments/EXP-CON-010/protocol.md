# EXP-CON-010: Platform identification of `.eb` files on macOS and in web browsers

| Field | Value |
|---|---|
| Status | **BLOCKED** — (a) `PLATFORM_BLOCKED`: macOS (Launch Services, Uniform Type Identifiers, Finder, Quick Look, Spotlight `mdls`) is unavailable to this program; (b) no browser automation environment: Chromium on Ubuntu 24.04 is snap-only and snapd does not run in this WSL distro, no browser is installed on the host for automation, and the ecosystem identification experiment `EXP-ECO-012` excludes browser download sniffing for the same reason |
| Exact missing access | A macOS 15 (or current) host with a non-admin user and `mdls`, `mdimport`, `plutil`, `lsregister` available; a headless Chromium and Firefox at pinned versions with automation (Playwright or WebDriver) on any host, plus an HTTP server serving test files with controlled `Content-Type` and `Content-Disposition` headers |
| Domain / program section | container / §11 (with §32) |
| decision_ids | DEC-CON-021, DEC-CON-019, DEC-CON-017 |
| Decision type (proposal) | EMPIRICAL (identification behaviour); registrations remain `EXTERNAL_REVIEW_REQUIRED` (§8.3) |
| OD / HC (proposal) | OD-26 (M26.4 identification support), OD-25 |
| Runner | Scripted probes (no corpus-item loop, no `spec.yaml`) |
| Method, author, L7, gates | `../_index/container-conventions.md` §1-§2 |

## 1. Question

How do macOS (Launch Services and UTI conformance, Finder kind, Quick Look, Spotlight metadata, Archive Utility association) and the major browsers (download sniffing, file-extension handling on save, `Content-Type` interplay) identify and handle files named `.eb` and `.eba` whose bytes are Entrybound archives, EasyBuild easyconfig text files, or other formats, with and without a registered UTI or media type?

## 2. Hypotheses

- **H1 (macOS).** Without an exported UTI, `mdls -name kMDItemContentType` reports a dynamic `dyn.*` type for `.eb` Entrybound files and for `.eb` easyconfig files alike (extension-only, magic ignored); with an app-declared UTI for `.eb`, text easyconfig files are also claimed by it (no magic arbitration).
- **H2 (browsers).** Chromium and Firefox save a `.eb` download served as `application/octet-stream` under its URL name without renaming, and a response served as `text/plain` with `.eb` content is not sniffed as an archive (WHATWG MIME Sniffing has no rule for the Entrybound magic).
- **Falsification.** H1 by magic-based arbitration on macOS; H2 by any rename or archive sniffing.

## 3. Decisions informed

| Decision | Supplied here (once unblocked) | Elsewhere now |
|---|---|---|
| DEC-CON-021 | Extension-only identification behaviour on macOS and browsers for `.eb` vs `.eba` | Collision survey, libmagic and shared-mime-info glob-vs-magic, Windows association: EXP-ECO-012 |
| DEC-CON-019 | Whether a registered media type or exported UTI changes platform handling | IANA registry survey and RFC 6838 template: EXP-ECO-012 and external review |
| DEC-CON-017 | Whether any macOS or browser path consults leading magic bytes | Magic prefix-collision and transport-damage tests: EXP-ECO-012 |

## 4. Candidates

Extensions `.eb` (status quo), `.eba` (fallback), `.entrybound` (longer); with and without an app-declared UTI (a minimal test app bundle declaring `UTExportedTypeDeclarations` for `org.entrybound.archive`, conforming to `public.data` and `public.archive`); media types `application/vnd.entrybound.archive`, `application/octet-stream`, `text/plain` on the HTTP arm.

## 5. Inputs

Status-quo archives packed from `f01-tuning-ripgrep-14-1-1-git` and `f04-tuning-sourcelike` (tuning items) by `ebound-prod`; easyconfig text files from a pinned clone of `easybuilders/easybuild-easyconfigs`; renamed ZIP and tar files as controls. No held-out data.

## 6. Environment and platform requirements

macOS host (missing); headless Chromium and Firefox at pinned versions with automation (missing); local HTTP server (available). No timing.

## 7. Commands (to run once unblocked)

```sh
# macOS
bash research/tools/container/platform-ident/macos_probe.sh --files research/experiments/EXP-CON-010/test-files.json --out research/raw/EXP-CON-010/macos.jsonl
# browsers
python3 research/tools/container/platform-ident/browser_probe.py --server-root <test files> --browsers chromium@<pinned>,firefox@<pinned> --out research/raw/EXP-CON-010/browsers.jsonl
```

Tooling to build when unblocked: `macos_probe.sh` (records `mdls` content type and conformance tree, Finder kind via AppleScript `kind`, `qlmanage -m` generator, default handler via Launch Services dump), the test app bundle, `browser_probe.py` (downloads each file under each header combination and records saved name, sniffed type and warnings), `test-files.json`.

## 8. Seeds and repetitions

Deterministic probes; each probe run twice on a fresh user profile; any difference is recorded as platform nondeterminism.

## 9. Metrics

`identification_support_count` (M26.4, descriptive): number of platform paths that identify an Entrybound `.eb` file as Entrybound; binary rows per (platform path, file, candidate); count of paths that misidentify easyconfig `.eb` text files as Entrybound (and vice versa).

## 10. Analysis

Descriptive identification matrix (AGG-P per candidate); joined with EXP-ECO-012's Linux and Windows matrix for DEC-CON-021's extension decision.

## 11. Practical-significance thresholds

None (descriptive and binary rows).

## 12. Sensitivity analysis

macOS version arm (current and previous major); browser version arm (stable and extended-support release).

## 13. Expected negative results

No platform path consulting magic bytes (extension choice is then decisive on macOS and browsers); `.eba` equally unrecognized without registration.

## 14. Threats to validity

Platform behaviour changes across releases; Launch Services caches (reset per run with a fresh user); app-declared UTIs differ from system-declared ones.

## 15. Phase D held-out placeholder

Not applicable (no corpus-based selection; identification facts are platform behaviour).

## 16. Cost and effort (when unblocked)

About 4 machine hours; negligible disk; agent effort scripted plus owner-provided macOS access.
