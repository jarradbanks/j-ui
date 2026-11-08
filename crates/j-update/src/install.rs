//! Inno owns the file transaction. The helper never terminates a process or replaces user data.
use crate::{
    Config, InstallPaths, Result, error, metadata,
    network::{Download, copy_verified},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub executable: PathBuf,
    pub parent_pid: u32,
    pub tag: String,
    pub report: PathBuf,
}

pub struct Prepared {
    download: Download,
    helper: PathBuf,
}

pub fn prepare(
    config: &Config,
    current: &str,
    download: Download,
    paths: &InstallPaths,
) -> Result<Prepared> {
    config.validate()?;
    let report = &paths.report;
    let executable = fs::canonicalize(&paths.executable).map_err(error)?;
    // Inno and PowerShell's Process.Path use ordinary drive paths, not verbatim prefixes.
    let executable = match executable.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(s) if s.starts_with("UNC\\") => {
            return Err("Network install paths are unsupported.".into());
        }
        Some(s) => PathBuf::from(s),
        None => executable,
    };
    installed_root(config, &executable)?;
    let helper = download.dir.path().join(&config.install.helper_name);
    fs::copy(&paths.helper, &helper).map_err(error)?;
    let plan = Plan {
        executable,
        parent_pid: std::process::id(),
        tag: download.update.tag.clone(),
        report: report.into(),
    };
    if let Some(parent) = report.parent() {
        fs::create_dir_all(parent).map_err(error)?;
    }
    fs::write(report, "Update prepared. Installation has not started.").map_err(error)?;
    validate(config, download.dir.path(), &plan, current)?;
    fs::write(
        download.dir.path().join("plan.json"),
        serde_json::to_vec(&plan).map_err(error)?,
    )
    .map_err(error)?;
    Ok(Prepared { download, helper })
}

fn installed_root<'a>(config: &Config, executable: &'a Path) -> Result<&'a Path> {
    let root = executable.parent().ok_or("Missing install directory.")?;
    if !cfg!(windows)
        || !metadata::self_update(&config.target, &config.distribution)
        || !config
            .install
            .executables
            .iter()
            .any(|n| executable.file_name() == Some(n.as_ref()))
        || metadata::distribution(config, executable) != "windows-installer"
    {
        return Err("Only installed Windows builds can install an update.".into());
    }
    Ok(root)
}

impl Prepared {
    /// Launch after the app has durably saved its restart checkpoint. The child waits for exit.
    pub fn launch(self) -> Result<()> {
        let mut command = Command::new(&self.helper);
        command.arg(self.download.dir.path());
        hide(&mut command);
        command.spawn().map_err(error)?;
        let _ = self.download.dir.keep();
        Ok(())
    }
}

pub fn validate(config: &Config, dir: &Path, plan: &Plan, current: &str) -> Result<PathBuf> {
    let m = metadata::verify_files(config, dir)?;
    if metadata::stable_version(&plan.tag)?.to_string() != m.version
        || metadata::stable_version(&m.version)?
            .cmp_precedence(&semver::Version::parse(current).map_err(error)?)
            .is_le()
    {
        return Err("Refusing a mismatched version or downgrade.".into());
    }
    let asset = m
        .asset(&config.target, "windows-installer")
        .ok_or("No compatible Windows installer.")?;
    let path = dir.join(&asset.name);
    copy_verified(
        fs::File::open(&path).map_err(error)?,
        std::io::sink(),
        asset,
        &AtomicBool::new(false),
        |_| {},
    )?;
    Ok(path)
}

fn hide(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    #[cfg(not(windows))]
    let _ = command;
}

const WAIT: &str = r#"
$ErrorActionPreference = 'Stop'
try { $process = Get-Process -Id ([int]$env:J_UPDATE_PID) -ErrorAction Stop }
catch {
    if ($_.FullyQualifiedErrorId -eq 'NoProcessFoundForGivenId,Microsoft.PowerShell.Commands.GetProcessCommand') { exit 0 }
    throw
}
if ($process.Path -ne $env:J_UPDATE_EXE) { throw 'Process identity changed; update deferred' }
$process | Wait-Process -Timeout ([int]$env:J_UPDATE_TIMEOUT)
exit 0
"#;

const CHECK_IDLE: &str = r#"
$ErrorActionPreference = 'Stop'
$names = $env:J_UPDATE_NAMES -split ','
$processes = Get-Process -Name $names -ErrorAction SilentlyContinue
foreach ($process in $processes) {
    if (-not $process.Path) { throw 'Cannot inspect an application process; update deferred' }
    if ((Split-Path -Parent $process.Path) -eq $env:J_UPDATE_ROOT) {
        throw 'An application is still running from this installation; update deferred'
    }
}
exit 0
"#;

