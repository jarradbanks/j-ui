//! Release utility: configuration is a trusted local file, secrets never CLI arguments.
use j_update::{Config, Result, credentials, metadata, release};
use std::{fs, io::Read, path::Path};
use zeroize::Zeroizing;
fn config(path: &str) -> Result<Config> {
    let c: Config =
        serde_json::from_slice(&fs::read(path).map_err(|_| "Cannot read updater configuration.")?)
            .map_err(|_| "Invalid updater configuration.")?;
    c.validate()?;
    Ok(c)
}
fn seed() -> Result<Zeroizing<[u8; 32]>> {
    if let Some(path) = std::env::var_os("J_UPDATE_SIGNING_KEY_FILE") {
        let bytes = Zeroizing::new(fs::read(path).map_err(|_| "Cannot read signing key file.")?);
        return Ok(Zeroizing::new(
            bytes
                .as_slice()
                .try_into()
                .map_err(|_| "Signing key file must contain a 32-byte seed.")?,
        ));
    }
    let hex = Zeroizing::new(
        std::env::var("J_UPDATE_SIGNING_KEY")
            .map_err(|_| "Set J_UPDATE_SIGNING_KEY_FILE or J_UPDATE_SIGNING_KEY.")?,
    );
    Ok(Zeroizing::new(
        metadata::unhex(hex.trim()).map_err(|_| "Invalid signing key.")?,
    ))
}
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [op, name, public] if op == "init-key" => release::init_key_in_store(Path::new(public), name),
        [op, public] if op == "init-key" => release::init_key(Path::new(public), &*seed()?),
        [op, c, version, notes, dir] if op == "sign" => release::generate(&config(c)?, version,
            &fs::read_to_string(notes).map_err(|_| "Cannot read release notes.")?, Path::new(dir), &*seed()?),
        [op, c, version, dir] if op == "verify" => release::verify(&config(c)?, version, Path::new(dir)),
        [op, c, current] if op == "check" => {
            let update = j_update::Client::new(config(c)?)?.check(current)?;
            println!("{}",update.map_or_else(|| "No newer release.".into(), |u| format!("Available: {}",u.version))); Ok(())
        }
        [op, c, name] if op == "store-token" => {
            let mut value = Zeroizing::new(String::new());
            std::io::stdin().take(2562).read_to_string(&mut value).map_err(|_| "Cannot read credential.")?;
            let value = Zeroizing::new(value.trim().to_owned());
            credentials::validate_token(&value)?;
            let client = j_update::Client::new(config(c)?)?.with_token_provider({
                let value=value.clone(); move || j_update::Token::new(value.to_string()).map(Some)
            });
            client.validate_access()?;
            credentials::write(name, value.as_bytes())
        }
        _ => Err("Usage: j-update-tool init-key PUBLIC_FILE | sign CONFIG VERSION NOTES DIRECTORY | verify CONFIG VERSION DIRECTORY | check CONFIG CURRENT_VERSION | store-token CONFIG CREDENTIAL_NAME (stdin)".into()),
    }
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
