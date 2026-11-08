use super::*;
use ::windows::Win32::Foundation::ERROR_NOT_FOUND;
use ::windows::Win32::Security::Credentials::{
    CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredFree, CredReadW, CredWriteW,
};
use ::windows::core::{PCWSTR, PWSTR};

pub(super) fn read(name: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
    let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let mut credential = std::ptr::null_mut();
    // SAFETY: name is NUL-terminated; credential is a valid out-pointer, freed below on success.
    let result = unsafe {
        CredReadW(
            PCWSTR(name.as_ptr()),
            CRED_TYPE_GENERIC,
            None,
            &mut credential,
        )
    };
    if let Err(e) = result {
        if e.code() == ERROR_NOT_FOUND.to_hresult() {
            return Ok(None);
        }
        return Err(format!("Credential Manager read failed: {}", e.code()));
    }
    // SAFETY: CredReadW returned this allocation and its bounded blob; copy before CredFree.
    let secret = unsafe {
        let c = &*credential;
        let bytes = if c.CredentialBlobSize == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize).to_vec()
        };
        CredFree(credential.cast());
        Zeroizing::new(bytes)
    };
    Ok(Some(secret))
}

pub(super) fn write(name: &str, secret: &[u8]) -> Result<()> {
    if secret.len() > 2560 {
        return Err("Credential is too large.".into());
    }
    let mut name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let mut user: Vec<u16> = "j-update".encode_utf16().chain(Some(0)).collect();
    let credential = CREDENTIALW {
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(name.as_mut_ptr()),
        UserName: PWSTR(user.as_mut_ptr()),
        CredentialBlobSize: secret.len() as u32,
        CredentialBlob: secret.as_ptr().cast_mut(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        ..Default::default()
    };
    // SAFETY: all buffers remain live through the synchronous call; flags=0 copies the blob.
    unsafe { CredWriteW(&credential, 0) }
        .map_err(|e| format!("Credential Manager write failed: {}", e.code()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::windows::Win32::Security::Credentials::CredDeleteW;
    #[test]
    fn binary_credentials_round_trip_in_an_isolated_test_entry() {
        let name = format!(
            "vectorize/test/{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        struct Cleanup(Vec<u16>);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                // SAFETY: test-only NUL-terminated target; delete only the unique fixture entry.
                let _ = unsafe { CredDeleteW(PCWSTR(self.0.as_ptr()), CRED_TYPE_GENERIC, None) };
            }
        }
        let _cleanup = Cleanup(name.encode_utf16().chain(Some(0)).collect());
        assert!(read(&name).unwrap().is_none());
        write(&name, b"fixture\0secret").unwrap();
        assert_eq!(read(&name).unwrap().unwrap().as_slice(), b"fixture\0secret");
    }
}
