#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use crate::fixtures::TARGET;
    use crate::release::*;
    #[allow(dead_code)]
    const REPOSITORY: &str = "jarradbanks/jvec-desktop";
    fn config() -> crate::Config {
        crate::fixtures::config("jvec", &crate::fixtures::key(37))
    }

    fn verify_fixture(v: &str, d: &Path, k: &[u8; 32]) -> Result<()> {
        let mut c = config();
        c.public_key = *k;
        verify(&c, v, d)
    }
    #[test]
    fn signed_local_release_rejects_wrong_key_wrong_version_and_changed_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vectorize-1.2.0-windows-x64-setup.exe");
        fs::write(&path, b"installer fixture").unwrap();
        let key = SigningKey::from_bytes(&[37; 32]);
        generate(
            &config(),
            "1.2.0",
            "Release notes",
            dir.path(),
            &key.to_bytes(),
        )
        .unwrap();
        verify_fixture("1.2.0", dir.path(), &key.verifying_key().to_bytes()).unwrap();
        assert!(verify_fixture("1.2.1", dir.path(), &key.verifying_key().to_bytes()).is_err());
        assert!(
            verify_fixture(
                "1.2.0",
                dir.path(),
                &SigningKey::from_bytes(&[38; 32]).verifying_key().to_bytes()
            )
            .is_err()
        );
        fs::write(path, b"changed installer").unwrap();
        assert!(verify_fixture("1.2.0", dir.path(), &key.verifying_key().to_bytes()).is_err());
    }
}
