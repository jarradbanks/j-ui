# Soft restyle completion

The default library profile now implements the Soft / Figtree component set in
[the HTML reference](../prototype/component-styles/index.html). This completes and
corrects the partial implementation in `23a37dc`; the reference is from `c153633`.

The Figtree asset, weight instances, core tokens and component modules from the
partial implementation were retained. This pass corrected geometry, focus,
disabled rendering and keyboard behavior, made the additions available in the
interactive gallery, and refreshed the rendered checks.

## Components

All measurements below are logical points (CSS pixels in the reference).

| Component | Final behavior and presentation |
| --- | --- |
| Typography | Figtree 400/500/600, tabular figures, 28/20/16/14/12 scale, -0.01em SemiBold heading tracking and matching paragraph leading through `theme::text`. Existing script fallbacks remain available. |
| Panels | Borderless Panel fill, radius 16, header 48, inset 16, transparent border space preserved. Panel tabs share the new tab strip. |
| Cards | Raised fill, radius 16, one point of transparent border space, caller-owned artwork and content. Reference tiles have equal heights and 16-point gaps. |
| Buttons | Height 36, radius 10, 16-point padding plus transparent border, Medium labels, 18-point icons. Primary hover/focus and pressed colors, secondary/ghost hover and focus fills, destructive colors and disabled fills match the reference. No focus rings. |
| Text fields | Height 36, Ground fill, radius 10, 12-point padding plus border. Plain/search/prefix/suffix/error forms, muted placeholders, red error line. Focus has a blue edge and a complete 3-point outer ring, including at clipped container edges. |
| Dropdown | Same field geometry; Raised menu, radius 16, 4-point padding, 36-point items, radius 8 item highlight. Selected text is Medium with a trailing tick. Arrow navigation, Enter, Escape, outside click, disabled state and focus restoration work. |
| Menu | Leading check slot, shortcut column, selected/hover/focus fill, disabled ink and one-physical-pixel separator. |
| Autocomplete | Substring matching, highlighted matches, up to five suggestions, immediate filtering, keyboard selection, Escape/reopen behavior and accessible empty results. Popup flips/scrolls to stay in small viewports. |
| Segmented control | Ground track, radius 10, 3-point inner inset including transparent border, 2-point item gap, Raised selected item with radius 8. |
| Pills | 36-point target, 4-point gap, fully rounded selected fill, Medium labels. |
| Checkbox | 18-point box, radius 8, blue checked fill and white tick, 8-point label gap, 36-point target. Focus follows the small box. |
| Toggle | 32 × 18 track, 14-point white knob, 2-point inset. `show_label()` adds the reference's 36-point target and 8-point label gap while preserving the existing bare-switch API. |
| Radio group | 18-point circles, selected blue ring and white center, 8-point label gap, 36-point target. Unselected focus has a Light Blue edge and outer ring. |
| Slider | 6-point track, 16-point thumb, 20-point visual row, blue completed portion, 4-point thumb focus ring. Existing keyboard and drag behavior retained. |
| Progress | 6-point rounded track and blue completed portion, accessible percentage. |
| Navigation | Underlined 48-point tabs with 20-point gaps, 36-point rail rows with 10-point padding, breadcrumb and collapsible section header. Tab/row focus uses ink or fill, without rings. |
| Badge | Height 24, minimum width 72, radius 8, Small Medium labels; Neutral, Info, Success, Warning and Danger tones, plus content-sized `fit()` tags. |
| Banner | Tone fill, radius 10, 16-point horizontal padding, minimum height 48, icon and optional ghost action. Wrapping copy retains paragraph leading. |
| Toast | Raised bordered/shadowed surface, radius 16, 54-point single-line height including the border, success icon, optional action and green dismiss icon. |
| Table | 40-point rows, 8-point padding, 12-point column gaps, Small muted headers, aligned status chips and right-aligned numeric cells. Row selection/focus uses the selection fill. |
| Dialog | Raised surface, radius 20, 24-point padding, overlay shadow, Title and muted Body leading, content-sized right-aligned actions. Existing modal focus, dismissal and long-content behavior retained. |
| Empty state | Centered 32-point icon, Title, Body and action separated by 12 points. Short bodies scroll; the reference-sized panel now keeps the action visible. |

Reusable additions across the complete branch are `Autocomplete`, `Card`, `Column`,
`Table`, `Badge`, `Tone`, `Banner`, `Toast`, `ToastResponse`, `Progress`, `Tabs`,
`NavRow` and `RadioGroup`. This finishing commit also adds `theme::text`,
`Toggle::show_label` and the semantic selection-fill helper.

The gallery's Components tab exercises the additions, including opening and
dismissing a toast. All new gallery text uses the existing localization catalog;
all 19 gallery locales include the new keys and corrected type-scale labels.

## Render comparison

