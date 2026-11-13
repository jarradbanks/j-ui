# Item 18: font-aware input centring

The before/after images use the bundled Inter font with `FontTweak.y_offset = -2pt`, reproducing the reported 1.5px upward displacement at 1× DPI. The same tests also cover unchanged Inter and a downward-displaced font. No font asset or dependency was added.

Rows use 24/32/36/48pt field heights; columns show plain placeholder, plain value, search placeholder and search value. Each contains “Search Games”. Images are rendered by `egui_kittest` with lavapipe at 1280×720. Geometry and raster checks also run at 1920×1080.

| DPI | Before | After | Focused selection |
| --- | --- | --- | --- |
| 1× | [Before](18-before.png) | [After](18-after.png) | [Selection](18-selection-1x.png) |
| 1.25× | [Before](18-before-1.25x.png) | [After](18-after-1.25x.png) | [Selection](18-selection-1.25x.png) |
| 1.5× | [Before](18-before-1.5x.png) | [After](18-after-1.5x.png) | [Selection](18-selection-1.5x.png) |

The final regression tests fail against untouched main, then pass with the shared fix: shifted-font ink is within 0.5 physical px of the field centre, and Inter remains within its existing 1px tolerance. Caret, selection geometry and IME output share the editor's translated origin; placeholder-to-value replacement keeps a stable baseline. Existing multiline padding tests remain unchanged.

The reference capital measures the font's ink, independent of the field value. This preserves a stable baseline when the value gains descenders or accents.

The real jclip Onest font also passes a tightened 0.5px raster check for placeholder and typed search text on both screens, at 1280×800/1920×1080 and 1/1.25/1.5× DPI. These app tests run in a throwaway caller copy with a path dependency; no sibling checkout is edited. Before captures come from B2's original backport-pin baseline; after captures restore B2's search fixtures against this shared fix.

| Screen | Before | After |
| --- | --- | --- |
| Clips Game Filters | [Before](18-clips-before.png) | [After](18-clips-after.png) |
| Games | [Before](18-games-before.png) | [After](18-games-after.png) |

Only `input_singleline_1.25x.png` needed re-recording among the existing j-ui goldens: 1,148 changed pixels in the 36pt row's text. Its rendered diff was inspected; focus, multiline, gallery and other existing snapshots remain unchanged.
