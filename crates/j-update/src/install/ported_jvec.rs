#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use crate::install::*;
    #[allow(dead_code)]
    const VERSION: &str = "0.1.0";
    #[allow(dead_code)]
    const REPOSITORY: &str = "jarradbanks/jvec-desktop";
    #[allow(dead_code)]
    const MANIFEST: &str = "vectorize-update.json";
    #[allow(dead_code)]
    const SIGNATURE: &str = "vectorize-update.json.sig";
    #[allow(dead_code)]
    fn config(key: &[u8; 32]) -> crate::Config {
        crate::fixtures::config("jvec", key)
    }
    #[cfg(windows)]
    fn installed_root(exe: &Path) -> Result<&Path> {
        crate::install::installed_root(&config(&crate::fixtures::key(59)), exe)
    }
    #[cfg(windows)]
    fn validate(dir: &Path, plan: &Plan, key: &[u8; 32], current: &str) -> Result<PathBuf> {
        crate::install::validate(&config(key), dir, plan, current)
    }
    #[cfg(windows)]
    fn check_idle(root: &Path) -> Result<()> {
        crate::install::check_idle(&config(&crate::fixtures::key(59)), root)
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
    #[ignore = "isolated installer upgrade acceptance; requires VECTORIZE_INNO_TEST_ISCC"]
    fn windows_installer_upgrade_handoff() {
        use ed25519_dalek::SigningKey;
        let temp = tempfile::Builder::new()
            .prefix("vectorize update acceptance ")
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
        fs::copy(&fixture, root.join("vectorize-desktop.exe")).unwrap();
        fs::write(root.join("vectorize-distribution"), "windows-installer\n").unwrap();
        fs::write(root.join("version.txt"), "old").unwrap();
        fs::write(root.join("project.vec"), "saved project").unwrap();
        fs::write(root.join("recovery.vec"), "recovery edits").unwrap();
        fs::write(root.join("artwork.png"), "private artwork").unwrap();
        fs::write(root.join("settings.toml"), "saved settings").unwrap();
        let payload = temp.path().join("payload");
        fs::create_dir(&payload).unwrap();
        fs::copy(&fixture, payload.join("jvec.exe")).unwrap();
        let compiler =
            std::env::var_os("VECTORIZE_INNO_TEST_ISCC").expect("Set VECTORIZE_INNO_TEST_ISCC");
        let recipe = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/jvec/windows.iss");
        let compile = |version: &str| {
            Command::new(&compiler)
                .arg("/DFixture")
                .arg(format!("/DIdentity=VectorizeFixture{}", std::process::id()))
                .arg(format!("/DVersion={version}"))
                .arg(format!("/DPayload={}", payload.display()))
                .arg(format!("/DOutput={}", stage.display()))
                .arg(&recipe)
                .output()
                .unwrap()
        };
        fs::write(payload.join("version.txt"), "old").unwrap();
        assert!(compile("998.0.0").status.success());
        assert!(
            Command::new(stage.join("vectorize-998.0.0-windows-x64-setup.exe"))
                .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-"])
                .arg(format!("/DIR={}", root.display()))
                .status()
                .unwrap()
                .success()
        );
        assert_eq!(
            fs::read(root.join("jvec.exe")).unwrap(),
            fs::read(root.join("vectorize-desktop.exe")).unwrap()
        );
        assert_eq!(installed_root(&root.join("jvec.exe")).unwrap(), root);
        let mut renamed = Command::new(root.join("jvec.exe"))
            .arg("--wait")
            .spawn()
            .unwrap();
        assert!(check_idle(&root).is_err());
        renamed.wait().unwrap();
        fs::write(payload.join("version.txt"), "new").unwrap();
        let output = compile("999.0.0");
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let name = "vectorize-999.0.0-windows-x64-setup.exe";
        let key = SigningKey::from_bytes(&[59; 32]);
        crate::release::generate(
            &config(&key.verifying_key().to_bytes()),
            "999.0.0",
            "Fixture",
            &stage,
            &key.to_bytes(),
        )
        .unwrap();
        // This is the same signature/hash/version verification used before upload.
        let plan_fixture = Plan {
            executable: root.join("vectorize-desktop.exe"),
            parent_pid: 1,
            tag: "v999.0.0".into(),
            report: temp.path().join("report.txt"),
        };
        validate(
            &stage,
            &plan_fixture,
            &key.verifying_key().to_bytes(),
            VERSION,
        )
        .unwrap();
        let mut parent = Command::new(root.join("vectorize-desktop.exe"))
            .arg("--wait")
            .spawn()
            .unwrap();
        assert!(check_idle(&root).is_err());
        let plan = Plan {
            executable: root.join("vectorize-desktop.exe"),
            parent_pid: parent.id(),
            tag: "v999.0.0".into(),
            report: temp.path().join("report.txt"),
        };
        fs::write(stage.join("plan.json"), serde_json::to_vec(&plan).unwrap()).unwrap();
        // The production helper and production installer copy/guard paths run in temp.
        run_helper_with(&stage, &key.verifying_key().to_bytes(), check_idle).unwrap();
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
            fs::read_to_string(root.join("settings.toml")).unwrap(),
            "saved settings"
        );
        for (name, expected) in [
            ("project.vec", "saved project"),
            ("recovery.vec", "recovery edits"),
            ("artwork.png", "private artwork"),
        ] {
            assert_eq!(fs::read_to_string(root.join(name)).unwrap(), expected);
        }
        // The final installer refuses to replace a portable tree even when called silently.
        let portable = temp.path().join("portable app");
        fs::create_dir(&portable).unwrap();
        fs::write(portable.join("vectorize-portable"), "windows-portable").unwrap();
        fs::write(portable.join("artwork.png"), "private artwork").unwrap();
        let log = temp.path().join("portable.log");
        let mut rejected = Command::new(stage.join(name))
            .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-"])
            .arg(format!("/DIR={}", portable.display()))
            .arg(format!("/LOG={}", log.display()))
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let code = loop {
            if let Some(code) = rejected.try_wait().unwrap() {
                break code;
            }
            if std::time::Instant::now() > deadline {
                rejected.kill().unwrap();
                panic!("Portable rejection hung");
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        };
        assert!(!code.success());
        assert!(
            fs::read_to_string(log)
                .unwrap()
                .contains("Refusing portable")
        );
        assert_eq!(
            fs::read_to_string(portable.join("artwork.png")).unwrap(),
            "private artwork"
        );
        assert!(!portable.join("vectorize-desktop.exe").exists());
        fs::write(root.join("vectorize-portable"), "portable").unwrap();
        assert!(installed_root(&root.join("vectorize-desktop.exe")).is_err());
        fs::remove_file(root.join("vectorize-portable")).unwrap();
        // Changed bytes are rejected immediately before execution, and the original app returns.
        fs::write(stage.join(name), b"tampered").unwrap();
        assert!(run_helper_with(&stage, &key.verifying_key().to_bytes(), check_idle).is_err());
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    #[cfg(windows)]
    #[test]
    fn native_parent_wait_handles_gone_identity_timeout_and_invalid_pid() {
        let shell = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let mut command = Command::new(&shell);
        command.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 4"]);
        hide(&mut command);
        let mut child = command.spawn().unwrap();
        assert!(wait_for(child.id(), &shell.with_file_name("wrong.exe"), 1).is_err());
        assert!(wait_for(child.id(), &shell, 1).is_err());
        child.wait().unwrap();
        wait_for(child.id(), &shell, 1).unwrap();
        assert!(wait_for(0, &shell, 1).is_err());
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