The committed HTML was opened through Orca's browser and captured with Chromium
at a controlled device scale of 1. Native renders use egui's WGPU harness with
lavapipe, after the font-install frame. The native fixture uses the same component
arrangement and content as the reference at 1280 points wide.

| Evidence | Reference | Native |
| --- | --- | --- |
| Dark components | [1280px HTML](screenshots/soft-restyle/prototype-dark-1280.png) | [100%](../tests/snapshots/components_dark.png), [150%](../tests/snapshots/components_dark_1.5x.png) |
| Light components | [1280px HTML](screenshots/soft-restyle/prototype-light-1280.png) | [100%](../tests/snapshots/components_light.png), [150%](../tests/snapshots/components_light_1.5x.png) |
| Wide reference | [Dark 1920px](screenshots/soft-restyle/prototype-dark-1920.png), [Light 1920px](screenshots/soft-restyle/prototype-light-1920.png) | [Interactive gallery, 1920 at 150%](../tests/snapshots/gallery_components_1920_1.5x.png) |
| High Contrast | Not specified by HTML | [Components](../tests/snapshots/components_high-contrast.png), [states at 150%](../tests/snapshots/soft_states_high-contrast_1.5x.png) |
| Default, hover, focus, pressed, disabled | CSS rules in reference | [Dark](../tests/snapshots/soft_states_dark_1x.png), [Light](../tests/snapshots/soft_states_light_1x.png) |
| Live gallery | Reusable components composed in the existing gallery | [1280 at 100%](../tests/snapshots/gallery_components_1280_1x.png), [1280 at 150%](../tests/snapshots/gallery_components_1280_1.5x.png) |

Compared the full pages and full-resolution crops of the cards, controls and
feedback sections. Button widths, control heights, row centers, repeated edges,
chip widths, card heights, field padding, focus extents and palette fills have
additional geometry or pixel assertions. The state matrix covers 15 control rows
in five states across all three built-in themes at 100% and 150%.

The HTML polish check passed at 1280 and 1920 widths, with no ignored rules.
Its five copy reviews per width are composite menu/table accessible names that
include shortcuts or multiple columns; those names were retained. Native polish
checks cover centering, clipping, equal sizes, tab overflow, dialogs, empty states,
selection, updates and translated labels.

## Exact-match limits

The result matches the Soft tokens and component styling, but is not a
pixel-identical Chromium rendering:

- Chromium and egui use different font rasterizers and fractional positioning.
  The native fixture rounds panel/row geometry to physical pixels; cumulative
  panel positions can differ by a few pixels over the full 2980-point page.
  Figtree text metrics and line breaking can differ slightly. The font tests keep
  visual bounds within 0.5 physical pixels of center and raster bounds within 1,
  unchanged by artificial font offsets.
- Native icons retain the shared Lucide font and visible-glyph centering. The
  prototype uses inline SVGs. In particular, the empty-state folder uses the
  existing font stroke instead of the prototype's thinner 1.5 SVG stroke, and
  spinner paths and animation phase differ.
- Overlay shadow tokens match (0, 8, blur 24, black 40%), but egui's shadow kernel
  differs visibly from Chromium, most clearly on Light surfaces.
- Rules remain one physical pixel at fractional DPI, following the library's
  existing rendering contract. A CSS one-pixel border becomes 1.5 physical pixels
  at 150% browser scaling.
- The reference specifies only Dark and Light. High Contrast uses Raised instead
  of its bright Border token for filled controls, and retains outlines on fields
  and off switches. This keeps labels and switch knobs visible. Disabled states
  not specified in the HTML retain the library's semantic disabled treatment.
- Native color transitions follow existing widget behavior; button hover fades
  use 120ms. The HTML applies CSS transitions to more component classes. The
  settled states are the comparison target.
- The reference's Current/Line choices, font-picker demo and Token Map are
  reference-page composition. This change implements the default Soft library
  profile; it does not add alternative density profiles or a token-map widget.
  The native fixture omits the Token Map appendix.

## Verification

Run from this worktree on 2026-11-17:

```sh
cargo fmt --all --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings

UPDATE_SNAPSHOTS=1 \
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json \
WGPU_BACKEND=vulkan JVID_GPU_TESTS=required \
cargo test --workspace --no-fail-fast -- --test-threads=1

VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json \
WGPU_BACKEND=vulkan JVID_GPU_TESTS=required \
cargo test --workspace --no-fail-fast -- --test-threads=1

python3 /mnt/c/Users/Jarrad/.agents/skills/code-cleanup/style-check.py
git diff --check
```

Final command results are recorded in
[verification.txt](screenshots/soft-restyle/verification.txt).

Consumer pins were inspected before changing the public styling surface: jvid
v0.2.0, jclip revision `9610442`, jmix v0.3.0 and jvec-desktop v0.4.0. Consumer
repositories and pins were not changed or built. Version/release work, a compact
jvid profile, pushing and opening a PR are outside this task. Independent
validation remains with the coordinator.
