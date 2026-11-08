#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[path = "inno_tests.rs"]
    mod inno_tests;
    #[cfg(windows)]
    use crate::fixtures::TARGET;
    #[allow(unused_imports)]
    use crate::install::*;
    #[allow(dead_code)]
    const VERSION: &str = "0.1.0";
    #[allow(dead_code)]
    const REPOSITORY: &str = "jarradbanks/jvid";
    #[allow(dead_code)]
    const MANIFEST: &str = "jvid-update.json";
    #[allow(dead_code)]
    const SIGNATURE: &str = "jvid-update.json.sig";
    #[allow(dead_code)]
    fn config(key: &[u8; 32]) -> crate::Config {
        crate::fixtures::config("jvid", key)
    }
    fn validate(
        dir: &Path,
        plan: &Plan,
        key: &[u8; 32],
        target: &str,
        current: &str,
    ) -> Result<PathBuf> {
        let mut c = config(key);
        c.target = target.into();
        crate::install::validate(&c, dir, plan, current)
    }
    #[cfg(windows)]
    fn wait_for_parent(pid: u32, timeout: u32) -> Result<()> {
        let shell = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let expected = if pid == std::process::id() {
            std::env::current_exe().unwrap()
        } else {
            shell
        };
        crate::install::wait_for(pid, &expected, timeout)
    }
    #[cfg(windows)]
    fn run_helper_with_key(dir: &Path, key: &[u8; 32]) -> Result<()> {
        crate::install::run_helper(&config(key), VERSION, dir, &())
    }

    use crate::metadata::{Asset, Manifest};
    use ed25519_dalek::{Signer, SigningKey};
    use sha2::{Digest, Sha256};

    #[test]
    fn staged_payload_is_reverified_and_version_bound_before_execution() {
        let dir = tempfile::tempdir().unwrap();
        let payload = b"signed installer fixture";
        let asset = Asset {
            target: "x86_64-pc-windows-msvc".into(),
            distribution: "windows-installer".into(),
            name: "jvid-1.2.0-windows-x86_64-setup.exe".into(),
            size: payload.len() as u64,
            sha256: format!("{:x}", Sha256::digest(payload)),
        };
        let m = Manifest {
            schema: 1,
            repository: REPOSITORY.into(),
            version: "1.2.0".into(),
            notes: String::new(),
            assets: vec![asset.clone()],
        };
        let bytes = serde_json::to_vec(&m).unwrap();
        let key = SigningKey::from_bytes(&[43; 32]);
        fs::write(dir.path().join(MANIFEST), &bytes).unwrap();
        fs::write(dir.path().join(SIGNATURE), key.sign(&bytes).to_bytes()).unwrap();
        fs::write(dir.path().join(&asset.name), payload).unwrap();
        let mut plan = Plan {
            executable: dir.path().join("jvid-app.exe"),
            parent_pid: 42,
            tag: "v1.2.0".into(),
            report: dir.path().join("report"),
        };
        let verify = |p: &Plan, current| {
            validate(
                dir.path(),
                p,
                &key.verifying_key().to_bytes(),
                "x86_64-pc-windows-msvc",
                current,
            )
        };
        assert!(verify(&plan, "1.0.0").is_ok());
        assert!(verify(&plan, "1.2.0-beta.1").is_ok());
        assert!(verify(&plan, "1.2.0+local").is_err());
        assert!(verify(&plan, "1.2.0").is_err());
        assert!(verify(&plan, "2.0.0").is_err());
        plan.tag = "v1.3.0".into();
        assert!(verify(&plan, "1.0.0").is_err());
        plan.tag = "v1.2.0".into();
        assert!(
            validate(
                dir.path(),
                &plan,
                &key.verifying_key().to_bytes(),
                "aarch64-apple-darwin",
                "1.0.0"
            )
            .is_err()
        );
        // Tampering after the initial download check is caught on the helper path.
        fs::write(dir.path().join(&asset.name), b"corrupt").unwrap();
        assert!(verify(&plan, "1.0.0").is_err());
    }

    #[test]
    fn installer_failure_cancel_and_permission_errors_attempt_old_app_restart() {
        for code in [0, 1, 2, 5, 3010] {
            let mut restarted = false;
            let result = install_and_restart(
                || Ok(code),
                || {
                    restarted = true;
                    Ok(())
                },
            );
            assert!(restarted);
            assert_eq!(result.is_ok(), code == 0);
        }
        let mut restarted = false;
        assert!(
            install_and_restart(
                || Err("Access denied".into()),
                || {
                    restarted = true;
                    Ok(())
                }
            )
            .is_err()
        );
        assert!(restarted);
        assert!(
            install_and_restart(|| Ok(0), || Err("missing executable".into()))
                .unwrap_err()
                .contains("Launch the existing installation manually")
        );
        let err = install_and_restart(
            || Err("installer failed".into()),
            || Err("restart failed".into()),
        )
        .unwrap_err();
        assert!(err.contains("installer failed") && err.contains("restart failed"));
    }

    #[cfg(windows)]
    #[test]
    fn native_windows_parent_wait_accepts_exited_process_and_rejects_timeout_or_bad_pid() {
        let powershell = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let mut parent = Command::new(powershell)
            .args(["-NoProfile", "-NonInteractive", "-Command", "exit 0"])
            .spawn()
            .unwrap();
        let pid = parent.id();
        assert!(parent.wait().unwrap().success());
        drop(parent);
        wait_for_parent(pid, 1).unwrap();

        // This test process remains alive: a timeout must fail rather than install.
        let timeout = wait_for_parent(std::process::id(), 1).unwrap_err();
        assert!(
            timeout.contains("Update was deferred") && timeout.contains("ProcessNotTerminated")
        );
        // Conversion errors must not be mistaken for a process that has already exited.
        assert!(wait_for_parent(u32::MAX, 1).is_err());
    }

    /// Exercises native Windows waiting, .exe invocation, arguments, and relaunch using a
    /// harmless locally compiled fixture. This is deliberately not an Inno upgrade test.
    #[cfg(windows)]
    #[test]
    fn native_windows_helper_waits_runs_verified_fixture_and_restarts_after_success_or_failure() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("installed with spaces");
        fs::create_dir(&root).unwrap();
        let stage = dir.path().join("stage");
        fs::create_dir(&stage).unwrap();
        let source = dir.path().join("fixture.rs");
        fs::write(&source, r#"
            fn main() {
                if std::env::args().any(|a| a == "--wait") { std::thread::sleep(std::time::Duration::from_secs(2)); return; }
                let here = std::env::current_exe().unwrap();
                if let Some(dir) = std::env::args().find_map(|a| a.strip_prefix("/DIR=").map(String::from)) {
                    let root = std::path::Path::new(&dir);
                    let code: i32 = std::fs::read_to_string(root.join("exit-code")).unwrap().parse().unwrap();
                    if code == 0 { std::fs::write(root.join("version"), b"1.2.0").unwrap(); }
                    std::process::exit(code);
                }
                let root = here.parent().unwrap();
                let version = std::fs::read(root.join("version")).unwrap();
                std::fs::write(root.join("restarted"), version).unwrap();
            }
        "#).unwrap();
        let exe = root.join("jvid-app.exe");
        let compiled = Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(&exe)
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        fs::write(root.join("jvid-distribution"), b"windows-installer\n").unwrap();
        let name = "jvid-1.2.0-windows-x86_64-setup.exe";
        let payload = fs::read(&exe).unwrap();
        fs::write(stage.join(name), &payload).unwrap();
        let key = SigningKey::from_bytes(&[43; 32]);
        let manifest = Manifest {
            schema: 1,
            repository: REPOSITORY.into(),
            version: "1.2.0".into(),
            notes: "Fixture".into(),
            assets: vec![Asset {
                target: TARGET.into(),
                distribution: "windows-installer".into(),
                name: name.into(),
                size: payload.len() as u64,
                sha256: format!("{:x}", Sha256::digest(&payload)),
            }],
        };
        let bytes = serde_json::to_vec(&manifest).unwrap();
        fs::write(stage.join(MANIFEST), &bytes).unwrap();
        fs::write(stage.join(SIGNATURE), key.sign(&bytes).to_bytes()).unwrap();
        for (code, already_exited) in [(0, false), (5, false), (0, true), (5, true)] {
            fs::write(root.join("version"), b"1.0.0").unwrap();
            fs::write(root.join("exit-code"), code.to_string()).unwrap();
            let _ = fs::remove_file(root.join("restarted"));
            // Helper must wait for an actual Windows process, not a WSL PID.
            let mut parent = Command::new(&exe).arg("--wait").spawn().unwrap();
            let parent_pid = parent.id();
            if already_exited {
                assert!(parent.wait().unwrap().success());
            }
            // Drop the reaped process handle so Get-Process sees a genuinely absent PID.
            let mut parent = (!already_exited).then_some(parent);
            let plan = Plan {
                executable: exe.clone(),
                parent_pid,
                tag: "v1.2.0".into(),
                report: dir.path().join("result.txt"),
            };
            fs::write(stage.join("plan.json"), serde_json::to_vec(&plan).unwrap()).unwrap();
            assert_eq!(
                run_helper_with_key(&stage, &key.verifying_key().to_bytes()).is_ok(),
                code == 0
            );
            if let Some(parent) = parent.as_mut() {
                assert!(parent.try_wait().unwrap().is_some());
            }
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while !root.join("restarted").is_file() && std::time::Instant::now() < deadline {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            assert_eq!(
                fs::read_to_string(root.join("restarted")).unwrap(),
                if code == 0 { "1.2.0" } else { "1.0.0" }
            );
            assert!(exe.is_file());
        }
        // A changed staged executable cannot run, and the existing install stays usable.
        fs::write(stage.join(name), b"corrupt").unwrap();
        assert!(run_helper_with_key(&stage, &key.verifying_key().to_bytes()).is_err());
        assert!(exe.is_file());
    }
}
