# Soft component profile

The default j-ui profile implements the **Soft / Figtree** selection in
[the component reference](prototype/component-styles/index.html). Home's design
system owns the suite's semantic roles; this profile owns the library's default
geometry. Consumer font and density adapters remain independent.

| Role | Points |
| --- | --- |
| Control / header / application bar | 36 / 48 / 56 |
| Small / control / panel / dialog radius | 8 / 10 / 16 / 20 |
| Panel inset / stack gap | 16 / 12 |
| Button / field horizontal padding | 16 / 12 |
| Dialog padding | 24 |
| Icon / check / toggle | 18 / 18 / 32 × 18 |
| Toggle knob / slider thumb / track | 14 / 16 / 6 |
| Badge height / minimum width | 24 / 72 |
| Display / headline / title / body / small | 28 / 20 / 16 / 14 / 12 |

Figtree uses weights 400, 500 and 600. SemiBold headings have -0.01em tracking.
Use `theme::text` for paragraph leading and headings. Painted control labels
center their visible ink, including when a font has asymmetric line metrics. Figtree has frozen tabular figures; Inter and
locale-specific Noto fonts provide the existing script fallbacks.

Panels and cards have no visible border. Fields use Ground, secondary buttons
use the selection fill, and menus/dialogs/toasts use Raised with the overlay
shadow. The transparent borders in the reference still reserve one point of
layout space. Visible rules remain one physical pixel at fractional DPI.

Buttons, navigation rows and menu rows use fills for focus. Tabs use Ink text.
Only fields, check controls and slider thumbs receive a focus ring: 3 points
outside the former controls, 4 points around the slider thumb. Disabled controls
choose Ink 4 directly instead of applying a second opacity fade.

Dark and Light use the reference's palette unchanged. The reference does not
specify High Contrast. Its border color is too bright for filled controls, so
`Palette::selection_fill` derives a middle tone when Ink on Border would fall
below 4.5:1. In High Contrast the resulting fill is `#626262`, with at least 3:1
contrast against Panel and Raised and at least 4.5:1 for its text. Panels, cards
and controls use the semantic surface outline to preserve their boundaries.

Control colors and focus rings transition over 120ms using CSS ease. Interrupted
transitions continue from the current color; reduced motion applies the target
immediately.

`tests/components.rs` renders the reference's component arrangement;
`tests/soft_style.rs` checks state colors, geometry, typography and keyboard
behavior. The gallery's Components tab exercises the reusable additions.
See [the restyle report](docs/soft-restyle.md) for comparison evidence and commands.
