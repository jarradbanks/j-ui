//! Secrets belong to the current Windows user's Credential Manager, never settings or releases.
use crate::Result;
use zeroize::Zeroizing;

pub const JPLAY_GITHUB: &str = "jplay/github.com/jarradbanks/jplay";
pub const VECTORIZE_GITHUB: &str = "vectorize/github.com/jarradbanks/vectorize-desktop";
pub const JPLAY_SIGNING: &str = "jplay/release-signing/ed25519";
pub const VECTORIZE_SIGNING: &str = "vectorize/release-signing/ed25519";

#[cfg(windows)]
#[allow(unsafe_code)]
mod windows;

pub fn read(name: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
    #[cfg(windows)]
    {
        windows::read(name)
    }
    #[cfg(not(windows))]
    {
        let _ = name;
        Err("Credential storage requires Windows.".into())
    }
}

pub fn write(name: &str, secret: &[u8]) -> Result<()> {
    #[cfg(windows)]
    {
        windows::write(name, secret)
    }
    #[cfg(not(windows))]
    {
        let _ = (name, secret);
        Err("Credential storage requires Windows.".into())
    }
}

pub fn token(name: &str) -> Result<Option<crate::Token>> {
    let Some(bytes) = read(name)? else {
        return Ok(None);
    };
    let token = Zeroizing::new(
        String::from_utf8(bytes.to_vec()).map_err(|_| "Invalid stored GitHub credential.")?,
    );
    crate::Token::new(token.to_string()).map(Some)
}

pub fn validate_token(token: &str) -> Result<()> {
    if token.is_empty()
        || token.len() > 2560
        || !token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err("Invalid GitHub token format.".into());
    }
    Ok(())
}
