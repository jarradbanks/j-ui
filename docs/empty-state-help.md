# Empty-state help layout

An empty help string previously contributed a Body line and a preceding 12pt
gap, and created an empty accessibility label. The icon/title group therefore
sat above its panel center. Help now contributes layout and a label only when
present; actions remain optional and follow the last real text line.

The fix preserves real help, wrapping, short-body scrolling, long translated
actions, viewer inks and keyboard action access. It adds no consumer offset or
public API. jclip removes its old export-queue body extension when adopting it.

[Before and after](screenshots/empty-state-help/before-after.png) uses the same
440×360 native egui fixture, Figtree/Dark at 100%. The `after-*` and `onest-*`
images cover all three themes at 100% and 150%, with and without an action.
They were rendered with isolated lavapipe and inspected. No snapshot golden
was changed by this fix.

## Regression and verification

`tests/empty_state.rs::absent_help_has_no_label_gap_or_height` measures actual
icon bounds, accessible heading/action rectangles, raster bounds and absent
accessibility labels. It failed on base `83a0f0c` with the visible group centered
at y165.5 instead of y180. Real-help centering also runs at both scales; existing
wrapping/overflow, viewer and keyboard checks remain.

Run with `PATH=$HOME/.cargo/bin:$PATH`, `CARGO_BUILD_JOBS=3`,
`VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`, `WGPU_BACKEND=vulkan`,
and `JVID_GPU_TESTS=required`. Heavy commands take `/tmp/rust-debug-build.lock`.

```sh
cargo fmt --all --check
flock /tmp/rust-debug-build.lock cargo clippy --locked --workspace --all-targets -- -D warnings
flock /tmp/rust-debug-build.lock cargo test --locked --workspace --no-fail-fast -- --test-threads=1
```

For the consumer font adapter, set `EMPTY_STATE_FONT_DIR` to jclip's
`crates/jclip-app/assets/fonts` and rerun `cargo test --locked --test empty_state
-- --test-threads=1`. This substitutes its existing Onest 400/500/600 faces into
the suite's primary font slots. Optional `EMPTY_STATE_EVIDENCE` saves the rendered
matrix into an existing directory. These environment hooks affect tests only.

The focused suite passes with Figtree and Onest: 6 passed, 0 failed each.
Formatting and strict workspace/all-target Clippy pass (exit 0). The forced
lavapipe workspace suite passes without snapshot update mode: 165 passed,
0 failed, 0 ignored (exit 0). Scoped style checking passes with no failures;
two optional evidence/font branches are informational review items. Exact
commands and test summaries are in [verification](screenshots/empty-state-help/verification.txt).
This is software-rendered component verification, not capture/GPU hardware or
native Windows acceptance. The separate consumer migration and independent
validator cover the jclip integration.
