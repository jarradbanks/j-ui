# j-ui

Shared egui UI kit for jvid, jclip and jvec.

```toml
j-ui = { git = "https://github.com/jarradbanks/j-ui", tag = "v0.2.0" }
```

```sh
cargo run --example gallery   # browse the components
cargo test
```

`Palette` includes `blue_soft`, `blue_hover`, `red_soft`, `amber_soft` and
`green_soft`, also exposed through `tokens::color`. Soft fills are opaque colours
precomposed over the theme's panel. Custom TOML themes may omit tokens; each
missing token uses its Dark value, including these variants.

`Slider::new` stays horizontal by default. Add `.vertical()` for a bottom-to-top
slider; `ui.add_sized([24.0, 88.0], slider.vertical())` sets its height. Vertical
sliders use Up/Down; horizontal sliders use Left/Right. Both support `.step`,
`.centred` and `.state`.

`MenuRow::new(label, checked).shortcut(shortcut).enabled(enabled)` shows a label,
right-aligned shortcut and optional trailing checkmark. Labels elide to keep room
for the shortcut and checkmark. Shortcut text is display only; applications route
commands and decide availability. Disabled rows ignore pointer and keyboard input.

MIT OR Apache-2.0.
