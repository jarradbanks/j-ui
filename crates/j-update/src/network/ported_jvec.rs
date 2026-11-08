use super::*;
use crate::fixtures::TARGET;
const REPOSITORY: &str = "jarradbanks/jvec-desktop";
const MANIFEST: &str = "vectorize-update.json";
const SIGNATURE: &str = "vectorize-update.json.sig";
fn config() -> crate::Config {
    crate::fixtures::config("jvec", &crate::fixtures::key(47))
}
#[allow(dead_code)]
fn verify(bytes: &[u8], sig: &[u8], key: &[u8; 32]) -> crate::Result<metadata::Manifest> {
    let mut config = config();
    config.public_key = *key;
    metadata::verify(bytes, sig, &config)
}

use ed25519_dalek::{Signer, SigningKey};
use std::{net::TcpListener, thread::JoinHandle};

fn server(responses: Vec<(u16, Vec<u8>, Option<String>)>) -> (String, JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let thread = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, body, location) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            requests.push(String::from_utf8(request).unwrap());
            let redirect = location
                .map(|l| format!("Location: {l}\r\n"))
                .unwrap_or_default();
            write!(stream, "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n{redirect}\r\n", body.len()).unwrap();
            stream.write_all(&body).unwrap();
        }
        requests
    });
    (base, thread)
}

fn signed() -> (metadata::Manifest, Vec<u8>, Vec<u8>, [u8; 32]) {
    let key = SigningKey::from_bytes(&[47; 32]);
    let m = metadata::Manifest {
        schema: 1,
        repository: REPOSITORY.into(),
        version: "1.2.0".into(),
        notes: "Private release notes".into(),
        assets: vec![Asset {
            target: TARGET.into(),
            distribution: "windows-installer".into(),
            name: "vectorize-1.2.0-windows-x64-setup.exe".into(),
            size: 9,
            sha256: format!("{:x}", Sha256::digest(b"installer")),
        }],
    };
    let bytes = serde_json::to_vec(&m).unwrap();
    let signature = key.sign(&bytes).to_bytes().to_vec();
    (m, bytes, signature, key.verifying_key().to_bytes())
}

fn listing(m: &metadata::Manifest) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!([
        {"tag_name":"v99.0.0","draft":true,"prerelease":false,"assets":[]},
        {"tag_name":"sources-v99.0.0","draft":false,"prerelease":false,"assets":[]},
        {"tag_name":"v2.0.0-beta.1","draft":false,"prerelease":true,"assets":[]},
        {"tag_name":"v1.2.0","draft":false,"prerelease":false,"assets":[
            {"id":10,"name":MANIFEST,"state":"uploaded"},
            {"id":11,"name":SIGNATURE,"state":"uploaded"},
            {"id":12,"name":m.assets[0].name,"state":"uploaded"}
        ]}
    ]))
    .unwrap()
}

#[test]
fn private_api_asset_ids_are_used_for_signed_metadata_and_verified_installer() {
    let (m, manifest, signature, _key) = signed();
    let (base, thread) = server(vec![
        (404, vec![], None),
        (200, listing(&m), None),
        (404, vec![], None),
        (200, manifest, None),
        (404, vec![], None),
        (200, signature, None),
        (404, vec![], None),
        (200, b"installer".to_vec(), None),
    ]);
    let client = Client::at(config(), base, false)
        .unwrap()
        .with_token_provider(|| Token::new("fixture_token".into()).map(Some));
    let update = client.check("1.0.0").unwrap().unwrap();
    assert_eq!(update.version, "1.2.0");
    let download = client
        .download(update, &AtomicBool::new(false), |_| {})
        .unwrap();
    assert_eq!(
        std::fs::read(download.dir.path().join(&m.assets[0].name)).unwrap(),
        b"installer"
    );
    let requests = thread.join().unwrap();
    assert_eq!(requests.len(), 8);
    for request in requests.iter().step_by(2) {
        assert!(!request.to_ascii_lowercase().contains("authorization:"));
    }
    let authenticated: Vec<_> = requests.iter().skip(1).step_by(2).collect();
    for (request, path) in authenticated.iter().zip([
        "/releases?",
        "/releases/assets/10",
        "/releases/assets/11",
        "/releases/assets/12",
    ]) {
        assert!(request.contains(path));
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer fixture_token")
        );
    }
    assert!(authenticated[1].contains("application/octet-stream"));
}

