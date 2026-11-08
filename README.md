# j-ui

Shared egui UI kit for the j suite of desktop apps, extracted with history from jvid's `crates/suite-ui`. It includes the existing themes, fonts, widgets, panels, dialogs, localisation, gallery and GPU test helpers, with no visual changes. egui, eframe and egui_kittest are pinned to jvid's resolved version, 0.36.2. Licensed under MIT OR Apache-2.0; bundled font/icon licenses are in `assets/`.

```toml
[dependencies]
egui = "=0.36.2"
j-ui = { git = "https://github.com/jarradbanks/j-ui", tag = "v0.1.0" }
```

```rust
j_ui::theme::install(&ctx); // once at startup, with your egui::Context
// In your UI closure:
ui.add(j_ui::widgets::Button::primary("Save"));
```

```sh
cargo run --example gallery
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
JVID_GPU_TESTS=required cargo test --locked --workspace --no-fail-fast
python3 -m unittest discover -s scripts
python3 scripts/i18n_check.py
pip install fonttools==4.65.0
python3 assets/subset-cjk.py
```

Linux snapshot tests use Mesa lavapipe (`sudo apt install mesa-vulkan-drivers`). The inherited `JVID_GPU_TESTS` variable is retained for consumers. Snapshot baselines are in `tests/snapshots`; intentional updates use `UPDATE_SNAPSHOTS=1 cargo test`.