fn powershell(script: &str, vars: &[(&str, &std::ffi::OsStr)]) -> Result<()> {
    let shell = PathBuf::from(std::env::var_os("SystemRoot").ok_or("No SystemRoot.")?)
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let mut command = Command::new(shell);
    command.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    for (name, value) in vars {
        command.env(name, value);
    }
    hide(&mut command);
    let output = command.output().map_err(error)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Update was deferred: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn wait(pid: u32, executable: &Path) -> Result<()> {
    wait_for(pid, executable, 120)
}

fn wait_for(pid: u32, executable: &Path, seconds: u32) -> Result<()> {
    if pid == 0 || pid > i32::MAX as u32 {
        return Err("Invalid parent process ID.".into());
    }
    powershell(
        WAIT,
        &[
            ("J_UPDATE_PID", std::ffi::OsStr::new(&pid.to_string())),
            ("J_UPDATE_EXE", executable.as_os_str()),
            (
                "J_UPDATE_TIMEOUT",
                std::ffi::OsStr::new(&seconds.to_string()),
            ),
        ],
    )
}

fn check_idle(config: &Config, root: &Path) -> Result<()> {
    let names = config.install.process_names.join(",");
    powershell(
        CHECK_IDLE,
        &[
            ("J_UPDATE_ROOT", root.as_os_str()),
            ("J_UPDATE_NAMES", std::ffi::OsStr::new(&names)),
        ],
    )
}

pub fn install_and_restart(
    install: impl FnOnce() -> Result<i32>,
    restart: impl FnOnce() -> Result<()>,
) -> Result<()> {
    let installed = install().and_then(|code| match code {
        0 => Ok(()),
        3010 => Err("Windows needs to restart to complete this update.".into()),
        n => Err(format!(
            "Installer exited with code {n}; see the installer log."
        )),
    });
    let restarted = restart();
    match (installed, restarted) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(e), Ok(())) => Err(e),
        (Ok(()), Err(e)) => Err(format!(
            "Installation finished, but automatic restart failed: {e}. Launch the existing installation manually."
        )),
        (Err(e), Err(r)) => Err(format!("{e} Restart also failed: {r}")),
    }
}

/// App-owned actions performed after the parent exits, before the final idle and payload checks.
/// jclip uses this to request recorder ShutdownIfIdle and preserve its autostart task.
/// The default has no companion work. A hook must never force-terminate active work.
pub trait HelperHooks {
    fn before_install(&self, _root: &Path) -> Result<Vec<std::ffi::OsString>> {
        Ok(Vec::new())
    }
}
impl HelperHooks for () {}

/// Called by a tiny app helper with its own pinned config/current version, never stage config.
pub fn run_helper(
    config: &Config,
    current: &str,
    dir: &Path,
    hooks: &impl HelperHooks,
) -> Result<()> {
    config.validate()?;
    let plan: Plan = serde_json::from_slice(&fs::read(dir.join("plan.json")).map_err(error)?)
        .map_err(|_| "Invalid update plan.")?;
    let root = installed_root(config, &plan.executable)?;
    if let Err(e) = validate(config, dir, &plan, current) {
        let _ = fs::write(&plan.report, &e);
        return Err(e);
    }
    // Identity/wait failures never start another app while its parent may still be running.
    if let Err(e) = wait(plan.parent_pid, &plan.executable) {
        let _ = fs::write(&plan.report, &e);
        return Err(e);
    }
    let result = install_and_restart(
        || {
            let result: Result<i32> = (|| {
                let args = hooks.before_install(root)?;
                check_idle(config, root)?;
                // Recheck install markers too: another operation may have changed the tree.
                installed_root(config, &plan.executable)?;
                let payload = validate(config, dir, &plan, current)?;
                let mut installer = Command::new(payload);
                installer
                    .args([
                        "/VERYSILENT",
                        "/SUPPRESSMSGBOXES",
                        "/NOCANCEL",
                        "/NORESTART",
                        "/RESTARTEXITCODE=3010",
                        "/NOCLOSEAPPLICATIONS",
                        "/NORESTARTAPPLICATIONS",
                        "/SP-",
                    ])
                    .arg(format!("/DIR={}", root.display()))
                    .arg(format!(
                        "/LOG={}",
                        plan.report.with_extension("installer.log").display()
                    ))
                    .args(args);
                hide(&mut installer);
                Ok(installer.status().map_err(error)?.code().unwrap_or(-1))
            })();
            let message = match &result {
                Ok(code) => format!("Installer exit code: {code}."),
                Err(e) => e.clone(),
            };
            fs::write(&plan.report, message).map_err(error)?;
            result
        },
        || {
            let mut app = Command::new(&plan.executable);
            hide(&mut app);
            app.spawn().map(|_| ()).map_err(error)
        },
    );
    fs::write(
        &plan.report,
        match &result {
            Ok(()) => "Installer completed and restart was requested.".to_owned(),
            Err(e) => e.clone(),
        },
    )
    .map_err(error)?;
    // Never recursively delete a caller-provided directory. Temp policy cleans the running helper.
    result
}

/// Optional installed-instance mutex. Use the existing installer mutex name across upgrades.
#[cfg(windows)]
#[allow(unsafe_code)]
pub mod instance {
    use windows::{
        Win32::{
            Foundation::{CloseHandle, HANDLE},
            System::Threading::CreateMutexW,
        },
        core::PCWSTR,
    };
    pub struct Guard(HANDLE);
    impl Guard {
        pub fn hold(
            config: &crate::Config,
            executable: &std::path::Path,
            name: &str,
        ) -> crate::Result<Option<Self>> {
            if crate::metadata::distribution(config, executable) != "windows-installer" {
                return Ok(None);
            }
            if name.contains('\0') {
                return Err("Invalid instance mutex name.".into());
            }
            let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
            // SAFETY: the NUL-terminated name lives through the call; Guard owns the returned handle.
            unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }
                .map(|h| Some(Self(h)))
                .map_err(crate::error)
        }
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            // SAFETY: this owned handle is closed exactly once.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}

#[cfg(test)]
mod ported_jplay;

#[cfg(test)]
mod ported_jvid;

#[cfg(test)]
mod ported_jvec;
