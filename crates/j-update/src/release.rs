//! Release tooling shares the application's exact signed contract. Secret provisioning
//! belongs to the release pipeline (stdin, protected file, or credentials helper).
use crate::{
    Config, Result, error,
    metadata::{self, Asset, Manifest},
};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

/// Refuse implicit key rotation; an existing public file must match the supplied seed.
pub fn init_key(public_file: &Path, seed: &[u8; 32]) -> Result<()> {
    let public = SigningKey::from_bytes(seed)
        .verifying_key()
        .to_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    if let Ok(existing) = fs::read_to_string(public_file)
        && !existing.trim().is_empty()
    {
        if existing.trim() != public {
            return Err("The signing key does not match the pinned public key. Key rotation must be explicit.".into());
        }
        return Ok(());
    }
    fs::write(public_file, format!("{public}\n")).map_err(error)
}

/// Include every present configured package (including jvid's manual distributions).
/// Missing packages are allowed; an empty release is not. No output on key mismatch.
pub fn generate(
    config: &Config,
    version: &str,
    notes: &str,
    dir: &Path,
    seed: &[u8; 32],
) -> Result<()> {
    config.validate()?;
    let key = SigningKey::from_bytes(seed);
    if key.verifying_key().to_bytes() != config.public_key {
        return Err("Signing key does not match the pinned public key.".into());
    }
    let version = metadata::stable_version(version)?.to_string();
    let mut assets = Vec::new();
    for p in &config.packages {
        let name = format!("{}-{version}-{}", config.asset_prefix, p.suffix);
        let path = dir.join(&name);
        if !path.exists() {
            continue;
        }
        let mut file = fs::File::open(path).map_err(error)?;
        let size = file.metadata().map_err(error)?.len();
        let mut hash = Sha256::new();
        std::io::copy(&mut file, &mut hash).map_err(error)?;
        assets.push(Asset {
            target: p.target.clone(),
            distribution: p.distribution.clone(),
            name,
            size,
            sha256: format!("{:x}", hash.finalize()),
        });
    }
    let manifest = Manifest {
        schema: 1,
        repository: config.releases_repo.clone(),
        version,
        notes: notes.into(),
        assets,
    };
    manifest.validate(config)?;
    let bytes = serde_json::to_vec_pretty(&manifest).map_err(error)?;
    let sig = key.sign(&bytes).to_bytes();
    metadata::verify(&bytes, &sig, config)?;
    fs::write(dir.join(&config.manifest_name), bytes).map_err(error)?;
    fs::write(dir.join(&config.signature_name), sig).map_err(error)
}

pub fn verify(config: &Config, version: &str, dir: &Path) -> Result<()> {
    let m = metadata::verify_files(config, dir)?;
    if m.version != metadata::stable_version(version)?.to_string() {
        return Err("Release version mismatch.".into());
    }
    for a in &m.assets {
        crate::network::copy_verified(
            fs::File::open(dir.join(&a.name)).map_err(error)?,
            std::io::sink(),
            a,
            &Default::default(),
            |_| {},
        )?;
    }
    Ok(())
}

/// Preserve an existing Windows signing entry; creating a new entry never rotates a pinned key.
pub fn init_key_in_store(public_file: &Path, credential_name: &str) -> Result<()> {
    use zeroize::Zeroizing;
    let seed = match crate::credentials::read(credential_name)? {
        Some(seed) => seed,
        None => {
            let mut bytes = Zeroizing::new(vec![0; 32]);
            getrandom::fill(&mut bytes).map_err(|_| "Cannot generate signing seed.")?;
            // A missing secret must never implicitly replace an existing public pin.
            if fs::read_to_string(public_file).is_ok_and(|s| !s.trim().is_empty()) {
                return Err("No stored signing seed matches the existing public key. Restore the original seed.".into());
            }
            crate::credentials::write(credential_name, &bytes)?;
            bytes
        }
    };
    init_key(
        public_file,
        seed.as_slice()
            .try_into()
            .map_err(|_| "Invalid stored signing seed.")?,
    )
}

#[cfg(test)]
mod key_tests {
    #[test]
    fn init_key_never_implicitly_rotates_an_existing_pin() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("public.hex");
        super::init_key(&path, &[31; 32]).unwrap();
        let original = std::fs::read(&path).unwrap();
        super::init_key(&path, &[31; 32]).unwrap();
        assert!(super::init_key(&path, &[32; 32]).is_err());
        assert_eq!(std::fs::read(path).unwrap(), original);
    }
}

#[cfg(test)]
mod ported_jplay;

#[cfg(test)]
mod ported_jvec;
