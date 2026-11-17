# Reusable autocomplete

`j_ui::widgets::Autocomplete` is a normal library widget, available without
`debug-ui` or `hot-reload`. It composes the shared `TextInput` and `MenuRow`
controls and follows the active font, theme and density.

Keep the search text separate from the application's selected ID. Use `show`
to commit a choice: its `inner` is `Some(index)` only for Enter or a clicked
suggestion. The index refers to the original options slice, before filtering.
Typing changes the search text and the ordinary `Response::changed()` flag;
it does not return a picked index.

```rust,ignore
// Initialize source_query from the selected source's display name.
let names: Vec<&str> = sources.iter().map(|source| source.name.as_str()).collect();
ui.push_id("capture_source", |ui| {
    let picked = j_ui::widgets::Autocomplete::new(&mut source_query, &names)
        .hint(tr!("settings.search_sources"))
        .empty(tr!("settings.no_matching_sources"))
        .width(ui.available_width())
        .height(36.0)
        .show(ui);
    if let Some(index) = picked.inner {
        selected_source = sources[index].id.clone();
    }
});
```

The translation keys in this example belong to the consumer's catalog. The
consumer also owns device IDs, discovery, loading/errors and applying a choice.
Update the search text when the application changes the selected source, such
as after loading a profile. Use a stable `push_id` for each picker in dynamic
layouts. `ui.add(Autocomplete::new(...))` remains available for simple text
suggestions that only need the standard egui response.

| State or transition | Behavior |
| --- | --- |
| Closed → focus, click or arrow key | Open suggestions; the editor retains keyboard focus. |
| Typing / empty query | Case-insensitive substring filtering; show at most five matches. An empty query shows the first five options. |
| Up / Down | Move the active suggestion, wrapping within the visible matches. |
| Enter / row click | Return its original option index, show the chosen label, and close. A repeated pick returns that index again. |
| Escape / Tab / outside click | Restore the last chosen or externally supplied text and close without a pick. Escape keeps editor focus. |
| No matches / empty options | Show the caller's empty message. Selection keys cannot commit a value. |
| Disabled | Close and cancel any draft; no selection or keyboard input is accepted. Use `ui.add_enabled_ui` or `ui.add_enabled`. |
| External text or option changes | An external text change becomes the cancellation value; matches and returned indices use the current options slice. |
| Resize / DPI | The popup flips or scrolls within the viewport; field height is in logical points. |

The current choice has a trailing check. Accessibility exposes an editable
combo box, listbox, selected options and the active suggestion. Labels come
from the caller; the widget performs no I/O or asynchronous work. Undo/redo
while editing uses the underlying egui text editor. `initially_open(true)` is
an optional unfocused preview used by the component gallery.

Regression coverage is in `tests/autocomplete.rs`, `tests/components.rs` and
`tests/soft_style.rs`, including cancellation, disabling, external changes,
original option indices, keyboard/mouse selection and short popup bounds at
100% and 150% DPI.
