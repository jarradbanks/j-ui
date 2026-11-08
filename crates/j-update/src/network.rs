use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use reqwest::blocking::{Client as Http, Response};
use reqwest::header::{ACCEPT, AUTHORIZATION, LOCATION};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use zeroize::Zeroizing;

use crate::metadata::{self, Asset};
use crate::{Config, Result, error};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Available {
    pub version: String,
    pub notes: String,
    pub asset: Asset,
    pub asset_id: u64,
    pub tag: String,
    pub manifest: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Debug)]
pub struct Download {
    pub update: Available,
    pub dir: TempDir,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    #[serde(default)]
    assets: Vec<RemoteAsset>,
}

#[derive(Deserialize)]
struct RemoteAsset {
    id: u64,
    name: String,
    state: String,
}

#[derive(Clone)]
pub struct Client {
    http: Http,
    api: String,
    config: Config,
    token_provider: Option<Arc<dyn TokenProvider>>,
    https: bool,
}

impl Client {
    pub fn config(&self) -> &Config {
        &self.config
    }
    /// Anonymous by default. A provider is consulted only after API HTTP 401/404.
    pub fn new(config: Config) -> Result<Self> {
        Self::at(config, "https://api.github.com".into(), true)
    }

    pub fn with_token_provider(mut self, provider: impl TokenProvider + 'static) -> Self {
        self.token_provider = Some(Arc::new(provider));
        self
    }

    fn at(config: Config, api: String, https: bool) -> Result<Self> {
        config.validate()?;
        let http = Http::builder()
            .https_only(https)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(600))
            .user_agent(concat!("j-update/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| "GitHub request could not be completed.")?;
        Ok(Self {
            http,
            api,
            config,
            token_provider: None,
            https,
        })
    }

    pub fn validate_access(&self) -> Result<()> {
        self.get(&format!("/repos/{}", self.config.releases_repo), false)
            .map(|_| ())
    }

    /// The credential goes only to the fixed API endpoint. Every redirect is a new,
    /// unauthenticated HTTPS request, including redirects back to the API host.
    fn get(&self, path: &str, binary: bool) -> Result<Response> {
        let accept = if binary {
            "application/octet-stream"
        } else {
            "application/vnd.github+json"
        };
        let request = || {
            self.http
                .get(format!("{}{path}", self.api))
                .header(ACCEPT, accept)
                .timeout(Duration::from_secs(if binary { 600 } else { 30 }))
        };
        let mut response = request()
            .send()
            .map_err(|_| "GitHub request could not be completed.")?;
        if matches!(response.status().as_u16(), 401 | 404)
            && let Some(provider) = &self.token_provider
            && let Some(token) = provider
                .token()
                .map_err(|_| "GitHub credential provider failed.")?
        {
            let mut authorization =
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token.0.as_str()))
                    .map_err(|_| "Invalid GitHub credential.")?;
            authorization.set_sensitive(true);
            response = request()
                .header(AUTHORIZATION, authorization)
                .send()
                .map_err(|_| "GitHub request could not be completed.")?;
        }
        for _ in 0..5 {
            if !response.status().is_redirection() {
                if !response.status().is_success() {
                    return Err(format!(
                        "GitHub request failed (HTTP {}).",
                        response.status().as_u16()
                    ));
                }
                return Ok(response);
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|h| h.to_str().ok())
                .ok_or("GitHub returned an invalid download redirect.")?;
            let url = response
                .url()
                .join(location)
                .map_err(|_| "Invalid download redirect.")?;
            if (self.https && url.scheme() != "https")
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return Err("An insecure download redirect was rejected.".into());
            }
            response = self
                .http
                .get(url)
                .header(ACCEPT, accept)
                .timeout(Duration::from_secs(if binary { 600 } else { 30 }))
                .send()
                .map_err(|_| "GitHub request could not be completed.")?;
        }
        Err("Too many download redirects.".into())
    }

    fn bytes(&self, path: &str, binary: bool, max: u64) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        self.get(path, binary)?
            .take(max + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Could not read update response.")?;
        if bytes.len() as u64 > max {
            return Err("Update response is too large.".into());
        }
        Ok(bytes)
    }

    pub fn check(&self, current: &str) -> Result<Option<Available>> {
        self.check_target(current, &self.config.target, &self.config.distribution)
    }

    pub fn check_target(
        &self,
        current: &str,
        target: &str,
        distribution: &str,
    ) -> Result<Option<Available>> {
        let current = semver::Version::parse(current).map_err(error)?;
        let mut releases = Vec::new();
        for page in 1..=10 {
            let bytes = self.bytes(
                &format!(
                    "/repos/{}/releases?per_page=100&page={page}",
                    self.config.releases_repo
                ),
                false,
                4 * 1024 * 1024,
            )?;
            let batch: Vec<Release> =
                serde_json::from_slice(&bytes).map_err(|_| "Invalid GitHub release listing.")?;
            let done = batch.len() < 100;
            releases.extend(batch);
            if done {
                break;
            }
            if page == 10 {
                return Err("Release listing exceeded the updater's limit.".into());
            }
        }
        let versions = stable_releases(releases, &current);
        let Some((version, release)) = versions.into_iter().next() else {
            return Ok(None);
        };
        let asset_id = |name: &str| -> Result<u64> {
            let mut matches = release
                .assets
                .iter()
                .filter(|a| a.name == name && a.state == "uploaded" && a.id != 0);
            let id = matches
                .next()
                .ok_or("The newer release is missing required update files.")?
                .id;
            if matches.next().is_some() {
                return Err("Duplicate release asset name.".into());
            }
            Ok(id)
        };
        let manifest = self.bytes(
            &self.asset_path(asset_id(&self.config.manifest_name)?),
            true,
            1024 * 1024,
        )?;
        let signature = self.bytes(
            &self.asset_path(asset_id(&self.config.signature_name)?),
            true,
            64,
        )?;
        let verified = metadata::verify(&manifest, &signature, &self.config)?;
        if version.to_string() != verified.version {
            return Err("Signed version does not match the GitHub release tag.".into());
        }
        let distribution = if distribution == "external" {
            self.config
                .packages
                .iter()
                .find(|p| p.target == target)
                .map_or(distribution, |p| p.distribution.as_str())
        } else {
            distribution
        };
        let asset = verified
            .asset(target, distribution)
            .ok_or("The newer release has no compatible installer for this platform.")?
            .clone();
        Ok(Some(Available {
            asset_id: asset_id(&asset.name)?,
            version: verified.version,
            notes: verified.notes,
            asset,
            tag: release.tag_name,
            manifest,
            signature,
        }))
    }

    pub fn download(
        &self,
        update: Available,
        cancel: &AtomicBool,
        progress: impl FnMut(u64),
    ) -> Result<Download> {
        self.download_at(
            update.clone(),
            &format!("{}{}", self.api, self.asset_path(update.asset_id)),
            cancel,
            progress,
        )
    }

    fn download_at(
        &self,
        update: Available,
        url: &str,
        cancel: &AtomicBool,
        progress: impl FnMut(u64),
    ) -> Result<Download> {
        // Public Available fields are presentation data, never a trust boundary.
        self.validate_available(&update)?;
        let dir = tempfile::Builder::new()
            .prefix("j-update-")
            .tempdir()
            .map_err(error)?;
        let mut file = std::fs::File::create(dir.path().join(&update.asset.name)).map_err(error)?;
        copy_verified(
            self.get(
                url.strip_prefix(&self.api)
                    .ok_or("Invalid asset endpoint.")?,
                true,
            )?,
            &mut file,
            &update.asset,
            cancel,
            progress,
        )?;
        file.sync_all().map_err(error)?;
        std::fs::write(
            dir.path().join(&self.config.manifest_name),
            &update.manifest,
        )
        .map_err(error)?;
        std::fs::write(
            dir.path().join(&self.config.signature_name),
            &update.signature,
        )
        .map_err(error)?;
        Ok(Download { update, dir })
    }
    fn asset_path(&self, id: u64) -> String {
        format!("/repos/{}/releases/assets/{id}", self.config.releases_repo)
    }

    fn validate_available(&self, update: &Available) -> Result<()> {
        let m = metadata::verify(&update.manifest, &update.signature, &self.config)?;
        if metadata::stable_version(&update.tag)?.to_string() != m.version
            || update.version != m.version
            || m.asset(&self.config.target, &update.asset.distribution) != Some(&update.asset)
            || update.asset_id == 0
        {
            return Err("Update does not match its signed metadata or configured target.".into());
        }
        Ok(())
    }
}

