#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use crate::fixtures::TARGET;
    use crate::metadata::*;
    #[allow(dead_code)]
    const REPOSITORY: &str = "jarradbanks/jplay";
    fn config() -> crate::Config {
        crate::fixtures::config("jplay", &crate::fixtures::key(37))
    }

    #[test]
    fn portable_external_and_wrong_platform_builds_never_self_install() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("jplay-app.exe");
        assert_eq!(distribution(&config(), &exe), "external");
        for p in &config().packages {
            let marker = p.distribution.as_str();
            std::fs::write(dir.path().join("jplay-distribution"), format!("{marker}\n")).unwrap();
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
        std::fs::write(dir.path().join("jplay-distribution"), "unknown\n").unwrap();
        assert_eq!(distribution(&config(), &exe), "external");
    }
}