#[test]
fn even_same_host_redirects_do_not_receive_the_github_credential() {
    let (base, thread) = server(vec![
        (401, vec![], None),
        (302, Vec::new(), Some("/cdn".into())),
        (200, b"payload".to_vec(), None),
    ]);
    let client = Client::at(config(), base, false)
        .unwrap()
        .with_token_provider(|| Token::new("fixture_token".into()).map(Some));
    assert_eq!(client.bytes("/asset", true, 100).unwrap(), b"payload");
    let requests = thread.join().unwrap();
    assert!(!requests[0].to_ascii_lowercase().contains("authorization:"));
    assert!(
        requests[1]
            .to_ascii_lowercase()
            .contains("authorization: bearer fixture_token")
    );
    assert!(!requests[2].to_ascii_lowercase().contains("authorization:"));
    assert!(!requests[2].contains("fixture_token"));
}

#[test]
fn invalid_signatures_and_missing_update_files_never_authorize_a_download() {
    let (m, manifest, mut signature, _key) = signed();
    signature[0] ^= 1;
    let (base, thread) = server(vec![
        (200, listing(&m), None),
        (200, manifest, None),
        (200, signature, None),
    ]);
    let client = Client::at(config(), base, false).unwrap();
    assert!(client.check("1.0.0").is_err());
    assert_eq!(thread.join().unwrap().len(), 3);
    let (base, thread) = server(vec![(
        200,
        br#"[{"tag_name":"v1.2.0","draft":false,"prerelease":false,"assets":[]}]"#.to_vec(),
        None,
    )]);
    assert!(
        Client::at(config(), base, false)
            .unwrap()
            .check("1.0.0")
            .is_err()
    );
    thread.join().unwrap();
}

#[test]
fn equal_versions_downgrades_and_api_auth_errors_are_handled_without_secret_output() {
    let (m, _, _, _key) = signed();
    for current in ["1.2.0", "1.2.0+local", "2.0.0"] {
        let (base, thread) = server(vec![(200, listing(&m), None)]);
        assert!(
            Client::at(config(), base, false)
                .unwrap()
                .check(current)
                .unwrap()
                .is_none()
        );
        thread.join().unwrap();
    }
    let (base, thread) = server(vec![(
        401,
        b"fixture_token must never be included in errors".to_vec(),
        None,
    )]);
    let error = Client::at(config(), base, false)
        .unwrap()
        .validate_access()
        .unwrap_err();
    assert!(error.contains("401"));
    assert!(!error.contains("fixture_token"));
    thread.join().unwrap();
}

#[test]
fn cancellation_corruption_truncation_and_oversized_payloads_fail_verification() {
    let (m, _, _, _) = signed();
    let asset = &m.assets[0];
    for bytes in [b"corrupted".as_slice(), b"installe", b"installer-extra"] {
        assert!(
            copy_verified(
                bytes,
                std::io::sink(),
                asset,
                &AtomicBool::new(false),
                |_| {}
            )
            .is_err()
        );
    }
    assert!(
        copy_verified(
            b"installer".as_slice(),
            std::io::sink(),
            asset,
            &AtomicBool::new(true),
            |_| {}
        )
        .is_err()
    );
}

#[test]
fn wrong_target_and_signed_tag_mismatch_are_errors_instead_of_current() {
    for target in [TARGET, "aarch64-pc-windows-msvc"] {
        let (mut m, _, _, _key) = signed();
        if target == TARGET {
            m.version = "1.3.0".into();
            m.assets[0].name = "vectorize-1.3.0-windows-x64-setup.exe".into();
        }
        let signing = SigningKey::from_bytes(&[47; 32]);
        let bytes = serde_json::to_vec(&m).unwrap();
        let sig = signing.sign(&bytes).to_bytes().to_vec();
        let (base, thread) = server(vec![
            (200, listing(&m), None),
            (200, bytes, None),
            (200, sig, None),
        ]);
        let client = Client::at(config(), base, false).unwrap();
        assert!(
            client
                .check_target("1.0.0", target, "windows-installer")
                .is_err()
        );
        thread.join().unwrap();
    }
}
