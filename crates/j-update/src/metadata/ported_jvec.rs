#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use crate::fixtures::TARGET;
    use crate::metadata::*;
    #[allow(dead_code)]
    const REPOSITORY: &str = "jarradbanks/jvec-desktop";
    fn config() -> crate::Config {
        crate::fixtures::config("jvec", &crate::fixtures::key(37))
    }

    #[test]
    fn portable_external_and_wrong_platform_builds_never_self_install() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("vectorize-app.exe");
        assert_eq!(distribution(&config(), &exe), "external");
        for p in &config().packages {
            let marker = p.distribution.as_str();
            std::fs::write(
                dir.path().join("vectorize-distribution"),
                format!("{marker}\n"),
            )
            .unwrap();
            assert_eq!(distribution(&config(), &exe), marker);
            assert_eq!(
                self_update("x86_64-pc-windows-msvc", marker),
                marker == "windows-installer"
            );
        }
        for target in [
            "aarch64-pc-windows-msvc",
            "aarch64-apple-darwin",
            "x86_64-unknown-linux-gnu",
        ] {
            assert!(!self_update(target, "windows-installer"));
        }
        assert!(!self_update("x86_64-pc-windows-msvc", "external"));
        assert!(!self_update("x86_64-pc-windows-msvc", "unknown"));
        std::fs::write(dir.path().join("vectorize-distribution"), "unknown\n").unwrap();
        assert_eq!(distribution(&config(), &exe), "external");
    }
}

#[cfg(test)]
mod contract_tests {
    #[allow(unused_imports)]
    use crate::fixtures::TARGET;
    use crate::metadata::*;
    #[allow(dead_code)]
    const REPOSITORY: &str = "jarradbanks/jvec-desktop";
    fn config() -> crate::Config {
        crate::fixtures::config("jvec", &crate::fixtures::key(37))
    }

    use ed25519_dalek::{Signer, SigningKey};
    fn verify_fixture(b: &[u8], s: &[u8], k: &[u8; 32]) -> crate::Result<Manifest> {
        let mut c = config();
        c.public_key = *k;
        verify(b, s, &c)
    }
    fn manifest() -> Manifest {
        Manifest {
            schema: 1,
            repository: REPOSITORY.into(),
            version: "1.10.0".into(),
            notes: "notes".into(),
            assets: vec![Asset {
                target: TARGET.into(),
                distribution: "windows-installer".into(),
                name: "vectorize-1.10.0-windows-x64-setup.exe".into(),
                size: 1,
                sha256: "ab".repeat(32),
            }],
        }
    }
    #[test]
    fn stable_versions_use_semantic_order_and_reject_nonstable_tags() {
        assert!(stable_version("v1.10.0").unwrap() > stable_version("1.9.9").unwrap());
        for version in [
            "1.2",
            "01.2.0",
            "1.2.0-beta.1",
            "v1.2.0+build",
            "sources-v1.2.0",
        ] {
            assert!(stable_version(version).is_err(), "{version}");
        }
    }
    #[test]
    fn signed_contract_binds_schema_repository_target_format_name_size_and_hash() {
        let good = manifest();
        good.validate(&config()).unwrap();
        let mut invalid = Vec::new();
        let mut m = good.clone();
        m.schema = 2;
        invalid.push(m);
        let mut m = good.clone();
        m.repository = "someone/else".into();
        invalid.push(m);
        let mut m = good.clone();
        m.assets[0].target = "aarch64-pc-windows-msvc".into();
        invalid.push(m);
        let mut m = good.clone();
        m.assets[0].distribution = "windows-portable".into();
        invalid.push(m);
        let mut m = good.clone();
        m.assets[0].name = "../installer.exe".into();
        invalid.push(m);
        let mut m = good.clone();
        m.assets[0].size = 0;
        invalid.push(m);
        let mut m = good.clone();
        m.assets[0].sha256 = "zz".repeat(32);
        invalid.push(m);
        let mut m = good.clone();
        m.assets.push(m.assets[0].clone());
        invalid.push(m);
        let key = SigningKey::from_bytes(&[91; 32]);
        for m in invalid {
            let bytes = serde_json::to_vec(&m).unwrap();
            assert!(
                verify_fixture(
                    &bytes,
                    &key.sign(&bytes).to_bytes(),
                    &key.verifying_key().to_bytes()
                )
                .is_err()
            );
        }
        let bytes = serde_json::to_vec(&good).unwrap();
        let sig = key.sign(&bytes).to_bytes();
        verify_fixture(&bytes, &sig, &key.verifying_key().to_bytes()).unwrap();
        assert!(verify_fixture(&bytes, &sig[..63], &key.verifying_key().to_bytes()).is_err());
        assert!(
            verify_fixture(
                &bytes,
                &sig,
                &SigningKey::from_bytes(&[92; 32]).verifying_key().to_bytes()
            )
            .is_err()
        );
        let mut changed = bytes;
        changed.push(b' ');
        assert!(verify_fixture(&changed, &sig, &key.verifying_key().to_bytes()).is_err());
    }
}
