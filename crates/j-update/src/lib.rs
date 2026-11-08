//! UI-free signed GitHub release updates, shared by egui and Iced applications.
//! Applications own configuration, credential fallback, scheduling, final saves and shutdown.
//! No application is migrated by adding this crate.
//!
//! # Configuration and calls
//!
//! Depend on `j-update = { git = "https://github.com/jarradbanks/j-ui" }`.
//! The updater knows only the **releases repository**; source hosting is independent.
//! There are no production app identities, release keys, versions or credential defaults.
//!
//! ```
//! use j_update::{Config, InstallConfig, Package, Client};
//! fn jvec_config(public_key: [u8; 32], target: String, distribution: String) -> Config {
//!     Config {
//!         releases_repo: "jarradbanks/jvec-releases".into(),
//!         public_key, // app's existing pinned key, also compiled into its helper
//!         asset_prefix: "vectorize".into(),
//!         manifest_name: "vectorize-update.json".into(),
//!         signature_name: "vectorize-update.json.sig".into(),
//!         target, distribution,
//!         packages: vec![Package {
//!             target: "x86_64-pc-windows-msvc".into(),
//!             distribution: "windows-installer".into(),
//!             suffix: "windows-x64-setup.exe".into(),
//!         }],
//!         install: InstallConfig {
//!             distribution_marker: "vectorize-distribution".into(),
//!             portable_markers: vec!["vectorize-portable".into()],
//!             executables: vec!["jvec.exe".into(), "vectorize-desktop.exe".into()],
//!             helper_name: "vectorize-update-helper.exe".into(),
//!             process_names: vec!["jvec".into(), "vectorize-desktop".into()],
//!         },
//!     }
//! }
//! # fn use_config(config: Config, current: &str) -> j_update::Result<()> {
//! let client = Client::new(config)?; // anonymous; no Credential Manager access
//! let available = client.check(current)?;
//! // On a worker: client.download(update, &cancel_flag, |received| ...).
//! // Or use state::Updater::{check, download, poll, cancel} to own worker threads.
//! # Ok(()) }
//! ```
//!
//! Only transitional builds pointing at an old private repository attach a fallback:
//! `client.with_token_provider(|| credentials::token(credentials::JPLAY_GITHUB))`.
//! Closures implement [`TokenProvider`]; custom providers return `Result<Option<Token>>`.
//! Successful anonymous requests never invoke one. Existing Windows token and signing entry
//! names are exported by [`credentials`]; new installs do not provision a GitHub token.
//!
//! Pass executable, helper and report paths in [`InstallPaths`]. After a successful final
//! save/checkpoint and a fresh [`state::Work`] check, call [`state::Updater::prepare`] or
//! [`install::prepare`]. Keep [`install::Prepared`] until shutdown succeeds, then `launch`.
//! Each app ships a tiny helper calling [`install::run_helper`] with the same **compiled-in**
//! config and the installed build's version. Never load its key/config/version from the
//! untrusted stage or plan. The helper copies itself out, waits for the original process,
//! revalidates the payload and invokes Inno, which owns file replacement and rollback.
//! There is deliberately no generic helper executable accepting a staged public key.
//!
//! `j_ui::update::Prompt` renders State and returns actions; callers retain persistence,
//! notification policy, release links and shutdown ownership. `save_before_restart` requests
//! the app's save flow; it never bypasses Work at preparation. Iced uses this crate directly.
//!
//! # Future app migrations (none performed here)
//!
//! | App | Releases repository | Contract to retain |
//! | --- | --- | --- |
//! | jvec-desktop | `jarradbanks/jvec-releases` | `vectorize-*` metadata/assets, both executable aliases, portable marker, `Local\\VectorizeDesktopInstalled` mutex |
//! | jvid | `jarradbanks/jvid` | `jvid-*`, `windows-x86_64-*` suffixes and all five existing target/distribution packages |
//! | jclip | Its public releases repository selected after the rename | Explicitly choose the new prefix/metadata/markers; retain old executable/process aliases needed by the bridge release |
//!
//! jclip: delete `jplay-updater` implementation and replace its app's Updates worker/modal
//! with Client/Updater + Prompt. Retain the active-work/unsaved-trim guards, diagnostics,
//! preference/report paths, and an app HelperHooks implementation requesting recorder
//! ShutdownIfIdle and returning the preserved `/TASKS=autostart` (or `/TASKS=`) flag.
//! Keep recorder IPC out of this crate. Include recorder and both app aliases in process names.
//!
//! jvid: delete `jvid-updater` implementation and duplicate update presentation. Retain the
//! final save-before-close flow, active-job checks, preference persistence, startup notice
//! policy and report path. Replace its metadata generator with release::generate or
//! `j-update-tool sign CONFIG VERSION NOTES DIRECTORY`; keep its build target/version/key
//! environment handling in the app's config factory. Retain manual portable/macOS/Linux routes.
//!
//! jvec-desktop: delete `vectorize-updater` implementation, retain its Iced screen and task
//! scheduling (Client is blocking), every-workspace final recovery checkpoint, all intake/job
//! guards and old instance mutex name. Its future egui rebuild may use Prompt. Return the
//! mutex Guard from `install::instance::Guard::hold` for the app lifetime. Keep project,
//! recovery and private artwork handling in the app, never in the installer helper.
//!
//! All three: replace app-specific helper/tool bodies with shared calls, but keep app-pinned
//! config in the helper. Publish signed metadata whose `repository` equals the releases repo.
//! Moving assets alone is insufficient: old repository-bound metadata must be regenerated
//! and signed for the public repo. A bridge release for already-installed private builds
//! needs separately signed metadata in the old private repo, then installs a build configured
//! for the public repo. Preserve signing keys; the rename is not a reason to rotate them.
//! The shared release CLI reads a hex seed from J_UPDATE_SIGNING_KEY or a raw 32-byte seed
//! from J_UPDATE_SIGNING_KEY_FILE; the old JPLAY_SIGNING_KEY_FILE can be mapped by its release
//! script. Windows init-key also accepts an explicit legacy signing credential entry name.
//!
//! # Behavior reconciled
//!
//! * All checks sort stable SemVer releases, ignore drafts/prereleases/non-version tags,
//!   accept a leading v on tags only, and ignore current build metadata for precedence.
//!   Equal versions and downgrades never install. Automatic installation additionally requires
//!   a stable installed version without build metadata (the stricter jplay/jvec helper rule);
//!   jvid previously allowed installed prereleases/build metadata at that boundary.
//! * Choose the newest stable release and fail visibly if it lacks valid update files or a
//!   compatible target/distribution. Do not fall back to an older candidate as jvid did.
//! * Preserve jvid's five manual package types through caller-supplied packages. External
//!   builds can show a matching platform release; only marked Windows x64 installer builds
//!   can self-install. Portable markers override install markers, including renamed aliases.
//! * Use one reqwest transport and API asset IDs for public and private repos. Anonymous is
//!   the default; an explicit provider retries API 401/404 once. No token in settings/plans,
//!   debug output, HTTP bodies, parse errors or URL diagnostics. Token length is capped at
//!   2560 bytes (jvec/Credential Manager's stricter limit, versus jplay's 4096).
//! * Strip authorization on every redirect, even back to the same host; require HTTPS and
//!   reject redirect userinfo. Requests have 10-second connect, 30-second API and 600-second
//!   artifact budgets; redirect requests keep the same budget. This replaces jvid's ureq
//!   30-second per-read transport with a bounded overall artifact operation.
//! * Listing cap is jvid's stricter 4 MiB/page, at most 10 pages of 100. Metadata is at most
//!   1 MiB, signatures exactly 64 bytes, notes 256 KiB, artifacts 1..=8 GiB. Keep schema,
//!   repository, canonical version, package pair, filename, SHA-256, duplicate/unknown-field
//!   checks and exact-byte strict Ed25519 verification. Check duplicates/upload state/nonzero
//!   asset IDs for anonymous releases too. Local staging reads have the same byte limits.
//! * Validate again before staging a download, during prepare, before helper waiting and
//!   immediately before execution. Unsigned Available fields cannot choose another payload.
//!   Wrong targets are errors, never Current. Partial/cancelled downloads are removed.
//! * Keep monotonic clamped progress and discard cancelled receivers so late results cannot
//!   revive a download. Checks preserve Ready. Dropping the worker owner cancels its download.
//! * Shared Prompt dismissal cancels an active download (jclip's stricter policy; jvid allowed
//!   downloads to continue when hidden). Dismissing Ready preserves it until process exit.
//!   Both Check and install stay disabled when their state/work guards require it.
//! * Unsaved work blocks core preparation. Prompt can request Save-and-Restart only when
//!   the app explicitly supports it; recording, exports and other active jobs still block.
//! * Use jvec/jclip's executable identity check, reject zero/out-of-range PIDs, wait up to
//!   120 seconds, then require every configured app/companion process in that install to be
//!   idle. Never terminate another process. Defer on an uninspectable matching process.
//! * Probe report permissions before shutdown. Report all helper failures. Initial invalid
//!   metadata/layout or an unconfirmed parent prevents install/relaunch; after confirmed
//!   exit, installer/hook/final-verification failures still attempt to restore the app.
//!   Treat exit 3010 as reboot-required; combine install/restart errors. Inno retains file
//!   rollback and user-data ownership. Never recursively remove a caller-supplied stage.
//! * Signed manifest/asset JSON remains schema 1. Public keys, asset naming, paths, target,
//!   install aliases and companion behavior are caller configuration/hooks, not app defaults.
//!
//! # Source/test provenance
//!
//! Read after fetching main: jplay/jclip e16131b, jvid f5cfe84 (updater unchanged from b515c03),
//! jvec-desktop 973948c. Jvid's dirty CI edit prevented a working-tree pull and was preserved.
//! All 39 original test cases are represented by `ported_jplay`, `ported_jvid`, `ported_jvec`,
//! the shared binary-credential round trip and Work guards, plus `tests/generator.rs`.
//! The original opt-in Windows/Inno tests remain opt-in; packaging recipes/icons under
//! tests/fixtures are isolated test inputs, not app packaging. The UI has interaction tests
//! and an eight-state gallery snapshot in dark, light and high-contrast themes.
pub mod config;
pub mod credentials;
pub mod install;
pub mod metadata;
pub mod network;
pub mod release;
pub mod state;

pub use config::{Config, InstallConfig, InstallPaths, Package};
pub use network::{Client, Token, TokenProvider};
pub type Result<T> = std::result::Result<T, String>;
pub(crate) fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[cfg(test)]
mod fixtures;
