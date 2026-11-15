# Native Debug Bridge

`j-ui-debug` is a separate development helper for egui/eframe 0.36.2. It does
not depend on j-ui, change a palette, or require consumers to upgrade their
existing j-ui pin. Native app pixels remain native; the browser displays paired
frames and transparent semantic element proxies. Use **Orca's browser Annotate
and Draw tools** for feedback. There is no annotation editor or feedback delivery
service in this package.

## Integrate

Add an optional Git dependency, pinned to a reviewed feature-branch commit:

```toml
[features]
debug-ui = ["dep:j-ui-debug"]

[dependencies]
j-ui-debug = { git = "https://github.com/jarradbanks/j-ui.git", rev = "<SDK commit>", optional = true }
```

After constructing the egui context, install once:

```rust,ignore
#[cfg(all(debug_assertions, feature = "debug-ui"))]
j_ui_debug::install_from_env(&cc.egui_ctx, "app-id")?;
```

Enable explicitly with `JUI_DEBUG=1` and
`JUI_DEBUG_CONNECTION=/absolute/per-checkout/per-app/connection.json`. Without
both, the bridge stays off or reports a missing-path error. Repeated installation
on the same context and path is idempotent. A different context/process trying
to use the same connection path fails clearly. Bind uses an ephemeral loopback
port, avoiding fixed native port collisions. Dropping the egui context joins the
transport thread and removes only its own connection file; a killed process
releases the OS file lock and the next launch replaces its stale connection.

**Choose a disposable app profile before constructing application state.** The
helper cannot redirect app-specific projects, settings, audio devices, recorder
instances, artwork, or databases. Launchers must set each app's approved profile
environment and keep production resources untouched. The helper never triggers
app exit, save, restart, or process replacement.

The plugin uses egui's input/output hooks. Existing `App::logic`,
`raw_input_hook`, `save`, `on_exit`, clear colours, and close/unsaved-work handling
remain owned by the app. Screenshot replies are inspected without being removed;
typed tokens keep app screenshots distinct from bridge screenshots. Install
other output-changing plugins before this one so the bridge sees their final
layout. Only the root viewport is captured.

Standard egui and j-ui widgets that emit AccessKit metadata are discovered
automatically with real labels, native roles, clipped rectangles, layer order,
and egui IDs. Give dynamic rows explicit IDs, for example a child scope with
`UiBuilder::id(ui.make_persistent_id(domain))`, or the widget's explicit ID API.
An egui auto ID has exactly egui's lifetime; this package cannot make unstable app IDs stable.
Generic containers, text runs, hidden elements, empty/clipped rectangles, and arbitrary
painted pixels do not become semantic widgets. Unlabelled custom painting needs
explicit registration:

```rust,ignore
let response = ui.add(MyCustomMeter::new());
#[cfg(all(debug_assertions, feature = "debug-ui"))]
j_ui_debug::target(ui, &response, "mixer/master/level", "Master Level");
```

Registration enriches an existing semantic widget or adds a `Custom` target.
Its path must be unique in the viewport and supplies its stable domain identity. Caller file/line is available only for
explicit targets; automatic nodes do not invent source locations. Native layer
order and AccessKit child order determine proxy stacking. Arbitrary custom paint
reordering within one layer is not discoverable; register the domain explicitly
and avoid overlapping responses when semantic order differs from paint order.

Explicit registrations preserve the supplied response's visible `rect` and
`interact_rect`, intersect them with the caller's clip rectangle, then apply the
native layer transform and viewport clipping. This supports timeline/domain
widgets whose native interaction area extends beyond their painted lane. Parent
registrations stay beneath semantic children. Recognized printable logical keys
and Space retain their native press/release events alongside text/IME delivery.

## Launch the Browser Viewer

Node 22+ runs the viewer with no production npm dependencies:

```sh
node <j-ui-debug-manifest-directory>/browser/server.mjs \
  --connection /absolute/per-checkout/per-app/connection.json --port 0
```

Resolve the manifest directory from `cargo metadata`'s `packages` entry named
`j-ui-debug`; do not assume a machine path or a separately cloned SDK. Successful
bind writes exactly one JSON line `{"url":"http://127.0.0.1:<port>/"}` to stdout.
Diagnostics go to stderr and failed binds exit nonzero. Open that URL with Orca's
actual browser. A launcher owns viewer/process deduplication and cleanup; stopping
the viewer does not close the native app or discard its work.

`GET /health` returns `{"ready":true}`/200 only after a fresh native snapshot is
available, otherwise `{"ready":false}`/503. It exposes no token. The viewer stays
running when the connection file is absent, when compilation fails, or while the
native process restarts. It rereads the file for each upstream request. The native
connection JSON contains `{url, token, app, pid, protocol: 1}`; `token` is secret,
the file is mode 0600 on Unix, and native endpoints require a bearer token and
reject browser origins. Keep the connection directory private on other platforms.

