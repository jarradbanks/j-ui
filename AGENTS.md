# Working on j-ui

This repo owns shared Rust themes, Lucide glyphs, widgets, localisation and
component behaviour for the suite. Read `README.md`, the current manifests,
Home’s system through `~/.agents/design.md`, and the target app’s design profile.
Use the shared `design` skill for UI work.

Keep shared components independent of app composition. Inspect callers and their
actual pinned versions before changing a public API or palette. App font/density
adapters are intentional; a common appearance does not require identical fonts.
Use semantic tokens and existing widgets, `widgets::paint_icon` for visible-glyph
centring, and accessible names for icon actions. All user-visible text follows
the existing i18n route.

`examples/gallery.rs` and `tests/gallery.rs` show the components. Layout checks
are in `tests/layout.rs`; translation discipline is in `tests/i18n_guard.rs`.
Also use the relevant dialog/empty-state/selection/update checks. Render across
the supported themes and target DPI sizes when a component’s presentation changes.
The test harness installs fonts on one frame before painting; forced GPU checks
use `JVID_GPU_TESTS=required` with the machine guide’s environment.

For implementation changes, run formatting, strict Clippy, relevant/workspace
tests and the applicable rendered `polish` checks. Verify affected consumers before
changing their pins. Version/tag/release work is performed when requested; this
library has no app-package download footer to invent for release notes.
