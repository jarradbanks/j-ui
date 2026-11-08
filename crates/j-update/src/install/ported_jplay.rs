#[cfg(test)]
mod tests {
    #[cfg(windows)]
    use crate::fixtures::TARGET;
    #[allow(unused_imports)]
    use crate::install::*;
    #[allow(dead_code)]
    const VERSION: &str = "0.1.0";
    #[allow(dead_code)]
    const REPOSITORY: &str = "jarradbanks/jplay";
    #[allow(dead_code)]
    const MANIFEST: &str = "jplay-update.json";
    #[allow(dead_code)]
    const SIGNATURE: &str = "jplay-update.json.sig";
    #[allow(dead_code)]
    fn config(key: &[u8; 32]) -> crate::Config {
        crate::fixtures::config("jplay", key)
    }
    #[cfg(windows)]
    fn run_helper_with(
        dir: &Path,
        key: &[u8; 32],
        stop: impl FnOnce(&Path) -> Result<()>,
    ) -> Result<()> {
        struct Hook<F>(std::cell::RefCell<Option<F>>);
        impl<F: FnOnce(&Path) -> Result<()>> crate::install::HelperHooks for Hook<F> {
            fn before_install(&self, root: &Path) -> Result<Vec<std::ffi::OsString>> {
                self.0.borrow_mut().take().unwrap()(root)?;
                Ok(vec![])
            }
        }
        crate::install::run_helper(
            &config(key),
            VERSION,
            dir,
            &Hook(std::cell::RefCell::new(Some(stop))),
        )
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "isolated installer upgrade acceptance; requires JPLAY_INNO_TEST_ISCC"]
    fn windows_installer_upgrade_handoff() {
        use ed25519_dalek::{Signer, SigningKey};
        use sha2::{Digest, Sha256};
        let temp = tempfile::Builder::new()
            .prefix("jplay update acceptance ")
            .tempdir()
            .unwrap();
        let root = temp.path().join("installed app");
        let stage = temp.path().join("stage");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&stage).unwrap();
        let source = temp.path().join("fixture.rs");
        fs::write(&source, r##"#![windows_subsystem = "windows"]
fn main() {
    if std::env::args().any(|a| a == "--wait") { std::thread::sleep(std::time::Duration::from_secs(2)); }
    else {
        let root = std::env::current_exe().unwrap().parent().unwrap().to_owned();
        std::fs::write(root.join("restarted.txt"), std::fs::read(root.join("version.txt")).unwrap()).unwrap();
    }
}"##).unwrap();
        let fixture = temp.path().join("fixture.exe");
        let compiled = Command::new("rustc")
            .arg(&source)
            .args(["-C", "target-feature=+crt-static", "-o"])
            .arg(&fixture)
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        fs::copy(&fixture, root.join("jplay.exe")).unwrap();
        fs::write(root.join("jplay-distribution"), "windows-installer\n").unwrap();
        fs::write(root.join("version.txt"), "old").unwrap();
        fs::write(root.join("clip.mp4"), "saved clip").unwrap();
        fs::write(root.join("settings.toml"), "saved settings").unwrap();
        let new_version = temp.path().join("new-version.txt");
        fs::write(&new_version, "new").unwrap();
        let script = temp.path().join("fixture.iss");
        fs::write(
            &script,
            format!(
                r##"[Setup]
AppId=jplay-update-fixture-{}
AppName=jplay update fixture
AppVersion=999.0.0
DefaultDirName={}
PrivilegesRequired=lowest
Uninstallable=no
CreateUninstallRegKey=no
UsePreviousAppDir=no
CloseApplications=no
RestartApplications=no
OutputDir={}
OutputBaseFilename=jplay-999.0.0-windows-x64-setup
[Tasks]
Name: autostart; Description: "fixture startup choice"; Flags: unchecked
[Files]
Source: "{}"; DestDir: "{{app}}"; DestName: "jplay.exe"; Flags: ignoreversion
Source: "{}"; DestDir: "{{app}}"; DestName: "version.txt"; Flags: ignoreversion
"##,
                std::process::id(),
                root.display(),
                stage.display(),
                fixture.display(),
                new_version.display()
            ),
        )
        .unwrap();
        let output = Command::new(
            std::env::var_os("JPLAY_INNO_TEST_ISCC").expect("Set JPLAY_INNO_TEST_ISCC"),
        )
        .arg(&script)
        .output()
        .unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let name = "jplay-999.0.0-windows-x64-setup.exe";
        let bytes = fs::read(stage.join(name)).unwrap();
        let manifest = metadata::Manifest {
            schema: 1,
            repository: REPOSITORY.into(),
            version: "999.0.0".into(),
            notes: "Fixture".into(),
            assets: vec![metadata::Asset {
                target: TARGET.into(),
                distribution: "windows-installer".into(),
                name: name.into(),
                size: bytes.len() as u64,
                sha256: format!("{:x}", Sha256::digest(&bytes)),
            }],
        };
        let manifest = serde_json::to_vec(&manifest).unwrap();
        let key = SigningKey::from_bytes(&[59; 32]);
        fs::write(stage.join(MANIFEST), &manifest).unwrap();
        fs::write(stage.join(SIGNATURE), key.sign(&manifest).to_bytes()).unwrap();
        let mut parent = Command::new(root.join("jplay.exe"))
            .arg("--wait")
            .spawn()
            .unwrap();
        let plan = Plan {
            executable: root.join("jplay.exe"),
            parent_pid: parent.id(),
            tag: "v999.0.0".into(),
            report: temp.path().join("report.txt"),
        };
        fs::write(stage.join("plan.json"), serde_json::to_vec(&plan).unwrap()).unwrap();
        // An injected idle recorder keeps this fixture completely off the user's real control pipe.
        run_helper_with(&stage, &key.verifying_key().to_bytes(), |_| Ok(())).unwrap();
        parent.wait().unwrap();
        for _ in 0..100 {
            if root.join("restarted.txt").exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(
            fs::read_to_string(root.join("restarted.txt")).unwrap(),
            "new"
        );
        assert_eq!(
            fs::read_to_string(root.join("clip.mp4")).unwrap(),
            "saved clip"
        );
        assert_eq!(
            fs::read_to_string(root.join("settings.toml")).unwrap(),
            "saved settings"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    #[test]
    fn failures_and_reboot_requests_still_restore_the_app() {
        for code in [0, 1, 2, 3010] {
            let restarted = std::cell::Cell::new(false);
            let result = install_and_restart(
                || Ok(code),
                || {
                    restarted.set(true);
                    Ok(())
                },
            );
            assert!(restarted.get());
            assert_eq!(result.is_ok(), code == 0);
        }
        assert!(install_and_restart(|| Err("busy".into()), || Ok(())).is_err());
        assert!(install_and_restart(|| Ok(0), || Err("restart failed".into())).is_err());
    }
}