/// Reused by the helper immediately before installation.
pub fn copy_verified(
    mut input: impl Read,
    mut output: impl Write,
    asset: &Asset,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> Result<()> {
    let expected = metadata::unhex::<32>(&asset.sha256)?;
    let mut hash = Sha256::new();
    let mut received = 0;
    let mut buf = [0; 65536];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("Download cancelled.".into());
        }
        let n = input
            .read(&mut buf)
            .map_err(|_| "Could not read update artifact.")?;
        if n == 0 {
            break;
        }
        received += n as u64;
        if received > asset.size {
            return Err("Artifact exceeds its signed size.".into());
        }
        hash.update(&buf[..n]);
        output.write_all(&buf[..n]).map_err(error)?;
        progress(received);
    }
    if received != asset.size || hash.finalize().as_slice() != expected {
        return Err("Artifact size or SHA-256 does not match signed metadata.".into());
    }
    Ok(())
}

/// Zeroized on drop; Debug deliberately never reveals the credential.
pub struct Token(Zeroizing<String>);
impl Token {
    pub fn new(value: String) -> Result<Self> {
        let value = Zeroizing::new(value);
        crate::credentials::validate_token(&value)?;
        Ok(Self(value))
    }
}
impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token([redacted])")
    }
}
pub trait TokenProvider: Send + Sync {
    fn token(&self) -> Result<Option<Token>>;
}
impl<F: Fn() -> Result<Option<Token>> + Send + Sync> TokenProvider for F {
    fn token(&self) -> Result<Option<Token>> {
        self()
    }
}
impl Available {
    pub fn release_url(&self, config: &Config) -> Result<String> {
        config.validate()?;
        metadata::stable_version(&self.tag)?;
        Ok(format!(
            "https://github.com/{}/releases/tag/{}",
            config.releases_repo, self.tag
        ))
    }
}

fn stable_releases(
    releases: Vec<Release>,
    current: &semver::Version,
) -> Vec<(semver::Version, Release)> {
    let mut versions: Vec<_> = releases
        .into_iter()
        .filter(|r| !r.draft && !r.prerelease)
        .filter_map(|r| metadata::stable_version(&r.tag_name).ok().map(|v| (v, r)))
        .filter(|(v, _)| v.cmp_precedence(current).is_gt())
        .collect();
    versions.sort_by(|a, b| b.0.cmp(&a.0));
    versions
}
#[cfg(test)]
mod ported_jplay;
#[cfg(test)]
mod ported_jvec;
#[cfg(test)]
mod ported_jvid;

#[cfg(test)]
mod tests;
