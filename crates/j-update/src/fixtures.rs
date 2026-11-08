//! Test-only identities copied from the source contracts; no production app defaults.
use crate::{Config, InstallConfig, Package};
pub(crate) const TARGET: &str = "x86_64-pc-windows-msvc";
pub(crate) fn config(app: &str, key: &[u8; 32]) -> Config {
    let (repo, prefix, exe, marker, portable) = match app {
        "jplay" => (
            "jarradbanks/jplay",
            "jplay",
            vec!["jplay.exe", "jclip.exe"],
            "jplay-distribution",
            "jplay-portable",
        ),
        "jvec" => (
            "jarradbanks/jvec-desktop",
            "vectorize",
            vec!["vectorize-desktop.exe", "jvec.exe"],
            "vectorize-distribution",
            "vectorize-portable",
        ),
        _ => (
            "jarradbanks/jvid",
            "jvid",
            vec!["jvid-app.exe"],
            "jvid-distribution",
            "jvid.portable",
        ),
    };
    let packages: &[(&str, &str, &str)] = if app == "jvid" {
        &[
            (TARGET, "windows-installer", "windows-x86_64-setup.exe"),
            (TARGET, "windows-portable", "windows-x86_64-portable.zip"),
            ("aarch64-apple-darwin", "macos-dmg", "macos-aarch64.dmg"),
            (
                "aarch64-apple-darwin",
                "macos-portable",
                "macos-aarch64-portable.zip",
            ),
            (
                "x86_64-unknown-linux-gnu",
                "linux-portable",
                "linux-x86_64-portable.tar.gz",
            ),
        ]
    } else {
        &[(TARGET, "windows-installer", "windows-x64-setup.exe")]
    };
    Config {
        releases_repo: repo.into(),
        public_key: *key,
        asset_prefix: prefix.into(),
        manifest_name: format!("{prefix}-update.json"),
        signature_name: format!("{prefix}-update.json.sig"),
        packages: packages
            .iter()
            .map(|(t, d, s)| Package {
                target: (*t).into(),
                distribution: (*d).into(),
                suffix: (*s).into(),
            })
            .collect(),
        target: TARGET.into(),
        distribution: "windows-installer".into(),
        install: InstallConfig {
            distribution_marker: marker.into(),
            portable_markers: vec![portable.into()],
            executables: exe.into_iter().map(str::to_owned).collect(),
            helper_name: format!("{prefix}-update-helper.exe"),
            process_names: if app == "jplay" {
                vec!["jplay".into(), "jclip".into(), "jplay-recorder".into()]
            } else if app == "jvec" {
                vec!["jvec".into(), "vectorize-desktop".into()]
            } else {
                vec!["jvid-app".into()]
            },
        },
    }
}
pub(crate) fn key(seed: u8) -> [u8; 32] {
    ed25519_dalek::SigningKey::from_bytes(&[seed; 32])
        .verifying_key()
        .to_bytes()
}
