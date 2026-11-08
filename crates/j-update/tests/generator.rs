use j_update::*;
use std::{fs, process::Command};
#[path = "../src/fixtures.rs"]
mod fixtures;

#[test]
fn pipeline_generator_signs_exact_final_artifacts_and_rejects_a_mismatched_key() {
    let dir = tempfile::tempdir().unwrap();
    let notes = dir.path().join("notes.txt");
    fs::write(&notes, "Release notes\n").unwrap();
    let mut c = fixtures::config("jvid", &fixtures::key(43));
    for p in &c.packages {
        fs::write(
            dir.path().join(format!("jvid-1.2.0-{}", p.suffix)),
            format!("payload:{}", p.suffix),
        )
        .unwrap();
    }
    let path = dir.path().join("config.json");
    let invoke = |c: &Config| {
        fs::write(&path, serde_json::to_vec(c).unwrap()).unwrap();
        Command::new(env!("CARGO_BIN_EXE_j-update-tool"))
            .arg("sign")
            .arg(&path)
            .arg("v1.2.0")
            .arg(&notes)
            .arg(dir.path())
            .env_remove("J_UPDATE_SIGNING_KEY_FILE")
            .env("J_UPDATE_SIGNING_KEY", "2b".repeat(32))
            .output()
            .unwrap()
    };
    let good = invoke(&c);
    assert!(
        good.status.success(),
        "{}",
        String::from_utf8_lossy(&good.stderr)
    );
    let bytes = fs::read(dir.path().join(&c.manifest_name)).unwrap();
    let sig = fs::read(dir.path().join(&c.signature_name)).unwrap();
    let m = metadata::verify(&bytes, &sig, &c).unwrap();
    assert_eq!(m.assets.len(), c.packages.len());
    for a in &m.assets {
        network::copy_verified(
            fs::read(dir.path().join(&a.name)).unwrap().as_slice(),
            std::io::sink(),
            a,
            &Default::default(),
            |_| {},
        )
        .unwrap();
    }
    c.public_key = fixtures::key(44);
    let bad = invoke(&c);
    assert!(!bad.status.success());
    assert!(!String::from_utf8_lossy(&bad.stderr).contains(&"2b".repeat(32)));
    assert_eq!(fs::read(dir.path().join(&c.manifest_name)).unwrap(), bytes);
    assert_eq!(fs::read(dir.path().join(&c.signature_name)).unwrap(), sig);
}

#[test]
fn pipeline_generator_without_production_configuration_never_emits_metadata() {
    for (seed, public) in [
        (None, None),
        (Some("2b".repeat(32)), None),
        (None, Some(fixtures::key(43))),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let notes = dir.path().join("notes.txt");
        fs::write(&notes, "Release notes").unwrap();
        let path = dir.path().join("config.json");
        let c = fixtures::config("jvid", &public.unwrap_or(fixtures::key(44)));
        if public.is_some() {
            fs::write(&path, serde_json::to_vec(&c).unwrap()).unwrap();
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_j-update-tool"));
        command
            .arg("sign")
            .arg(&path)
            .arg("1.2.0")
            .arg(&notes)
            .arg(dir.path())
            .env_remove("J_UPDATE_SIGNING_KEY")
            .env_remove("J_UPDATE_SIGNING_KEY_FILE");
        if let Some(seed) = seed {
            command.env("J_UPDATE_SIGNING_KEY", seed);
        }
        let result = command.output().unwrap();
        assert!(!result.status.success());
        assert!(!dir.path().join(c.manifest_name).exists());
        assert!(!dir.path().join(c.signature_name).exists());
        assert!(!String::from_utf8_lossy(&result.stderr).contains(&"2b".repeat(32)));
    }
}
