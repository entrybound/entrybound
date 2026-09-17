# WIP (not wired into make_sources.py): F20 structurally-adversarial-archives gap closure.
#
# Status as of 2026-09-16 (interrupted): drafted and desk-checked, NOT YET run through
# provision.py --check or provisioned/fingerprinted. The four new generator scripts this block
# references already exist, committed, in the parent directory:
#   decode_uu_files.py         -- generic *.uu decoder run-step (binascii.a2b_uu, no `uu` module)
#   malicious_archives.py      -- tuning/validation: zip/tar traversal, symlink/hardlink escape,
#                                  duplicate names, CD/LFH mismatch. SMOKE-TESTED standalone (ran
#                                  it directly with --out/--seed/--params, then parsed the outputs
#                                  with zipfile/tarfile and a raw byte read of the first local
#                                  header, which confirmed the CD/LFH name actually disagrees:
#                                  central directory reports "innocuous.txt", the local header
#                                  bytes say "smuggled.txt"). Not yet run *through* provision.py.
#   malicious_archives_v2.py   -- held-out: reserved device names, symlink loop, zip64 duplicate
#                                  names, PAX path-override traversal, malformed 7z/RAR4 headers.
#                                  Traced by hand against malicious_archives.py's already-verified
#                                  struct layout; NOT independently executed.
#   fifield_zipbomb_regen.py   -- regenerates (never vendors) David Fifield's public-domain
#                                  overlapping-entry zip bombs. The underlying zipbomb tool and
#                                  exact CLI invocations used here WERE manually verified: e.g.
#                                  `python3 zipbomb --mode=quoted_overlap --num-files=250
#                                  --compressed-size=21179` produced a 42,374-byte zip whose
#                                  central directory claims 250 entries totalling 5,461,307,620
#                                  claimed uncompressed bytes. The wrapper script itself (which
#                                  extracts the tool from the declared zip input and writes to
#                                  EB_OUT) was not run through provision.py.
#
# Why this stalled: the WSL2 VM used for provision.py/make_sources.py became unresponsive
# (vmmemWSL working set pegged at ~33 GiB and static across many repeated checks spanning most of
# an hour; host free RAM ~8 GiB of 64 GiB; even `wsl.exe -d Ubuntu -- bash -lc "echo alive"` never
# returned within 120 s across five separate attempts). This is systemic host/VM resource exhaustion
# from many concurrent Phase A/B1 agents sharing the same WSL instance (see PROGRESS.md), not
# specific to this item -- it stopped ALL WSL work, not just this one. It may need a host-side WSL
# restart (`wsl --shutdown` then relaunch) rather than more waiting; that is outside a
# corpus-provisioning agent's own permissions.
#
# To resume once WSL responds again:
#   1. Open research/corpus/generators/g2-generated/make_sources.py and find the comment
#      "# NOTE: an F20 structurally-adversarial-archives addition ... was drafted and then held
#      back" (right before `DOC_NOTES = (`). Replace that NOTE comment with the `ITEMS += [...]`
#      block below (delete the NOTE, paste this block in its place, keep DOC_NOTES after it).
#   2. cd research/corpus/generators/g2-generated && python3 make_sources.py
#   3. cd ../../../.. && bash research/corpus/tools/run.sh provision --check
#      -- fix any schema errors (git subpaths syntax, script paths, etc.) before provisioning.
#   4. bash research/corpus/tools/run.sh provision --family F20 --jobs 3
#      (or --item <id> one at a time while debugging; the four real/git-archive items each do a
#      real network fetch -- libarchive subset, CPython derive [no network], commons-compress
#      subset, Go derive [no network], fastzip/malo [tiny, ~229 KiB repo], Fifield zip [18,130 B])
#   5. Check each new item's scale_check in research/corpus/fingerprints/<id>.json; if any
#      "fits_smaller_tier": true appears (as happened for f04-tuning-real-netbsd-pkgsrc and
#      f04-validation-real-gentoo-repo-20260915 in this same session -- both declared "large" but
#      materialized under 512 MiB), edit that item's scale in make_sources.py to match, re-run
#      make_sources.py + provision for that item, and adjust this file's own comments/description
#      text if it mentions the old tier.
#   6. Once all 8 items provision cleanly, delete this wip/ file (or leave it; it is inert once its
#      content has been merged into make_sources.py) and checkpoint:
#        research/corpus/sources/g2-generated.json
#        research/corpus/generators/g2-generated/make_sources.py
#        research/corpus/generators/g2-generated/decode_uu_files.py
#        research/corpus/generators/g2-generated/malicious_archives.py
#        research/corpus/generators/g2-generated/malicious_archives_v2.py
#        research/corpus/generators/g2-generated/fifield_zipbomb_regen.py
#        research/corpus/fingerprints/f20-tuning-real-libarchive-testsuite.json
#        research/corpus/fingerprints/f20-validation-real-cpython-archivetestdata.json
#        research/corpus/fingerprints/f20-validation-real-commons-compress-testdata.json
#        research/corpus/fingerprints/f20-heldout-real-go-archive-testdata.json
#        research/corpus/fingerprints/f20-heldout-real-fastzip-malo.json
#        research/corpus/fingerprints/f20-heldout-real-fifield-zipbomb-regen.json
#        research/corpus/fingerprints/f20-tuning-generated-malicious-structures.json
#        research/corpus/fingerprints/f20-validation-generated-malicious-structures.json
#        research/corpus/fingerprints/f20-heldout-generated-malicious-structures-v2.json
#        research/corpus/pins
#      and this wip/ file's deletion (git rm), in one checkpoint.py call.
#
# --- the ITEMS block to paste back into make_sources.py (needs `item`, `gen`, `derive`,
# `copy_whole`, `lic`, `GENERATED_LIC`, `GEN` already in scope there, as they are) ---

