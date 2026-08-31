# ZIP compatibility probe tooling

This directory is development/conformance tooling, not production code.

Run from the repository root:

```text
python tools/zip-compat/regenerate_matrix.py
```

The script verifies exact installed runtime versions, generates every ZIP case
from source, compiles the Java probe, runs Python `zipfile`, Java `ZipFile`, Java
`ZipInputStream`, and libarchive `bsdtar`, then rewrites
`observed-outcomes-v1.json`. Production Entrybound never starts these programs.

The checked matrix is behavioral evidence. Changing a frozen profile requires
a new versioned profile ID and a reviewed matrix update.
