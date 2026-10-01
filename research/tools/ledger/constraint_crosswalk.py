#!/usr/bin/env python3
"""Generate and check the R1 crosswalk for distinct decision-ledger constraints.

The classifier is a proposal, not the independent sign-off required by R1.  A
reviewer may fill the reviewer column in the generated CSV.  Regeneration keeps
that signature only while the classified string and its disposition are exact.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import re
from pathlib import Path


FIELDS = (
    "constraint_string", "string_sha256", "class", "target", "rationale",
    "classifier", "reviewer",
)
CLASSIFIER = "B01 constraint-classification session, 2026-10-01"
BARE_INVARIANT = re.compile(r"I(?:[1-9]|[12][0-9]|3[01])\Z")
INVARIANT = re.compile(r"\bI(?:[1-9]|[12][0-9]|3[01])\b")
TARGET = re.compile(r"(?:HC-(?:0[1-9]|1[0-8])|F-(?:0[1-9]|[12][0-9]|30))\Z")

# Appendix A of archetypal-objective.md (including supporting HC links).
I_TO_HC = {
    1: ("HC-01",), 2: ("HC-01",), 3: ("HC-03", "HC-17"),
    4: ("HC-08", "HC-17"), 5: ("HC-08", "HC-17"),
    6: ("HC-03", "HC-17"), 7: ("HC-10",),
    8: ("HC-10", "HC-13"), 9: ("HC-10", "HC-13", "HC-14"),
    10: ("HC-01", "HC-09"), 11: ("HC-04", "HC-07"),
    12: ("HC-04", "HC-12", "HC-14"), 13: ("HC-07",),
    14: ("HC-02", "HC-07"),
    15: ("HC-03", "HC-11", "HC-12", "HC-13", "HC-16"),
    16: ("HC-05", "HC-18"), 17: ("HC-05", "HC-06"),
    18: ("HC-07",), 19: ("HC-01", "HC-03", "HC-06"),
    20: ("HC-06",), 21: ("HC-03", "HC-04", "HC-12", "HC-15", "HC-18"),
    22: ("HC-10", "HC-16"), 23: ("HC-01", "HC-10", "HC-17"),
    24: ("HC-14",), 25: ("HC-05", "HC-14", "HC-15", "HC-18"),
    26: ("HC-03", "HC-08", "HC-17"),
    27: ("HC-08", "HC-17"), 28: ("HC-02", "HC-10", "HC-13"),
    29: ("HC-06", "HC-09"),
    30: ("HC-02", "HC-09", "HC-11", "HC-12", "HC-13", "HC-16"),
    31: ("HC-02", "HC-11"),
}

# Adversarial source-row review found labels where a broad source/keyword rule
# masks the actual normative claim.  These exact strings retain a local reason
# and take precedence over path-based classification.  Any source-text edit
# creates a new row and requires review again.
EXACT_ROUTES = {
    "SPEC §4.2 L385 hint advisory and never trusted":
        ("HC-xx", "HC-01", "An advisory hint cannot become a second semantic authority under HC-01."),
    "SPEC §4.9 L478 (uncompressed default; compact-manifest incompat)":
        ("HC-xx", "HC-04", "Default uncompressed manifest and any compact-manifest incompatibility are a fail-closed feature rule under HC-04."),
    "SPEC §4.9 L479 (explicit decoder budget)":
        ("HC-xx", "HC-12", "Explicit baseline decoder size budget supports a small independently rebuildable reader under HC-12."),
    "SPEC §4.9 compact-manifest must be an incompat feature":
        ("HC-xx", "HC-04", "A compact-manifest decoder change must fail closed as an incompatible feature under HC-04."),
    "SPEC §5.8 registries with reserved ranges":
        ("HC-xx", "HC-04;HC-16", "Closed registry admission fails unknown IDs (HC-04) and preserves assigned/reserved ID meaning (HC-16)."),
    "SPEC §7.4 range-addressable without full residency":
        ("HC-xx", "HC-06", "Range-addressable metadata must avoid unbounded full residency under HC-06."),
    "Data root outside git; only definitions, pins and fingerprints committed (PROGRESS standing decision 2026-09-12)":
        ("PROGRAM", "", "Corpus custody and repository hygiene are program controls, not candidate HC/freeze screens."),
    "Extraction policy is caller-owned (README.md L312-316; docs/platform-fidelity-v1.md)":
        ("HC-xx", "HC-05", "Caller-constructed extraction policy is HC-05, despite the platform-fidelity source path."),
    "Profile change requires new versioned ID and reviewed matrix (tools/zip-compat/README.md)":
        ("HC-xx", "HC-16", "A published profile version ID cannot change historical meaning under HC-16."),
    "SPEC §17.1 L1792 dangerous things named dangerous and never defaults":
        ("HC-xx", "HC-05", "Dangerous restoration requires explicit caller policy and cannot become a default under HC-05."),
    "SPEC §17.5 L1961 --drop-signatures required":
        ("F-xx", "F-13", "Operations invalidating independent signatures require explicit drop semantics under F-13."),
    "SPEC §17.5 L1961 --drop-signatures required for operations that invalidate signatures":
        ("F-xx", "F-13", "Explicit signature invalidation/drop semantics are registered in F-13."),
    "SPEC §19.4 object shape and three normative behaviours not deferred":
        ("HC-xx", "HC-05;HC-15", "The three behaviours are visible Sequential/RandomAccess complexity, caller-constructed archive-immutable policy (HC-05), and carried verification state (HC-15); the object shape is decision context."),
    "SPEC §21.2 legacy handles surface source format and policy; strict default":
        ("F-xx", "F-19", "Legacy observation handles expose source format and strict policy under the registered F-19 boundary."),
    "SPEC §3.3a L192 a never-appearing target is Irreconcilable":
        ("HC-xx", "HC-07;HC-17", "A missing hardlink target is declared irreconcilable loss (HC-07) and cannot create an invalid path graph (HC-17)."),
    "SPEC §6.2 L707 balanced lookback 0 by default":
        ("F-xx", "F-03", "Frozen planner profile default is in F-03; HC-09 separately bounds any declared lookback."),
    "SPEC §9.6 L1125 timestamps recommended":
        ("DECISION_INPUT", "", "Recommendation about using timestamps is not immutable signature wire or a candidate freeze."),
    "SPEC §9.6 L1125 timestamps recommended because archives outlive certificates":
        ("DECISION_INPUT", "", "Longevity rationale for recommended timestamps is design input, not an immutable wire rule."),
    "docs/legacy-export-v1.md L51-52 legacy targets never embed Entrybound signatures":
        ("F-xx", "F-22", "Legacy export representation and receipt boundary is registered in F-22, not signature wire F-13."),
    "eam/validate.rs:791-829 POSIX1E ACCESS ACL agrees with posix.mode":
        ("F-xx", "F-17", "Versioned platform metadata requires POSIX ACL/mode consistency under F-17."),
    "Exclusive create, no overwrite (docs/stream-layout-v1.md L383-385)":
        ("HC-xx", "HC-18", "Exclusive output creation is the HC-18 no-overwrite obligation, despite its stream-layout source."),
    "R1 §7.5 limits enforced in the inner loop":
        ("HC-xx", "HC-06", "Inner-loop resource limits are the HC-06 bounded-work obligation."),
    "SPEC §19.1 L2091-2095 (normative spec is Core)":
        ("HC-xx", "HC-12", "Normative Core specification supports the independent-reader HC-12 obligation."),
    "SPEC §19.4 L2152 verification state carried, never inferred":
        ("HC-xx", "HC-15", "Verification status must report only established evidence under HC-15."),
    "SPEC §19.4 complexity-visible rule (no silent O(n) scan presented as random access)":
        ("F-xx", "F-09", "The registered access/stream contract forbids hiding a scan as a seekable operation."),
    "SPEC §2.1(5) L95 every extraction records what it restored and could not":
        ("HC-xx", "HC-07", "Restoration and loss reporting is the HC-07 semantic-fidelity obligation."),
    "SPEC §5.3 long window is a declared decoder requirement":
        ("HC-xx", "HC-06", "A long decode window is admitted only as a declared bounded resource requirement."),
    "SPEC §6.0 profiles are creation-time only":
        ("F-xx", "F-05", "Creation profiles cannot become decoder requirements under registered profile freeze F-05."),
    "SPEC §6.0a L673 archives record requirements, not profile names":
        ("F-xx", "F-05", "F-05 records decoder requirements rather than a creation-profile name."),
    "SPEC §9.10 L1177 no PQ recipients mixed with classical-only recipients":
        ("F-xx", "F-12", "Hybrid-only recipient policy is fixed in the crypto-v1 suite F-12."),
    "SPEC §9.10 L1177 no PQ recipients with classical-only recipients":
        ("F-xx", "F-12", "Hybrid-only recipient policy is fixed in the crypto-v1 suite F-12."),
    "SPEC §9.10 hybrid PQ KEM mandatory from v1":
        ("F-xx", "F-12", "Mandatory hybrid KEM is a crypto-v1 suite rule in F-12."),
    "STREAM layout stays single-pass and seek-free for writers and readers (SPEC §4.1 L363)":
        ("F-xx", "F-09", "Single-pass seek-free source/sink behavior is registered by F-09."),
    "Standing constraint: platform experiments local/WSL/Docker only; macOS/APFS PLATFORM_BLOCKED":
        ("PROGRAM", "", "Experiment environment availability is program scope, not a candidate screen."),
    "V6 vector binds raw password bytes":
        ("F-xx", "F-12", "Crypto-v1 password-byte transcript is fixed by the suite and vectors in F-12."),
    "ZIP overlapping extents are Irreconcilable (legacy/zip.rs:2362-2395)":
        ("F-xx", "F-20", "ZIP modeled-divergence refusal is part of the compatibility profile F-20."),
    "codec.rs:24-25 zstandard/v1 window is 1 MiB":
        ("F-xx", "F-03", "Registered codec decoder parameters belong to the F-03 codec/planner baseline."),
    "codec.rs:41-53 allowlist of nine construction strings (-v3..-v5)":
        ("F-xx", "F-03", "Closed dictionary-construction allowlist is a registered planner/transform rule in F-03."),
    "codec.rs:41-53 closed dictionary construction allowlist":
        ("F-xx", "F-03", "Closed dictionary-construction allowlist is a registered planner/transform rule in F-03."),
    "compromised host is a non-goal (docs/crypto-threat-model-v1.md L209)":
        ("DECISION_INPUT", "", "Threat-model exclusion scopes a claim; it is not a crypto wire freeze."),
    "crypto-v1 transcripts bind ecf/bootstrap-v1 0.1 (crypto/container.rs:2236-2248)":
        ("F-xx", "F-12", "Authenticated crypto transcript namespace is fixed in the F-12 crypto wire contract."),
    "crypto-v1 wire frozen: 64 KiB token and derived 16-byte signer id":
        ("F-xx", "F-13", "Independent signature token and signer-id encoding belongs to F-13."),
    "crypto-v1 wire frozen: CONTENT binding mandatory":
        ("F-xx", "F-13", "Signature content binding is registered in F-13."),
    "crypto-v1 wire frozen: ContentBindingV1 = LAI, AUX, profile, namespace, format version":
        ("F-xx", "F-13", "ContentBindingV1 signature semantics are registered in F-13."),
    "crypto-v1 wire frozen: PhysicalBindingV1 binds only PCR":
        ("F-xx", "F-13", "PhysicalBindingV1 signature semantics are registered in F-13."),
    "crypto-v1 wire frozen: imprint over record through tag 8; token is tag 9":
        ("F-xx", "F-13", "Signature imprint and token wire fields are registered in F-13."),
    "crypto-v1 wire frozen: messageImprint SHA-256 over record through tag 8":
        ("F-xx", "F-13", "Signature imprint digest scope is registered in F-13."),
    "crypto-v1 wire frozen: signer id is derived 16 bytes":
        ("F-xx", "F-13", "Derived signer-id encoding is registered in F-13."),
    "decoders must not depend on creation profile (CONTRIBUTING.md L67-70)":
        ("F-xx", "F-05", "Creation profiles do not alter decoder semantics under F-05."),
    "docs/cross-file-compression-v1.md L52-57 frozen v3 rule (successors need new planner IDs)":
        ("F-xx", "F-03", "Cross-file planner version and successor-ID rule is registered in F-03."),
    "docs/cross-file-compression-v1.md L81-88 frozen v3+ physical order (changes need new planner IDs)":
        ("F-xx", "F-03", "Cross-file planner physical-order version is registered in F-03."),
    "docs/crypto-review-v1.md AR-14 crypto v1 INDEXED-only":
        ("F-xx", "F-12", "Crypto-v1 layout restriction is fixed by F-12."),
    "docs/crypto-review-v1.md AR-16 L454 verify stored transcript before binding status":
        ("F-xx", "F-13", "Signature binding status requires stored-transcript verification under F-13."),
    "docs/crypto-review-v1.md AR-28 L466 bounded timestamp attack surface":
        ("F-xx", "F-13", "Signature timestamp semantics and trust separation are registered in F-13."),
    "docs/crypto-review-v1.md L224-226 new algorithm gets new authenticated ID and feature":
        ("HC-xx", "HC-16", "New algorithm identity cannot reinterpret historical authenticated IDs or features."),
    "docs/crypto-review-v1.md L263 zeroize AFK, KDF output, segment/wrap keys, password buffers and expanded private keys where ownership permits":
        ("F-xx", "F-16", "Secret-buffer zeroization is registered in F-16."),
    "docs/crypto-review-v1.md Status L7-12: external cryptographic audit before production":
        ("PROGRAM", "", "Independent crypto release audit is a program gate, not a candidate HC screen."),
    "docs/crypto-threat-model-v1.md L105-107 removing a recipient rotates the file key and re-encrypts":
        ("F-xx", "F-14", "Recipient removal must rotate and re-encrypt under F-14."),
    "docs/format-v0.md 0x4000 requires 0x2000":
        ("F-xx", "F-20", "Versioned legacy preservation feature dependency is registered in F-20."),
    "docs/jpeg-reconstruction-v1.md L50-54 one physical representation per exact plaintext Chunk":
        ("F-xx", "F-03", "Reconstructive transform representation rule is in F-03."),
    "docs/stream-layout-v1.md L371-374 verified-before-write staging":
        ("F-xx", "F-11", "Verified staging before output release is registered in F-11."),
    "eam/validate.rs:922-995 contiguous groups with exact max_preceding_bytes":
        ("HC-xx", "HC-09", "Contiguous lookback groups and declared bound support HC-09 self-containment."),
    "ecf/container.rs:1856-1858 one frame per chunk digest":
        ("HC-xx", "HC-01", "One frame per declared chunk digest preserves single data authority under HC-01."),
    "model decision determinism-guarantee-scope (output must not depend on unrecorded ambient state)":
        ("HC-xx", "HC-11;HC-13", "No ambient decode interpretation (HC-11) or encode determinism variation (HC-13)."),
    "preamble bound by footer SHA-256 and feature digest":
        ("HC-xx", "HC-16", "Preamble/footer binding and feature identity cannot be reinterpreted historically."),
    "preamble offsets 96-160 hold exactly eight u64":
        ("HC-xx", "HC-16", "Published preamble field layout is immutable under HC-16."),
    "production semantics unchanged by experiments (research program standing decision)":
        ("PROGRAM", "", "Research isolation is a program gate rather than a candidate screen."),
    "registry names 4 posix.uid and 5 posix.gid":
        ("F-xx", "F-17", "Registered POSIX ownership metadata names belong to F-17."),
    "unknown TLV fields rejected (docs/format-v0.md §Decisions L24-29)":
        ("HC-xx", "HC-04", "Unknown format fields fail closed under HC-04."),
    "windows.security-descriptor exact self-relative bytes (type 42)":
        ("F-xx", "F-17", "Windows security-descriptor representation is registered in F-17."),
}


def invariant_hcs(value: str) -> tuple[str, ...]:
    return tuple(sorted({hc for token in INVARIANT.findall(value)
                         for hc in I_TO_HC[int(token[1:])]}))


def disposition(kind: str, target: str, rationale: str) -> tuple[str, str, str]:
    return kind, target, rationale


def freeze_target(s: str) -> str | None:
    """Choose only an existing Appendix B freeze whose subject is explicit."""
    rules = (
        (r"native tooling|no wire records or feature bits|inspection-v1|archive-diff-v1", "F-28"),
        (r"random.access.v1.md L3-4|random access v1 adds no wire", "F-29"),
        (r"unsafe_code|new dependencies require", "F-23"),
        (r"research features|production semantics unchanged by experiments", "F-24"),
        (r"never reused|retired identifiers|retired features|x- names", "F-26"),
        (r"(?:exact|raw) (?:file)?name bytes|(?:name|path).*never normali[sz]|no normali[sz]ation.*(?:name|path)", "F-27"),
        (r"recipient mutation|remov.*rotat.*AFK|add.*preserv.*AFK|archive_id.*re.encrypt", "F-14"),
        (r"descriptor.vectors|descriptor v[12]|descriptor ambiguity|descriptor.*tags", "F-15"),
        (r"secret hygiene|no secrets on|passwords? never|passwords? not accepted as command.line|no `Debug`", "F-16"),
        (r"signature|signing.key|\.ebsig|timestamp.*trust anchor|binding.*stale", "F-13"),
        (r"crypto|HKDF|X.Wing|Argon2|AEAD|recipient|AFK|segment nonce|encrypted preamble", "F-12"),
        (r"stream.layout|STREAM_BODY|STREAM MANIFEST_RECORD|stream ordering|stream frame|STREAM layout", "F-06"),
        (r"layout equivalence|INDEXED and STREAM agree", "F-07"),
        (r"http.range|strong ETag|Content.Range|no 200 whole.body|random.access.v1|verified partial", "F-10"),
        (r"Write.only sink|Read.only source|single.pass and seek.free|streaming.*no whole", "F-09"),
        (r"verified.before.write|verified staging|no plaintext.*before.*verif", "F-11"),
        (r"legacy.export|compressed.tar.export|migration.workflows|zip.export|ExportReceipt|export profiles|migration.report", "F-22"),
        (r"zip.compatib|zip.import|ZipObservation|modeled.divergence matrix|modeled.divergence|zip.rs", "F-20"),
        (r"tar.import|7z.import|compressed.stream.import|solid folder|wrapper observer", "F-21"),
        (r"legacy.preservation", "F-20"),
        (r"legacy.observation|ConversionProvenance|LOM contract|EAM projection", "F-19"),
        (r"platform.fidelity|filesystem.fidelity|posix.metadata|security.metadata|NFS4|ReparsePoint|ACL_V1|SparseMapV1|hardlink group", "F-17"),
        (r"repack identity|representation.only repack|repack preserves|explain never reruns|identity tiers", "F-08"),
        (r"gear.norm|chunker|chunking.v1|dedup equality|cross.file.compression", "F-04"),
        (r"planner|codec|transform|dictionary construction|similarity.rs|preflate|jixel|jxl.oxide|zstd window|lookback.*registered", "F-03"),
        (r"single.authority framing|stored_length.*sole|no second length", "F-01"),
        (r"ChunkGroup membership|group_ref.*member list", "F-02"),
    )
    for pattern, freeze in rules:
        if re.search(pattern, s, re.I):
            return freeze
    return None


def classify(value: str) -> tuple[str, str, str]:
    """Classify a ledger label; the CSV retains text for independent review."""
    s = value.strip()
    low = s.lower()
    if not s or s != value:
        raise ValueError(f"constraint text has surrounding whitespace: {value!r}")
    if s in EXACT_ROUTES:
        return EXACT_ROUTES[s]
    if s == "SPEC §20.7 no in-archive decompressor VM":
        return disposition("F-xx", "F-30", "SPEC §20.7 rejection is registered as objective Appendix B F-30.")

    # A range statement cites the program-wide invariant set, not 31 separate
    # per-decision screens. Individual invariant labels are handled below.
    if re.search(r"I1.I31 screen|all invariants|invariants not deferred", s, re.I):
        return disposition("PROGRAM", "", "Program-wide invariant governance, not an individual candidate screen.")

    if s.startswith("SPEC ") and re.search(r"§24(?:\b|\s)|\ufffd24(?:\b|\s)", s):
        return disposition("NON_GOAL", "F-25", "SPEC §24 non-goal; enforce through registered F-25 only.")
    if re.search(r"SPEC §22\.5.*(?:ECC|error correction)", s, re.I):
        return disposition("NON_GOAL", "F-25", "SPEC §24/F-25 refusal of in-archive error correction, repeated in §22.5.")
    if re.search(r"^SPEC [§\ufffd](?:17\.7|22\.[45]).*(?:append.in.place|no append|versioned log)", s, re.I):
        return disposition("NON_GOAL", "F-25", "SPEC §24 items 12–13 defer append-in-place and versioned journaling beyond v1.")
    if re.search(r"^SPEC [§\ufffd]22\.5.*self.extracting", s, re.I):
        return disposition("HC-xx", "HC-03", "SPEC §11.4 canonical no-prefix/no-polyglot rule excludes self-extracting archives.")

    # This captures a stated invariant even when followed by explanatory text.
    if re.match(r"I(?:[1-9]|[12][0-9]|3[01])\b", s):
        hcs = invariant_hcs(s)
        return disposition("HC-xx", ";".join(hcs), "Explicit SPEC invariant mapped by objective Appendix A (primary and supporting HCs).")

    # Governance and design handoffs are not R1 per-decision screens.
    if re.search(r"cluster decisions?|(?:model|access|integrity) decision|sibling shard|cross.ref.*decision", s, re.I):
        return disposition("DECISION_INPUT", "", "Cross-decision ownership or scope handoff; no independent R1 screen.")
    if re.search(r"^(?:PROGRESS|Program |program |Research |research/|R[12] [§\ufffd]|R[12] byte|Task \d+|SPEC authority|SPEC Header|SPEC and Research|SPEC §25\.2|SPEC \ufffd25\.2)", s, re.I) or re.search(r"owner approval|no outreach|no GitHub Actions|hosted runners|held.out.*(?:freeze|unread)|pre.registration gate|decision.relevant results|thresholds from research/decision.method|thresholds.py|ebr refuses|Corpus redistribution|per.item licen[cs]e|Simulated evaluations|sch_netem unavailable|source.inventory|merge.contract evidence|Research I|Research II|Research III", s, re.I):
        return disposition("PROGRAM", "", "Program/research governance, evidence policy, or cross-decision handoff; not a candidate HC/freeze screen.")

    if re.search(r"docs/crypto-threat-model-v1\.md.*(?:non.goals|deliberately public|freshness not claimed|recipient adversary|keys authenticated by caller|compromised host)", s, re.I):
        return disposition("DECISION_INPUT", "", "Threat-model scope or deliberately excluded claim; no exact Appendix B freeze is cited here.")
    if re.search(r"^SPEC [§\ufffd]7\.6 .*I24 is a design objective", s):
        return disposition("DECISION_INPUT", "", "SPEC distinguishes an aspiration from a delivered privacy proof; HC-14 retains the mechanical floor.")
    if re.search(r"^SPEC [§\ufffd]9\.11 .*inspect reports whether parameters meet current guidance", s):
        return disposition("DECISION_INPUT", "", "Current-guidance reporting is a UI/design input, not a fixed crypto-v1 wire parameter.")
    if re.search(r"^docs/crypto-suite-v1\.md.*(?:Entrybound defines no PKI)|^docs/crypto-implementation-v1\.md.*no online TSA", s, re.I):
        return disposition("NON_GOAL", "F-25", "SPEC §24 excludes key/signing infrastructure and requires offline verification.")
    if re.search(r"^CRYPTO-020 no secret.dependent public diagnostics|^docs/crypto-implementation-v1\.md.*no Debug or Display|^docs/crypto-suite-v1\.md.*zeroize", s, re.I):
        return disposition("F-xx", "F-16", "Secret handling and diagnostics are registered in F-16.")
    if re.search(r"^docs/crypto-suite-v1\.md.*caller trust anchors and separate timestamp status", s, re.I):
        return disposition("F-xx", "F-13", "Caller trust and timestamp separation are registered in F-13.")
    if re.search(r"^docs/crypto-threat-model-v1\.md.*valid tag never waives resource validation", s, re.I):
        return disposition("HC-xx", "HC-06", "Authenticated data is still subject to the exact HC-06 resource budget.")
    if re.search(r"^future features cannot reinterpret assigned values", s, re.I):
        return disposition("HC-xx", "HC-16", "Assigned values and historical meaning are immutable under HC-16.")
    if re.search(r"^readers never synthesize declarations", s, re.I):
        return disposition("HC-xx", "HC-01", "A reader cannot invent a second semantic authority under HC-01.")
    if re.search(r"^docs/crypto-review-v1\.md.*full 32.byte HMAC tags", s, re.I):
        return disposition("F-xx", "F-12", "Full-length crypto-v1 tag encoding is part of F-12, not truncation diagnostics.")
    if re.search(r"^SPEC [§\ufffd]6\.0a max_decode_window recorded", s):
        return disposition("HC-xx", "HC-06", "Decoder requirement must be declared and bounded under HC-06.")
    if re.search(r"docs/crypto-wire-v1\.md.*(?:vectors before production|required.*vectors before|external primitive vectors)", s, re.I):
        return disposition("PROGRAM", "", "Production vector qualification gate, not a candidate design screen.")
    if re.search(r"docs/crypto-suite-v1\.md.*(?:strong passwords essential|crate version alone insufficient)", s, re.I):
        return disposition("DECISION_INPUT", "", "Security guidance/evidence quality, not an immutable suite byte or per-candidate HC test.")
    if re.search(r"docs/crypto-suite-v1\.md.*AES.GCM.SIV not assumed key.committing", s, re.I):
        return disposition("HC-xx", "HC-14", "Key-commitment obligation is tested under HC-14; this is rationale against a particular assumption.")
    if re.search(r"SPEC [§\ufffd]14\.2 Refinement severity \(superseded", s):
        return disposition("DECISION_INPUT", "", "Superseded historical classification is context, not a live candidate screen.")
    if re.search(r"^SPEC [§\ufffd](?:22\.[12]|23\.1|23\.5|25\.3|26\.3)", s, re.I):
        return disposition("PROGRAM", "", "SPEC evidence, comparison, research, or feature-governance rule; not an individual R1 design screen.")
    if re.search(r"^SPEC [§\ufffd]25\.(?:1.*(?:security review|row 10|row 12|row 9)|4)", s, re.I):
        return disposition("PROGRAM", "", "SPEC design-review or release-timing obligation, not an individual R1 screen.")
    if re.search(r"^SPEC [§\ufffd]25\.1 (?:#1 |row 2 )", s):
        target = "F-09" if "#1 " in s else "HC-06"
        category = "F-xx" if target.startswith("F-") else "HC-xx"
        return disposition(category, target, f"SPEC §25.1 fixed constraint is covered by registered {target}.")
    if re.search(r"^SPEC [§\ufffd]25\.1 row 13", s):
        return disposition("F-xx", "F-12", "SPEC §25.1 row 13 fixes a declared bucketed crypto padding requirement represented by F-12.")
    if re.search(r"^SPEC [§\ufffd]26\.2 row 3 .*hybrid PQ KEM mandated", s):
        return disposition("F-xx", "F-12", "SPEC §9.10 mandatory hybrid recipient rule is registered in F-12.")
    if re.search(r"^SPEC [§\ufffd]26\.5 row 1 ", s):
        return disposition("F-xx", "F-13", "SPEC §9.6 independent signature binding semantics are registered as F-13.")
    if re.search(r"^SPEC [§\ufffd]11\.2 L1368", s):
        return disposition("HC-xx", "HC-05", "SPEC §11.2 setuid/setgid restoration is policy-gated; HC-05 screens policy expansion.")
    if re.search(r"^SPEC [§\ufffd]11\.2 file.versus.directory", s):
        return disposition("HC-xx", "HC-17", "SPEC §11.2 file/directory ancestor hazard is structurally excluded by HC-17.")
    if re.search(r"^SPEC [§\ufffd]10\.7 .*decmpfs never synthesised", s):
        return disposition("HC-xx", "HC-07", "Never synthesizing unobserved metadata is part of HC-07 semantic fidelity.")
    if re.search(r"^SPEC [§\ufffd]17\.3 .*no signatures reports", s):
        return disposition("HC-xx", "HC-15", "Reporting no signatures as verified would violate HC-15 verification honesty.")
    if re.search(r"^SPEC [§\ufffd]5\.2 .*categorise by bytes|^SPEC [§\ufffd]5\.2 .*recognise by bytes", s):
        return disposition("F-xx", "F-03", "Planner content classification is pinned within the registered planner contract F-03.")
    if re.search(r"^Adapter boundary: ZIP compression never crosses", s):
        return disposition("F-xx", "F-20", "ZIP compatibility projection stays outside native planning under F-20.")
    if re.search(r"^Adapter boundary: foreign structure never enters EAM", s):
        return disposition("F-xx", "F-19", "F-19 requires foreign observations to pass through LOM before EAM projection.")
    if re.search(r"^CollisionPolicy(?:::\w+)? (?:refuse default|only)", s):
        return disposition("HC-xx", "HC-08", "Hostile-name collision handling is screened by HC-08.")
    if re.search(r"^Features that cannot compose must be refused", s):
        return disposition("HC-xx", "HC-04;HC-07", "Unsupported composition fails closed (HC-04) and cannot silently drop semantics (HC-07).")
    if re.search(r"^INDEX is the only ignorable section", s):
        return disposition("HC-xx", "HC-04", "Only explicitly optional cache data may be ignored under HC-04.")
    if re.search(r"^docs/crypto-review-v1\.md L407-408 private Descriptor", s):
        return disposition("HC-xx", "HC-06", "Unauthenticated private data cannot authorize pre-unlock resource work under HC-06.")
    if re.search(r"^docs/repack-v1\.md L42: never silently decrypt", s):
        return disposition("F-xx", "F-08", "Repack identity/representation contract is registered in F-08.")
    if re.search(r"^historical records never reinterpreted|^identity profiles frozen once published|^preamble offsets 76-96 frozen", s, re.I):
        return disposition("HC-xx", "HC-16", "Published record/profile/field meaning remains immutable under HC-16.")
    if re.search(r"^never overwrite an existing destination", s):
        return disposition("HC-xx", "HC-18", "HC-18 requires atomic output without overwriting existing destinations.")
    if re.search(r"^pax vendor keywords must be VENDOR\.keyword", s):
        return disposition("F-xx", "F-21", "Tar import dialect semantics are registered under F-21.")
    if re.search(r"^stdin is read by the STREAM reader", s):
        return disposition("F-xx", "F-09", "Sequential read-only source boundary is registered in F-09.")
    if re.search(r"^x- names never participate", s):
        return disposition("HC-xx", "HC-10", "Writer-defined extension names cannot alter canonical LAI under HC-10.")
    if re.search(r"docs/crypto-review-v1\.md.*(?:review record|audit required|acceptance plan|release gate|fuzzing required)", s, re.I):
        return disposition("PROGRAM", "", "Independent crypto review or release evidence obligation, not a candidate screen.")
    if re.search(r"SPEC [§\ufffd]5\.7a", s):
        return disposition("HC-xx", "HC-07", "Consistent source capture is explicitly part of HC-07's mechanical oracle.")
    if re.search(r"SPEC [§\ufffd]6\.2 balanced is default", s):
        return disposition("DECISION_INPUT", "", "Current default is expressly open to evidence; no immutable identifier behavior is asserted.")
    if re.search(r"Cargo\.toml:8 rust-version|SPEC [§\ufffd]6\.0a compress_budget", s):
        return disposition("DECISION_INPUT", "", "Toolchain or profile design parameter, not a candidate HC/F screen.")

    if re.match(r"CRYPTO-\d+", s):
        target = freeze_target(s)
        return disposition("F-xx", target or "F-12", f"Existing crypto contract represented by objective Appendix B {target or 'F-12'}.")
    if re.search(r"Cargo\.toml.*unsafe_code|workspace unsafe_code|unsafe_code forbidden", s, re.I):
        return disposition("F-xx", "F-23", "Workspace unsafe-code and dependency discipline is registered as F-23.")

    if re.search(r"(?:frozen|freeze|byte.frozen|current freeze|versioned.*identif|never reused)", s, re.I):
        target = freeze_target(s)
        if target:
            return disposition("F-xx", target, f"Explicit fixed behavior within objective Appendix B {target}.")
        if re.search(r"historical|format constant|record|feature|registry|\bID\b|wire", s, re.I):
            return disposition("HC-xx", "HC-16", "Historical format/identifier meaning cannot be reinterpreted (HC-16); no narrower registered F matches this label.")

    # Source-specific freezes without the literal word 'frozen'.
    source_freeze = (
        r"docs/(?:crypto-suite|crypto-wire|crypto-implementation|stream-layout|random-access|http-range-access|"
        r"posix-metadata|security-metadata|platform-fidelity|filesystem-fidelity|"
        r"legacy-export|migration-workflows|compressed-tar-export|zip-export|"
        r"zip-import|zip-compatibility-profiles|tar-import|7z-import|compressed-stream-import|"
        r"legacy-observation-model|legacy-preservation|planner-v1|chunking-v1|"
        r"reconstructive-transform|jpeg-reconstruction|codec-transform|cross-file-compression|inspection-explanation|"
        r"signing-key-management|repack|archive-diff)(?:-v\d+)?\.md"
    )
    if re.search(source_freeze, s, re.I):
        target = freeze_target(s)
        if target:
            return disposition("F-xx", target, f"Existing normative implementation contract represented by objective Appendix B {target}.")

    if re.search(r"docs/security.metadata.v1.vectors\.txt|ACL_V1_POSIX_ACCESS vector|NFS4 registry", s, re.I):
        return disposition("F-xx", "F-17", "Versioned platform metadata contract in the Appendix B F-17 register.")

    # Public SPEC section citations are routed only where the governing HC or
    # registered freeze covers the cited subject.  Unmatched design scope remains
    # DECISION_INPUT below; it is not silently elevated into an R1 screen.
    section = re.search(r"^SPEC [§\ufffd](\d+)(?:\.(\d+))?", s)
    if section:
        major, minor = section.group(1), section.group(2) or ""
        if major == "9":
            target = "F-13" if minor in {"6", "10"} or "signature" in low else "F-12"
            return disposition("F-xx", target, f"Crypto or signature semantics already registered in objective Appendix B {target}.")
        if major == "14" and "tier" not in low and "rar5" not in low:
            target = "F-20" if minor == "3" else "F-21" if minor == "4" else "F-19"
            return disposition("F-xx", target, f"Legacy import boundary registered in objective Appendix B {target}.")
        if major == "15":
            return disposition("F-xx", "F-22", "Legacy export and migration workflow contract registered as F-22.")
        if major == "16":
            target = "F-13" if "signature" in low else "F-19"
            return disposition("F-xx", target, f"Canonical provenance or signature binding contract registered as {target}.")
        if major == "20":
            if minor in {"1", "2"}:
                return disposition("HC-xx", "HC-04", "Unknown-feature and optional-data behavior is the HC-04 fail-closed screen.")
            if minor == "3" and re.search(r"(?:identifier|feature bit|planner ID|registered|registry)", s, re.I):
                return disposition("F-xx", "F-26", "Identifier and registry permanence is registered as F-26.")
            if minor == "4" and re.search(r"(?:identif|retir|DEPRECATED|revision)", s, re.I):
                return disposition("F-xx", "F-26", "Retired identifier behavior is registered as F-26.")
            if minor == "5":
                return disposition("HC-xx", "HC-15", "Outcome and verification-state honesty is tested by HC-15.")
            if minor == "6":
                return disposition("HC-xx", "HC-12", "Independent decoder and normative specification obligation is HC-12.")
        section_map = {
            ("2", "1"): ("HC-xx", "HC-01"),
            ("3", "3"): ("HC-xx", "HC-10"),
            ("3", "4"): ("HC-xx", "HC-08"),
            ("3", "5"): ("HC-xx", "HC-02"),
            ("4", "2"): ("HC-xx", "HC-04"),
            ("4", "3"): ("HC-xx", "HC-04"),
            ("4", "6"): ("F-xx", "F-09"),
            ("4", "7"): ("F-xx", "F-12"),
            ("4", "8"): ("HC-xx", "HC-06"),
            ("4", "9"): ("HC-xx", "HC-09"),
            ("5", "5"): ("HC-xx", "HC-02"),
            ("5", "7"): ("F-xx", "F-03"),
            ("5", "8"): ("HC-xx", "HC-12"),
            ("6", "4"): ("HC-xx", "HC-06"),
            ("7", "2"): ("F-xx", "F-04"),
            ("7", "3"): ("F-xx", "F-04"),
            ("7", "4"): ("HC-xx", "HC-09"),
            ("7", "5"): ("F-xx", "F-03"),
            ("7", "6"): ("F-xx", "F-12"),
            ("8", "0"): ("HC-xx", "HC-10"),
            ("8", "3"): ("HC-xx", "HC-10"),
            ("8", "5"): ("HC-xx", "HC-10"),
            ("8", "6"): ("HC-xx", "HC-10"),
            ("8", "8"): ("HC-xx", "HC-10"),
            ("11", "4"): ("HC-xx", "HC-03"),
            ("11", "5"): ("HC-xx", "HC-06"),
            ("11", "6"): ("HC-xx", "HC-05"),
            ("11", "7"): ("HC-xx", "HC-05"),
            ("12", "1"): ("HC-xx", "HC-13"),
            ("12", "2"): ("HC-xx", "HC-13"),
            ("13", "3"): ("HC-xx", "HC-09"),
            ("19", "3"): ("F-xx", "F-18"),
            ("19", "4"): ("HC-xx", "HC-05"),
        }
        if (major, minor) in section_map:
            category, target = section_map[(major, minor)]
            return disposition(category, target, f"Public SPEC §{major}.{minor} maps to registered {target}; exact claim remains subject to independent review.")

    # HC restatements take precedence over broad contextual references.
    hc_rules = (
        (r"(?:one|single|sole|exactly one) (?:semantic )?authority|declar(?:ed|ation).*once|duplicat.*(?:header|authority)", "HC-01"),
        (r"lossless|byte.exact|round.trip|no lossy|exact source bytes", "HC-02"),
        (r"canonical (?:entry|serial|form|TLV)|unique interpretation|one interpretation|strict pars|noncanonical|ambiguity", "HC-03"),
        (r"unknown (?:critical|codec|feature|record|protection)|reserved (?:bit|field)|fail.closed|closed (?:core|metadata|registry)", "HC-04"),
        (r"caller.owned policy|extraction policy|confinement|no.follow|symlink escape|safe refuses|never.*outside.*root", "HC-05"),
        (r"budget|pre.allocat|pre.work|bounded memory|bounded.*(?:scratch|KDF|decoder|resource|repeated)|unbounded|resource safety|limits enforced", "HC-06"),
        (r"fidelity|no silent|loss declared|omission.*record|unrepresentable|unavailable classes|never invent.*mode", "HC-07"),
        (r"hostile name|hostile bytes|normalis|normali[sz]|case.fold|NFC|NFD|target.hostile|encoding.*path", "HC-08"),
        (r"self.contained|no external dependency|lookback|dependency closure|forward reference|independently decodable", "HC-09"),
        (r"\bLAI\b|\bAUX\b|\bPCR\b|identity root|identity digest|Merkle|representation.only repack", "HC-10"),
        (r"decoder.*(?:profile|planner|environment)|interpretation.*deterministic|decode behavior|normative specification over implementation", "HC-11"),
        (r"independent (?:reader|implement)|conformance corpus|test vectors|normative format spec|specification and vectors", "HC-12"),
        (r"deterministic|reproducib|independent of thread|no environment depend|integer arithmetic", "HC-13"),
        (r"authenticated encrypt|cryptographic|nonce|key commitment|no plaintext release|HYBRID_ONLY|post.quantum|sign.*binding|downgrade", "HC-14"),
        (r"no overwrite|exclusive.create|safe remote origin|crash.consisten", "HC-18"),
        (r"unverified|unchecked|truncat|corrupt|reason code|outcome class|salvage", "HC-15"),
        (r"historical archives|identifiers? never reused|retired identifiers?|feature bits never", "HC-16"),
        (r"logical.?path|duplicate path|file.as.ancestor|proper prefix|source symlink|path traversal|P[1-8]", "HC-17"),
    )
    for pattern, hc in hc_rules:
        if re.search(pattern, s, re.I):
            return disposition("HC-xx", hc, f"Restates the objective {hc} obligation; candidate screen is its MVT, not this wording alone.")

    # A freeze can be identified from a descriptive label as well as a path.
    target = freeze_target(s)
    if target and re.search(r"(?:wire|version|registry|format|profile|policy|refuse|strict|model|record|decode|reader|writer)", s, re.I):
        return disposition("F-xx", target, f"Fixed product contract under objective Appendix B {target}.")

    return disposition("DECISION_INPUT", "", "Scope, status-quo fact, or design input with no exact registered HC/F screen in this label.")


def source_rows(path: Path) -> list[str]:
    strings: set[str] = set()
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            if not line.strip():
                continue
            row = json.loads(line)
            for value in row["hard_constraints"]:
                if not BARE_INVARIANT.fullmatch(value):
                    strings.add(value)
    return sorted(strings)


def render(rows: list[dict[str, str]]) -> str:
    out = io.StringIO(newline="")
    writer = csv.DictWriter(out, fieldnames=FIELDS, lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)
    return out.getvalue()


def carried_reviewer(old: dict[str, str] | None, row: dict[str, str]) -> str:
    """A review signature is bound to the exact generated disposition."""
    if old and all(old.get(key) == row[key] for key in FIELDS if key != "reviewer"):
        return old.get("reviewer", "")
    return ""


def build(ledger: Path, prior: Path | None = None) -> list[dict[str, str]]:
    previous: dict[str, dict[str, str]] = {}
    if prior and prior.exists():
        with prior.open(encoding="utf-8", newline="") as handle:
            previous = {r["constraint_string"]: r for r in csv.DictReader(handle)}
    rows = []
    for value in source_rows(ledger):
        category, target, rationale = classify(value)
        digest = hashlib.sha256(value.encode("utf-8")).hexdigest()
        row = dict(zip(FIELDS, (value, digest, category, target, rationale, CLASSIFIER, "")))
        row["reviewer"] = carried_reviewer(previous.get(value), row)
        rows.append(row)
    return rows


def validate(rows: list[dict[str, str]]) -> None:
    seen = set()
    for row in rows:
        value = row["constraint_string"]
        if value in seen:
            raise ValueError(f"duplicate constraint: {value}")
        seen.add(value)
        if row["string_sha256"] != hashlib.sha256(value.encode("utf-8")).hexdigest():
            raise ValueError(f"incorrect string digest: {value}")
        category = row["class"]
        if category in ("HC-xx", "F-xx"):
            targets = row["target"].split(";")
            if not targets or any(not TARGET.fullmatch(t) for t in targets):
                raise ValueError(f"invalid target: {value}")
            if any(not t.startswith(category[:2]) for t in targets):
                raise ValueError(f"target/class mismatch: {value}")
        elif category not in ("NON_GOAL", "PROGRAM", "DECISION_INPUT"):
            raise ValueError(f"invalid class: {category}")
        elif category == "NON_GOAL" and row["target"] != "F-25":
            raise ValueError(f"SPEC non-goal must cite F-25: {value}")
        elif category in ("PROGRAM", "DECISION_INPUT") and row["target"]:
            raise ValueError(f"nonscreening class has a target: {value}")
        if not row["rationale"] or not row["classifier"]:
            raise ValueError(f"missing audit attribution: {value}")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--ledger", type=Path, default=Path("research/decision-ledger.jsonl"))
    ap.add_argument("--output", type=Path, default=Path("research/methods/constraint-crosswalk.csv"))
    mode = ap.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = ap.parse_args()
    rows = build(args.ledger, args.output)
    validate(rows)
    expected = render(rows)
    if args.write:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(expected, encoding="utf-8", newline="")
        print(f"wrote {len(rows)} crosswalk rows to {args.output}")
        return 0
    actual = args.output.read_text(encoding="utf-8") if args.output.exists() else ""
    if actual != expected:
        print(f"crosswalk stale: {args.output}")
        return 1
    print(f"crosswalk exact: {len(rows)} rows")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