The viewer binds only `127.0.0.1`, checks Host/Origin/fetch-site, sets an HttpOnly
SameSite=Strict cookie, and proxies native access from the server. Static assets
and frame/input endpoints share its origin. The bearer token never reaches page
JavaScript. There are no CORS allowances or internal Orca APIs.

## Protocol and States

Authenticated native `GET /snapshot` is one binary payload: a little-endian u32
JSON length, UTF-8 metadata, then its matching PNG. Metadata contains
`{session, frame, generation, width, height, ppp, targets}`. Width/height are physical
pixels, `ppp` is pixels per egui point, and `target.rect` is `[left, top, width,
height]` normalized to the captured root viewport after clipping/transforms.
Targets contain `{id, path, label, role, source, rect, z_order, enabled}`.
PNG encoding uses fast compression so native capture does not starve input.
Absolute request/write deadlines also bound shutdown when a client trickles data.

`POST /input` accepts `{session, frame, generation, events}`. Events are `move`,
`button`, `key`, `text`, `wheel`, `modifiers`, `reset`, or `keep_alive`; the schemas
are in `src/input.rs`. Coordinates are normalized, body/event counts are bounded,
and unknown fields, invalid keys/numbers, wrong sessions, stale frames/layouts,
and overflowing queues are rejected. A second fence checks layout, dimensions,
and queue age on the egui thread. The viewer batches input within the same
64-event/16-KiB limits and cancels queued events older than 300 ms or from a
previous layout/session, rather than replaying them after a slow request.
Frame and pending screenshot metadata stay
paired across resize/DPI changes. One outstanding screenshot has a typed ID and
timeout; unrelated or late screenshots cannot replace it.

| State or transition | Behaviour |
| --- | --- |
| Off / release build | No plugin, capture, listener, or remote input; release ignores opt-in even with all features |
| Connecting / missing file | Viewer stays up; health returns 503 |
| Live | Canvas and semantic proxies update together; browser input maps to native points |
| Offline / lost native connection | Old proxies disappear; held input times out within two seconds |
| Source compile failure | Existing native process, frame and state remain live while the developer fixes the source |
| Reconnect / new session | Cancel queued browser input, reset native holds, then show the new frame |
| Viewport/DPI/layout change | Advance generation; reject queued old input and release held controls |
| Focus loss / hidden tab | Reset keys/buttons/modifiers; native timeout covers lost reset requests |
| Duplicate startup / bind failure | File lock or port bind reports an error; no second native owner |
| Screenshot collision / late reply | Independent typed tokens; app replies remain in the event stream |
| Incompatible hot-patch / struct edit | Developer restarts deliberately after preserving work; no automatic app replacement |
| Unsaved work / close | App keeps its normal save and close policy; helper never issues Close |

## Hot Reload and Checks

Keep Subsecond integration in the app: optional `subsecond`/`dioxus-devtools` 0.7,
`connect_subsecond()` in debug startup, and `subsecond::call(|| self.frame(...))`
around the UI body. A bridge is not restart-on-save. Compile errors leave the
existing process/state running; successful compatible patches keep app state.
Structural edits can need restart. Windows hot-patching additionally needs the
upstream DioxusLabs/dioxus#5967 stub RAX fix and PC acceptance.

The self-contained `debug_gallery` example has an optional `hot-reload` feature
for a live Subsecond check without loading user projects or audio devices.
After setting the explicit bridge opt-in and disposable connection path, launch
with dx 0.7.10's default development profile:

```sh
dx serve --desktop --hot-patch --open false --interactive false \
  -p j-ui-debug --example debug_gallery --features hot-reload
```

Do not pass `--profile dev`: dx 0.7.10 adds profile inheritance and Cargo rejects
inheritance on its root `dev` profile. An isolated Linux Xvfb/lavapipe probe kept
the same PID and bridge session through a deliberate compile error and a
successful UI-body patch, preserving the native counter and unsaved text. This
does not verify Windows hot-patching or incompatible struct layout changes.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p j-ui-debug --lib -- --test-threads=1
cargo check -p j-ui-debug --no-default-features
cargo check -p j-ui-debug --release --all-features
JUI_DEBUG=1 cargo test -p j-ui-debug --release --lib release_has_no_bridge
cd crates/j-ui-debug/browser && npm ci && npm test && npm run test:browser
```

On omarchy, serialize heavy commands with `flock /tmp/rust-debug-build.lock` and
`CARGO_BUILD_JOBS=3`. Native rendered checks require isolated Xvfb/lavapipe and
`JVID_GPU_TESTS=required`; browser checks use headless Chromium. Set
`CHROMIUM_PATH` if Playwright's browser is not installed. The browser fixture tests
verify proxy geometry/input/security/reconnect, and cannot prove actual native
hot-patching or Orca toolbar delivery. Those require explicit live acceptance.
