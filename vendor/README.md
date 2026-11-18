# GLib 0.18 soundness backport

`glib/` is the published glib 0.18.5 crate, including its original license,
copyright and upstream tests. Its crates.io archive SHA-256 is
`233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5`.

The only upstream source change is the two-line `VariantStrIter::impl_get`
mutability correction from gtk-rs-core commit
[`b5a4071e439bef2b5eea76c3aa25e5ae84839e34`](https://github.com/gtk-rs/gtk-rs-core/commit/b5a4071e439bef2b5eea76c3aa25e5ae84839e34):
the C out-pointer must reference mutable storage (`let mut p` and `&mut p`).
This fixes [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html)
without changing GTK 0.18 types or introducing another GTK generation.

The root `[patch.crates-io]` selects this source for every GLib 0.18 user.
Do not rename its version or suppress the advisory to obtain a clean audit.
Version-only scanners may still flag 0.18.5; inspect the backport and its tests.
Remove the vendored crate and patch entry when the complete Tauri/GTK graph can
use an upstream fixed release. Do not format or otherwise refactor vendored code.

The adapter's `glib_variant_string_iteration` test covers mixed traversal,
Unicode, empty strings/arrays and both iterator directions. `test:linux` also runs
it in release mode, because debug builds can conceal the original undefined
behavior. The existing upstream string-iterator tests also reproduce the crash
without this backport in a release build.
