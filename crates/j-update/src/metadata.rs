use std::collections::HashSet;
use std::path::Path;

use ed25519_dalek::{Signature, VerifyingKey};
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{Config, Result, error};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub repository: String,
    pub version: String,
    pub notes: String,
    pub assets: Vec<Asset>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub target: String,
    pub distribution: String,
    pub name: String,
    pub size: u64,
    pub sha256: String,
}

pub fn stable_version(tag: &str) -> Result<Version> {
    let v = Version::parse(tag.strip_prefix('v').unwrap_or(tag)).map_err(error)?;
    if !v.pre.is_empty() || !v.build.is_empty() {
        return Err("Release version must be stable, without build metadata.".into());
    }
    Ok(v)
}

pub fn unhex<const N: usize>(s: &str) -> Result<[u8; N]> {
    if s.len() != N * 2 || !s.is_ascii() {
        return Err("Invalid hex length.".into());
    }
    let mut out = [0; N];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).map_err(error)?;
    }
    Ok(out)
}

impl Manifest {
    pub fn validate(&self, config: &Config) -> Result<()> {
        config.validate()?;
        if self.schema != 1 || self.repository != config.releases_repo {
            return Err("Unsupported updater schema or repository.".into());
        }
        let v = stable_version(&self.version)?;
        if v.to_string() != self.version || self.notes.len() > 256 * 1024 || self.assets.is_empty()
        {
            return Err("Invalid release metadata.".into());
        }
        let mut seen = HashSet::new();
        for a in &self.assets {
            let suffix = config
                .packages
                .iter()
                .find(|p| p.target == a.target && p.distribution == a.distribution)
                .ok_or("Unknown target/distribution pair.")?
                .suffix
                .as_str();
            if a.name != format!("{}-{}-{suffix}", config.asset_prefix, self.version)
                || [
                    &config.manifest_name,
                    &config.signature_name,
                    &config.install.helper_name,
                ]
                .contains(&&a.name)
                || a.name == "plan.json"
                || a.size == 0
                || a.size > 8 * 1024 * 1024 * 1024
                || !seen.insert((&a.target, &a.distribution))
            {
                return Err("Invalid or duplicate release asset.".into());
            }
            unhex::<32>(&a.sha256)?;
        }
        Ok(())
    }

    pub fn asset(&self, target: &str, distribution: &str) -> Option<&Asset> {
        self.assets
            .iter()
            .find(|a| a.target == target && a.distribution == distribution)
    }
}

/// Verify the exact bytes before parsing. The pinned key is never taken from the release.
pub fn verify(bytes: &[u8], signature: &[u8], config: &Config) -> Result<Manifest> {
    config.validate()?;
    if bytes.len() > 1024 * 1024 {
        return Err("Update metadata is too large.".into());
    }
    VerifyingKey::from_bytes(&config.public_key)
        .map_err(error)?
        .verify_strict(bytes, &Signature::from_slice(signature).map_err(error)?)
        .map_err(|_| "Release metadata signature is invalid.".to_string())?;
    let m: Manifest =
        serde_json::from_slice(bytes).map_err(|_| "Invalid release metadata JSON.")?;
    m.validate(config)?;
    Ok(m)
}

/// Only a recognized install marker grants capability. Portable markers take precedence.
pub fn distribution(config: &Config, executable: &Path) -> String {
    let Some(root) = executable.parent() else {
        return "external".into();
    };
    if config
        .install
        .portable_markers
        .iter()
        .any(|m| root.join(m).exists())
    {
        return "windows-portable".into();
    }
    std::fs::read_to_string(root.join(&config.install.distribution_marker))
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| config.packages.iter().any(|p| p.distribution == *s))
        .unwrap_or_else(|| "external".into())
}

pub fn self_update(target: &str, distribution: &str) -> bool {
    target == "x86_64-pc-windows-msvc" && distribution == "windows-installer"
}

/// Bound local staging reads just like HTTP reads, including immediately before execution.
pub fn verify_files(config: &Config, dir: &Path) -> Result<Manifest> {
    config.validate()?;
    use std::io::Read;
    let read = |name: &str, max: u64| -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        std::fs::File::open(dir.join(name))
            .map_err(error)?
            .take(max + 1)
            .read_to_end(&mut bytes)
            .map_err(error)?;
        if bytes.len() as u64 > max {
            return Err("Update response is too large.".into());
        }
        Ok(bytes)
    };
    verify(
        &read(&config.manifest_name, 1024 * 1024)?,
        &read(&config.signature_name, 64)?,
        config,
    )
}

#[cfg(test)]
mod extra_tests {
    #[test]
    fn stage_reads_and_signed_unknown_fields_are_bounded_and_rejected() {
        use ed25519_dalek::Signer;
        let config = crate::fixtures::config("jvid", &crate::fixtures::key(43));
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(&config.manifest_name),
            vec![0; 1024 * 1024 + 1],
        )
        .unwrap();
        assert!(super::verify_files(&config, dir.path()).is_err());
        let bytes = br#"{"schema":1,"token":"secret"}"#;
        let signature = ed25519_dalek::SigningKey::from_bytes(&[43; 32])
            .sign(bytes)
            .to_bytes();
        assert_eq!(
            super::verify(bytes, &signature, &config).unwrap_err(),
            "Invalid release metadata JSON."
        );
    }
}

#[cfg(test)]
mod ported_jplay;

#[cfg(test)]
mod ported_jvid;

#[cfg(test)]
mod ported_jvec;
