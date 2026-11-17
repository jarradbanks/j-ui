# Native Component Styles gallery

`cargo run --example gallery` now opens the page in
[`prototype/component-styles/index.html`](../prototype/component-styles/index.html).
The native window uses the same 17 sections, order, headings, copy, examples,
232-point rail, sticky toolbar, 32-point page margins and responsive grid.
The old Widgets/Effects/Components tabs, Inspector and state matrix are removed.

| Area | Implementation |
| --- | --- |
| Page | A scrolling document with the reference's section links, selected link state, wide sections and two/three-column layouts at 1280/1920 points. |
| Toolbar | Live Current/Line/Soft, Figtree/Inter and Dark/Light choices. The font field uses the toolbar's Panel fill. |
| Recent Projects | The three named projects, artwork colors, descriptions, edited captions and More/Share/Open actions; card rows share a footer baseline. |
| Typography / Spacing | The exact specimens and weight labels, timecode, six spacing bars and reference copy, including its literal “12 panel inset” wording. |
| Buttons / Text Input | Both button rows, loading/disabled examples, named/search/numeric/error fields and the reference values. |
| Autocomplete / Dropdown and Menu | Initial `in` query with unfocused suggestions, editable filtering and keyboard selection; frame-rate picker and the four-item menu with shortcuts. |
| Selection / Radio Group | Grid/List/Detail, All/Video/Audio, Loop Playback/Mute/Auto Save and Draft/Standard/High with the same initial selections. |
| Slider and Progress / Navigation | Volume 64, export 38%, breadcrumb, tabs and the collapsible Transform example. |
| Badge and Banner / Toast | Five status tones, three notices and actions, initially visible export toast with working Dismiss/Show Toast. |
| Table / Dialog / Empty State | The four reference rows, selected first row, inline Delete Clip dialog and No Clips Yet composition. |
| Token Map | All 27 rows, matching columns and values that follow the style/theme choices. |

Shared widgets compose the page. The library additions needed by this page are
an opt-in, scoped `theme::with_component_style` presentation override,
`Autocomplete::initially_open`, `Dropdown::fill` and `Slider::show_ticks` (integer
steps without tick marks). Existing constants and the unscoped Soft defaults
retain their values. The Current choice is the reference's
historical comparison, not an app-specific compact adapter.

The `debug-ui` installation and `hot-reload`/Subsecond calls from `954dddb` remain
under their existing debug/feature guards. The executable was built and launched
with both features enabled; its debug bridge supplied the native window captures.

All display copy uses the catalog route. Existing translated labels remain in
the 19-language gallery catalog. New page copy has English and Spanish catalogs;
the other languages use the existing explicit English fallback. Technical token
names, font names, shortcut notation and sample numeric units remain verbatim.
The guard checks the union of both catalogs, and a live Spanish/English switch
and glyph availability are tested.

## Comparison evidence

Every image places HTML on the left and native on the right, without resizing
either capture. A caption band and gutter sit outside the captured content.

| Theme | Actual 1280 × 860 native window, 1× | Actual native window, 1.5× |
| --- | --- | --- |
| Dark | [Compare](screenshots/gallery-page/window-dark-1x.png) | [Compare](screenshots/gallery-page/window-dark-1.5x.png) |
| Light | [Compare](screenshots/gallery-page/window-light-1x.png) | [Compare](screenshots/gallery-page/window-light-1.5x.png) |

At 1.5× the same logical viewport is 1920 × 1290 physical pixels. Native captures
come from the actual `eframe` executable under Xvfb and lavapipe. The Light capture
was reached by clicking its live toolbar control through the debug bridge.

The complete document is also rendered by the actual `gallery()` entry point in
`egui_kittest`, at the same viewport and device scale as Chromium. These large
viewports expose every section in a single image; they are not a separate demo
fixture. The short-window tests separately exercise the sticky chrome and scrolling.

| Theme / scale | 1280 × 4100 logical points | 1920 × 2400 logical points |
| --- | --- | --- |
| Dark / 1× | [Full page](screenshots/gallery-page/page-dark-1280-1x.png) | [Wide page](screenshots/gallery-page/page-dark-1920-1x.png) |
| Dark / 1.5× | [Full page](screenshots/gallery-page/page-dark-1280-1.5x.png) | [Wide page](screenshots/gallery-page/page-dark-1920-1.5x.png) |
| Light / 1× | [Full page](screenshots/gallery-page/page-light-1280-1x.png) | [Wide page](screenshots/gallery-page/page-light-1920-1x.png) |
| Light / 1.5× | [Full page](screenshots/gallery-page/page-light-1280-1.5x.png) | [Wide page](screenshots/gallery-page/page-light-1920-1.5x.png) |

Gallery snapshots additionally cover [Current](../tests/snapshots/gallery_current.png),
[Line with Inter](../tests/snapshots/gallery_line.png) and
[Line Light with Auto Save off](../tests/snapshots/gallery_line_light.png).

## Remaining differences

The section geometry and example content line up. The result is not a
pixel-identical Chromium raster:

- egui rounds panel paint bounds to physical pixels; Chromium retains fractional
  CSS positions. The geometry checks allow 1.5 logical points at the measured
  section boundaries, without accumulating rounded row heights down the page.
- Font rasterization/hinting and fractional glyph placement differ. The shared
  Lucide glyphs also differ from the reference's inline SVGs, particularly the
  thinner empty-state folder; loading-spinner paths and animation phase differ.
- Overlay shadow parameters match, but Chromium and egui use different blur
  kernels. Native rules retain the library's one-physical-pixel contract at
  fractional DPI; a CSS pixel occupies 1.5 physical pixels at 150%.
- New copy falls back to English outside English/Spanish as described above.

Illustrative actions such as Save, New Project and Delete have the same demo-only
behavior as the HTML. The reference's interactive choices, filtering, selections,
slider, disclosure, toast and navigation work. There are no file operations or
persistence in either page.

## Verification

Run on 2026-11-17 from this worktree:

```sh
cargo fmt --all --check
cargo build --workspace
cargo build --example gallery --features debug-ui,hot-reload
cargo clippy --workspace --all-targets -- -D warnings
UPDATE_SNAPSHOTS=1 VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json \
  WGPU_BACKEND=vulkan JVID_GPU_TESTS=required cargo test --test gallery -- --test-threads=1
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json \
  WGPU_BACKEND=vulkan JVID_GPU_TESTS=required cargo test --workspace --no-fail-fast -- --test-threads=1
python3 /mnt/c/Users/Jarrad/.agents/skills/code-cleanup/style-check.py
git diff --check
```

All 160 tests pass, with no failures or ignored tests; the final run compares
snapshots without update mode. Native polish checks pass for section order,
columns, margins/gutters, equal project footer alignment, control heights and
unclipped painted text at 1280/1920 and 1×/1.5×. The existing component, focus,
contrast, keyboard, translation and motion tests also pass.

The HTML polish check passes at 1280 and 1920 × 860. Its five copy reviews at each
width concern composite menu/table accessible names containing shortcuts or
multiple columns; those names are retained. The style checker reports no failures;
its one review is the optional evidence-directory branch inside the snapshot loop.
Full output is saved in [verification.txt](screenshots/gallery-page/verification.txt).

No versions, consumer pins or consumer apps changed. Consumer builds and a live
hot-swap were not exercised. Nothing was pushed and no PR was opened.