# Corpus round-1 critic gap (F20 structurally-adversarial archives, MAJOR): all 10 F20 items above
# are in-house entropy/CDC/bomb generators; there was no coverage of malicious archive *structure*
# (traversal, symlink/hardlink escape, duplicate names, CD/LFH mismatch, malformed headers) and no
# third-party fuzz/conformance-test archives. Real fixtures from four upstream test suites, plus a
# regenerated (not vendored) public-domain zip bomb, plus two structurally distinct generated sets
# (one for tuning/validation, a different one for held-out) close the gap.
ITEMS += [
    item("f20-tuning-real-libarchive-testsuite", "F20", "tuning", "medium", "git-archive", "real",
         {"git": {"repo": "https://github.com/libarchive/libarchive.git",
                 "commit": "5ddc1d9822a390424ce5bd42f35e58acc9e88df4", "ref": "master",
                 "subpaths": ["libarchive/test", "tar/test", "cpio/test", "unzip/test"]},
          "steps": [{"op": "extract", "input": "git-archive", "format": "tar"},
                    {"op": "run", "script": f"{GEN}/decode_uu_files.py", "interpreter": "python",
                     "seed": 0, "params": {}}]},
         lic("BSD-2-Clause per COPYING", True, "Tim Kientzle and libarchive contributors",
             "GitHub reports NOASSERTION for the repository as a whole; COPYING states the large majority of "
             "files (including this test suite) are BSD-2-Clause, but this has not been independently audited "
             "file-by-file, so treat per-file license as unconfirmed until audited."),
         "libarchive",
         "Real structurally-adversarial archives: libarchive's own conformance/regression test suite "
         "(libarchive/test, tar/test, cpio/test, unzip/test) -- hundreds of hand-crafted malformed and edge-case "
         "tar/cpio/zip/7z/iso9660/mtree fixtures used to test libarchive's own parsers against truncated, "
         "corrupted and adversarially-structured archives. Binary fixtures are stored uuencoded upstream; decoded "
         "in place by decode_uu_files.py.",
         tags=["real"]),
    item("f20-validation-real-cpython-archivetestdata", "F20", "validation", "small", "derive", "derived-from-real",
         derive("f01-validation-cpython-3-13-15-git",
               [copy_whole("f01-validation-cpython-3-13-15-git", src="Lib/test/archivetestdata")]),
         lic("PSF-2.0", True, "Python Software Foundation and CPython contributors",
             "Reused from the already-provisioned F01 validation source-tree item."),
         "cpython",
         "Real structurally-adversarial archives: CPython's zipfile/tarfile test fixture directory (recursion "
         "bombs, backslash-in-name zips, CP437-header zips, an executable-prepended zip, truncated/xz-compressed "
         "tars) -- the same fixtures CPython's own test_zipfile/test_tarfile suites use to test extraction safety "
         "and parser robustness, reused from the already-provisioned F01 validation item.",
         notes="Real content, distinct independence group (cpython) from every other F20 item; derives from an "
               "already-provisioned F01 item so no new third-party bytes are fetched.",
         tags=["real"]),
    item("f20-validation-real-commons-compress-testdata", "F20", "validation", "medium", "git-archive", "real",
         {"git": {"repo": "https://github.com/apache/commons-compress.git",
                 "commit": "852d9c23b94127feafc1649d9c7f13d4df338845", "ref": "rel/commons-compress-1.28.0",
                 "subpaths": ["src/test/resources"]}},
         lic("Apache-2.0", True, "The Apache Software Foundation and Apache Commons Compress contributors", ""),
         "apache-commons-compress",
         "Real structurally-adversarial archives: Apache Commons Compress's test-resource corpus (src/test/"
         "resources at release 1.28.0) -- hundreds of zip/tar/7z/ar/cpio/dump/arj fixtures covering Zip64 edge "
         "cases, UTF-8/EFS flag variants, bzip2/xz/zstd/brotli/lz4 wrapped archives, and known-malformed inputs "
         "collected from real-world interoperability and CVE-class parser bugs.",
         tags=["real"]),
    item("f20-heldout-real-go-archive-testdata", "F20", "heldout", "small", "derive", "derived-from-real",
         derive("f01-heldout-go-1-25-0-src",
               [copy_whole("f01-heldout-go-1-25-0-src", src="src/archive/zip/testdata", dest="zip-testdata"),
                copy_whole("f01-heldout-go-1-25-0-src", src="src/archive/tar/testdata", dest="tar-testdata")]),
         lic("BSD-3-Clause", True, "The Go Authors", "Reused from the already-provisioned F01 heldout item."),
         "golang",
         "Held-out: real structurally-adversarial archives, the Go standard library's archive/zip and archive/tar "
         "test fixture directories (malformed headers, zip64 edge cases, sparse/PAX/GNU tar variants, "
         "multi-header and pre-CVE regression fixtures), reused from the already-provisioned F01 heldout item.",
         notes="Held-out: distinct independence group (golang) from every tuning/validation F20 item; derives "
               "from a heldout-split source (f01-heldout-go-1-25-0-src).",
         tags=["real"]),
    item("f20-heldout-real-fastzip-malo", "F20", "heldout", "small", "git-archive", "real",
         {"git": {"repo": "https://github.com/fastzip/malo.git",
                 "commit": "aeb793c4c164ff8e0a7c50b88ab7a4a05d029d3a", "ref": "main"}},
         lic("BSD-2-Clause", True, "fastzip contributors", ""),
         "fastzip-malo",
         "Held-out: real structurally-adversarial archives, the fastzip/malo conformance corpus -- zip/tar/zstd/"
         "nar/zar archives sorted by fixture author into accept/iffy/reject/malicious directories per format "
         "(zip/malicious/ alone has short_usize, zip64-EOCD-confusion, zip-in-zip, trailing-slash-name/payload "
         "and unicode-extra-field-chain fixtures) -- purpose-built to probe exactly the parser-disagreement "
         "surface this gap targets.",
         notes="Held-out: distinct independence group (fastzip-malo) from every tuning/validation F20 item.",
         tags=["real"]),
    item("f20-heldout-real-fifield-zipbomb-regen", "F20", "heldout", "small", "build", "derived-from-real",
         {"inputs": [{"name": "zipbomb-src", "url": "https://www.bamsoftware.com/hacks/zipbomb/"
                     "zipbomb-20210121.zip",
                     "sha256": "50243fafe7407d88f08493ca53d61bd56504bf88fc35eabee2e7a391e08330ae", "size": 18130,
                     "notes": "public domain (David Fifield); size matches the file as published"}],
          "generator": gen("fifield_zipbomb_regen.py", 81011, {"zip_input": "zipbomb-src"}),
          "output_pin": "TOFU"},
         lic("Public domain", True, "David Fifield",
             "The generator script itself is public domain; per REQ-ECO-0174 the bomb archives it produces are "
             "regenerated fresh by this item's own generator run, never vendored as pre-made files."),
         "fifield-zipbomb",
         "Held-out: real overlapping-entry zip bombs regenerated (not vendored) from David Fifield's public-domain "
         "zipbomb generator (bamsoftware.com/hacks/zipbomb) -- full_overlap and quoted_overlap constructions where "
         "many directory entries' compressed-data ranges overlap in the archive body, the same technique behind "
         "well-known bombs such as 42.zip.",
         notes="Held-out: distinct independence group (fifield-zipbomb) from every tuning/validation F20 item and "
               "from every generated bomb_variants.py/compressible_bombs.py item (a different construction: "
               "overlapping entries, not codec expansion ratio).",
         tags=["real", "regenerated"]),
    item("f20-tuning-generated-malicious-structures", "F20", "tuning", "small", "generate", "generated",
         {"generator": gen("malicious_archives.py", 81012, {"variant": "tuning", "marker": "eb-f20-tuning"})},
         GENERATED_LIC, "ebrc-g2-f20-malicious-tuning",
         "Generated structurally-adversarial zip/tar archives: path traversal (../, absolute, backslash-style "
         "names), a symlink entry escaping the extraction root followed by an entry that walks through it, "
         "duplicate entry names, and a zip whose local file header disagrees with its central directory (a "
         "smuggled filename; a lying compression-method byte) -- and tar equivalents (traversal, symlink escape, "
         "hardlink escape to an absolute path, duplicate names).",
         tags=["generated"]),
    item("f20-validation-generated-malicious-structures", "F20", "validation", "small", "generate", "generated",
         {"generator": gen("malicious_archives.py", 81013, {"variant": "validation", "marker": "eb-f20-validation"})},
         GENERATED_LIC, "ebrc-g2-f20-malicious-validation",
         "Generated structurally-adversarial zip/tar archives (same construction as f20-tuning-generated-"
         "malicious-structures, independent marker/seed): path traversal, symlink escape, duplicate names, and "
         "zip CD/LFH mismatch, plus tar traversal/symlink-escape/hardlink-escape/duplicate-names.",
         tags=["generated"]),
    item("f20-heldout-generated-malicious-structures-v2", "F20", "heldout", "small", "generate", "generated",
         {"generator": gen("malicious_archives_v2.py", 81014, {"marker": "eb-f20-heldout"})},
         GENERATED_LIC, "ebrc-g2-f20-malicious-v2-heldout",
         "Held-out: a structurally distinct generator from malicious_archives.py -- Windows-reserved-device-name "
         "and colon-stream zip entry names, a self-referential symlink loop (a->b->a) instead of an outward "
         "escape, Zip64-extra-field duplicate names, a PAX tar member whose extended-attribute path escapes the "
         "root even though its plain ustar name looks safe, and syntactically-malformed 7z and RAR4 headers "
         "(NextHeaderOffset/HEAD_SIZE pointing past end-of-file).",
         notes="Held-out: distinct independence group and distinct generator (malicious_archives_v2.py) from "
               "every tuning/validation F20 item, per the family's own gap-closure requirement.",
         tags=["generated"]),
]
