# F03 `shared_packages` covariate (pre-registration)

Gap (corpus round-1 critic, family F03, MAJOR): split labels for F03 (dependency/vendor
trees) pass, but content leaks across splits because unrelated projects in different
splits vendor overlapping sets of third-party packages. Measured from non-held-out
fingerprints before this pass: `f03-validation-yq-go-mod-vendor` is 61.2% identical
(by bytes) to tuning's `f03-tuning-fzf-go-mod-vendor`; `f03-validation-bat-cargo-vendor`
shares 14.7% (79 MiB) with `f03-tuning-ripgrep-cargo-vendor`'s vendor tree; and
`f03-validation-bootstrap-npm-node-modules` shares 4.7% with
`f03-tuning-typescript-npm-node-modules`. The held-out F03 items
(`f03-heldout-alacritty-cargo-vendor`, `f03-heldout-direnv-go-mod-vendor`,
`f03-heldout-pdfjs-npm-node-modules`) are expected to show the same pattern (they are
also small/medium CLI-shaped Rust/Go/npm projects) but this cannot be measured before
the design freeze (`research/decisions/design-freeze.json`) without reading held-out
content, which is against program discipline (see PROGRESS.md).

## What this covariate is

For any experiment that uses one or more F03 items as independent samples (e.g. "how
well does entrybound compress a dependency-vendor tree" aggregated across items), a
`shared_packages` covariate records, per item, the largest pairwise overlap fraction
against every *other* F03 item outside its own `independence_group`, computed by
`f03_lockfile_audit.py` (provenance/name-identity overlap: intersecting each item's
`{name@version}` set parsed from its own Cargo.lock / go.sum / package-lock.json / pip
lock) and, once available, the byte-level overlap already used to state the figures
above (from corpus fingerprint comparison, not this script).

An item is flagged `shared_packages: true` for a given experiment's population if its
largest such overlap fraction exceeds **5%** (the same bar the gap names: "sharing
more than 5% of lock entries"). Every experiment that reports an aggregate statistic
over the F03 population must report it twice: once over the full population, and once
restricted to items with `shared_packages: false` (or, symmetrically, with the
flagged items down-weighted/excluded and the fact stated). This makes the
independence violation legible in every result derived from F03 instead of silently
inflating apparent generalization from what is really a handful of underlying
third-party packages appearing in multiple items.

## Status of this pre-registration

- Tuning/validation: measured now, pre-freeze, by `f03_lockfile_audit.py` (default
  invocation, no `--include-heldout`). Report:
  `research/corpus/generators/g1-code/authoring/f03_lockfile_audit_report.json`.
  Flagged over the 5% provenance-level bar: `f03-tuning-fzf-go-mod-vendor` /
  `f03-validation-yq-go-mod-vendor` (6.2%/6.1% of lock entries by count -- a much
  smaller figure than the 61.2% *byte* overlap already measured; the two are
  different, complementary measures, see the script's docstring),
  `f03-tuning-ripgrep-cargo-vendor` / `f03-validation-bat-cargo-vendor` (31.1%/9.1%),
  and `f03-tuning-typescript-npm-node-modules` /
  `f03-validation-bootstrap-npm-node-modules` (43.2%/13.2%). All three pairs the gap
  named by byte-overlap are also flagged by the independent, cheaper, provenance-level
  method, which is some evidence the method is not missing the cases that matter.
- Held-out: **not measured**. Run, after the design freeze commit exists and
  `--unlock-heldout <design-freeze-commit>` is available:

  ```
  run.sh f03_lockfile_audit --include-heldout --unlock-heldout <design-freeze-commit>
  ```

  This adds `f03-heldout-alacritty-cargo-vendor`, `f03-heldout-direnv-go-mod-vendor`,
  `f03-heldout-pdfjs-npm-node-modules` and `f03-heldout-pypi-web-site-packages` to the
  measured set and recomputes every pair. Any held-out F03 item found over the 5% bar
  at that point should be treated exactly like the tuning/validation ones above (flag
  `shared_packages: true` for it in every experiment, report with/without) rather than
  replaced -- by the time the freeze has happened, swapping the item for a fresh
  download changes the held-out set after commitments were made on it, which is worse
  than carrying a documented covariate.

## Why "flag + report both ways" instead of "replace the item"

The gap text offers either fix ("Replace ... or pre-register a shared_packages
covariate"). Replacing `f03-validation-yq-go-mod-vendor` with a differently-shaped Go
project does not really solve the underlying problem: essentially any small Go/Rust/
npm CLI vendors a handful of the same ubiquitous ecosystem packages (`golang.org/x/*`,
`serde`/`clap`-adjacent crates, `lodash`-adjacent npm packages, etc.), so a replacement
would likely still show *some* overlap with *some* tuning item, just not the specific
one measured today -- and it would cost another vendor-tree acquisition against an
already-tight group budget. Pre-registering the covariate is cheaper, more honest (it
documents the actual population instead of cherry-picking away the inconvenient
member), and composable with future items.
