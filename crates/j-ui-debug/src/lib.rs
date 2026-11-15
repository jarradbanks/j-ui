//! Development-only native frames and semantic elements for Orca's browser tools.

use std::io;
use std::path::PathBuf;

#[cfg(all(debug_assertions, feature = "bridge"))]
mod bridge;
#[cfg(all(debug_assertions, feature = "bridge"))]
mod input;
#[cfg(all(debug_assertions, feature = "bridge"))]
mod nodes;
#[cfg(all(debug_assertions, feature = "bridge"))]
mod server;

/// Installs once per context, only with `JUI_DEBUG=1` and an absolute `JUI_DEBUG_CONNECTION`.
/// The app must select a disposable profile before constructing its normal state.
pub fn install_from_env(ctx: &egui::Context, app: &str) -> io::Result<Option<PathBuf>> {
    #[cfg(all(debug_assertions, feature = "bridge"))]
    {
        if std::env::var("JUI_DEBUG").as_deref() != Ok("1") {
            return Ok(None);
        }
        let path = std::env::var_os("JUI_DEBUG_CONNECTION")
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::other("JUI_DEBUG_CONNECTION is required"))?;
        bridge::install(ctx, app, &path)?;
        Ok(Some(path))
    }
    #[cfg(not(all(debug_assertions, feature = "bridge")))]
    {
        _ = (ctx, app);
        Ok(None)
    }
}

/// Adds a domain path and caller source to the response's clipped viewport rectangle.
/// Standard egui/AccessKit elements are discovered automatically when installed.
#[track_caller]
pub fn target(ui: &egui::Ui, response: &egui::Response, path: &str, label: &str) {
    #[cfg(all(debug_assertions, feature = "bridge"))]
    {
        let location = std::panic::Location::caller();
        ui.ctx().with_plugin::<bridge::Bridge, _>(|bridge| {
            bridge.target(ui, response, path, label, location);
        });
    }
    #[cfg(not(all(debug_assertions, feature = "bridge")))]
    {
        _ = (ui, response, path, label);
    }
}

#[cfg(all(test, not(debug_assertions)))]
mod tests {
    #[test]
    fn release_has_no_bridge() {
        let ctx = egui::Context::default();
        assert!(super::install_from_env(&ctx, "release").unwrap().is_none());
    }
}
