use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// Trusted application configuration, compiled into both the app and its helper.
/// `releases_repo` is the public distribution repository, independent of source hosting.
/// Never deserialize this from a downloaded/staged update in the install helper.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub releases_repo: String,
    pub public_key: [u8; 32],
    pub asset_prefix: String,
    pub manifest_name: String,
    pub signature_name: String,
    pub packages: Vec<Package>,
    pub target: String,
    pub distribution: String,
    pub install: InstallConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub target: String,
    pub distribution: String,
    /// Asset name is `{asset_prefix}-{version}-{suffix}`.
    pub suffix: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallConfig {
    pub distribution_marker: String,
    pub portable_markers: Vec<String>,
    /// All accepted names, including aliases left by a rename.
    pub executables: Vec<String>,
    pub helper_name: String,
    /// All app/companion process names, without `.exe`. Passed as data to PowerShell.
    pub process_names: Vec<String>,
}

/// Paths selected by the app; never include credentials in a plan.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallPaths {
    pub executable: PathBuf,
    pub helper: PathBuf,
    pub report: PathBuf,
}

fn component(s: &str) -> bool {
    !s.is_empty()
        && s != "."
        && s != ".."
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        let repo: Vec<_> = self.releases_repo.split('/').collect();
        if repo.len() != 2 || !repo.iter().all(|s| component(s)) {
            return Err("Invalid GitHub releases repository.".into());
        }
        let names = [
            &self.asset_prefix,
            &self.manifest_name,
            &self.signature_name,
            &self.install.distribution_marker,
            &self.install.helper_name,
        ];
        if !names.iter().all(|s| component(s))
            || self.packages.is_empty()
            || self.install.executables.is_empty()
            || self.install.process_names.is_empty()
            || !self
                .install
                .executables
                .iter()
                .chain(&self.install.portable_markers)
                .chain(&self.install.process_names)
                .all(|s| component(s))
        {
            return Err("Invalid updater file or process configuration.".into());
        }
        // Stage control files and the helper must never collide with signed artifacts.
        let mut reserved = HashSet::from(["plan.json"]);
        for name in [
            &self.manifest_name,
            &self.signature_name,
            &self.install.helper_name,
        ] {
            if !reserved.insert(name.as_str()) {
                return Err("Updater file names collide.".into());
            }
        }
        let mut pairs = HashSet::new();
        let mut suffixes = HashSet::new();
        for p in &self.packages {
            if !component(&p.target)
                || !component(&p.distribution)
                || !component(&p.suffix)
                || !pairs.insert((&p.target, &p.distribution))
                || !suffixes.insert(&p.suffix)
            {
                return Err("Invalid or duplicate package configuration.".into());
            }
        }
        if !component(&self.target) || !component(&self.distribution) {
            return Err("Invalid update target or distribution.".into());
        }
        ed25519_dalek::VerifyingKey::from_bytes(&self.public_key)
            .map_err(|_| "Invalid pinned verification key.")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configuration_rejects_paths_urls_duplicate_contracts_and_stage_collisions() {
        let good = crate::fixtures::config("jvid", &crate::fixtures::key(43));
        good.validate().unwrap();
        for change in [
            |c: &mut Config| c.releases_repo = "https://github.com/owner/repo?token=secret".into(),
            |c: &mut Config| c.manifest_name = "../manifest.json".into(),
            |c: &mut Config| c.signature_name = "plan.json".into(),
            |c: &mut Config| c.install.helper_name = c.manifest_name.clone(),
            |c: &mut Config| c.packages.push(c.packages[0].clone()),
            |c: &mut Config| c.install.process_names = vec!["app;bad".into()],
        ] {
            let mut c = good.clone();
            change(&mut c);
            assert!(c.validate().is_err());
        }
    }
    #[test]
    fn portable_marker_overrides_an_installed_marker_for_all_apps() {
        for app in ["jplay", "jvid", "jvec"] {
            let c = crate::fixtures::config(app, &crate::fixtures::key(43));
            let dir = tempfile::tempdir().unwrap();
            let exe = dir.path().join(&c.install.executables[0]);
            std::fs::write(
                dir.path().join(&c.install.distribution_marker),
                "windows-installer",
            )
            .unwrap();
            assert_eq!(crate::metadata::distribution(&c, &exe), "windows-installer");
            std::fs::write(dir.path().join(&c.install.portable_markers[0]), "").unwrap();
            assert_eq!(crate::metadata::distribution(&c, &exe), "windows-portable");
        }
    }
    #[test]
    fn legacy_credential_names_and_stricter_token_size_stay_compatible() {
        use crate::credentials::*;
        assert_eq!(JPLAY_GITHUB, "jplay/github.com/jarradbanks/jplay");
        assert_eq!(
            VECTORIZE_GITHUB,
            "vectorize/github.com/jarradbanks/vectorize-desktop"
        );
        assert_eq!(JPLAY_SIGNING, "jplay/release-signing/ed25519");
        assert_eq!(VECTORIZE_SIGNING, "vectorize/release-signing/ed25519");
        for token in [
            "",
            "with space",
            "with\nnewline",
            "ghp_;",
            &"x".repeat(2561),
        ] {
            assert!(validate_token(token).is_err());
        }
        validate_token(&"x".repeat(2560)).unwrap();
    }
}
