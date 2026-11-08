use super::ported_jplay::{listing, server, signed};
use super::*;
use crate::fixtures::{config, key};
use std::sync::atomic::AtomicUsize;

#[test]
fn public_releases_are_anonymous_and_never_call_the_provider() {
    let (mut m, _, _, _) = signed();
    m.repository = "jarradbanks/jclip-releases".into();
    let bytes = serde_json::to_vec(&m).unwrap();
    use ed25519_dalek::Signer;
    let sig = ed25519_dalek::SigningKey::from_bytes(&[47; 32])
        .sign(&bytes)
        .to_bytes()
        .to_vec();
    let (base, task) = server(vec![
        (200, listing(&m), None),
        (200, bytes, None),
        (200, sig, None),
        (200, b"installer".to_vec(), None),
    ]);
    let mut c = config("jplay", &key(47));
    c.releases_repo = m.repository.clone();
    let client =
        Client::at(c, base, false)
            .unwrap()
            .with_token_provider(|| -> Result<Option<Token>> {
                panic!("public updates must not read credentials")
            });
    let available = client.check("1.0.0").unwrap().unwrap();
    assert!(
        available
            .release_url(client.config())
            .unwrap()
            .contains("jclip-releases")
    );
    client
        .download(available, &AtomicBool::new(false), |_| {})
        .unwrap();
    for r in task.join().unwrap() {
        assert!(r.contains("/repos/jarradbanks/jclip-releases/"));
        assert!(!r.to_lowercase().contains("authorization:"));
    }
}

#[test]
fn legacy_private_repo_retries_each_api_request_once_then_strips_auth_on_redirect() {
    let (m, bytes, sig, _) = signed();
    let mut responses = Vec::new();
    for body in [listing(&m), bytes, sig] {
        responses.push((404, vec![], None));
        responses.push((200, body, None));
    }
    responses.extend([
        (401, vec![], None),
        (302, vec![], Some("/cdn?secret_fixture".into())),
        (200, b"installer".to_vec(), None),
    ]);
    let (base, task) = server(responses);
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let client = Client::at(config("jplay", &key(47)), base, false)
        .unwrap()
        .with_token_provider(move || {
            count.fetch_add(1, Ordering::Relaxed);
            Token::new("fixture_token".into()).map(Some)
        });
    let available = client.check("1.0.0").unwrap().unwrap();
    client
        .download(available, &AtomicBool::new(false), |_| {})
        .unwrap();
    let requests = task.join().unwrap();
    assert_eq!(requests.len(), 9);
    assert_eq!(calls.load(Ordering::Relaxed), 4);
    for (i, r) in requests.iter().enumerate() {
        assert_eq!(
            r.to_lowercase()
                .contains("authorization: bearer fixture_token"),
            i % 2 == 1
        );
    }
}

#[test]
fn provider_and_malicious_http_errors_do_not_echo_secrets_or_urls() {
    let (base, task) = server(vec![(404, b"fixture_token".to_vec(), None)]);
    let client = Client::at(config("jplay", &key(47)), base, false)
        .unwrap()
        .with_token_provider(|| Err("fixture_token".into()));
    assert_eq!(
        client.validate_access().unwrap_err(),
        "GitHub credential provider failed."
    );
    task.join().unwrap();
    let (base, task) = server(vec![(200, b"fixture_token is invalid JSON".to_vec(), None)]);
    let client = Client::at(config("jplay", &key(47)), base, false).unwrap();
    assert_eq!(
        client.check("1.0.0").unwrap_err(),
        "Invalid GitHub release listing."
    );
    task.join().unwrap();
    assert!(
        !format!("{:?}", Token::new("fixture_token".into()).unwrap()).contains("fixture_token")
    );
}

#[test]
fn insecure_or_userinfo_redirects_and_redirect_loops_are_rejected() {
    for destination in [
        "https://fixture_token@example.com/file",
        "file:///fixture_token",
    ] {
        let (base, task) = server(vec![(302, vec![], Some(destination.into()))]);
        let client = Client::at(config("jplay", &key(47)), base, false).unwrap();
        let e = client.bytes("/asset", true, 64).unwrap_err();
        assert!(!e.contains("fixture_token"));
        task.join().unwrap();
    }
    let (base, task) = server(
        (0..6)
            .map(|_| (302, vec![], Some("/again".into())))
            .collect(),
    );
    let client = Client::at(config("jplay", &key(47)), base, false).unwrap();
    assert!(
        client
            .bytes("/asset", true, 64)
            .unwrap_err()
            .contains("Too many")
    );
    task.join().unwrap();
}

#[test]
fn bounded_listing_rejects_ten_full_pages_and_metadata_caps() {
    let body = serde_json::to_vec(&vec![
        serde_json::json!({"tag_name":"v0.1.0","draft":true,"prerelease":false});
        100
    ])
    .unwrap();
    let (base, task) = server((0..10).map(|_| (200, body.clone(), None)).collect());
    assert!(
        Client::at(config("jplay", &key(47)), base, false)
            .unwrap()
            .check("1.0.0")
            .unwrap_err()
            .contains("limit")
    );
    assert_eq!(task.join().unwrap().len(), 10);
    let (base, task) = server(vec![(200, vec![0; 65], None)]);
    assert!(
        Client::at(config("jplay", &key(47)), base, false)
            .unwrap()
            .bytes("/signature", true, 64)
            .is_err()
    );
    task.join().unwrap();
}

#[test]
fn strict_newest_release_missing_target_does_not_fall_back_to_an_older_release() {
    let (mut m, _, _, _) = signed();
    let releases = serde_json::json!([
        {"tag_name":"v2.0.0","draft":false,"prerelease":false,"assets":[]},
        {"tag_name":"v1.2.0","draft":false,"prerelease":false,"assets":[]}
    ]);
    let (base, task) = server(vec![(200, serde_json::to_vec(&releases).unwrap(), None)]);
    assert!(
        Client::at(config("jplay", &key(47)), base, false)
            .unwrap()
            .check("1.0.0")
            .is_err()
    );
    task.join().unwrap();
    // Unsigned presentation fields cannot change paths, target, version, or payload identity.
    let (_, bytes, sig, _) = signed();
    m.assets[0].name = "../escape.exe".into();
    let update = Available {
        version: m.version,
        notes: m.notes,
        asset: m.assets.remove(0),
        asset_id: 12,
        tag: "v1.2.0".into(),
        manifest: bytes,
        signature: sig,
    };
    let client = Client::at(
        config("jplay", &key(47)),
        "http://127.0.0.1:1".into(),
        false,
    )
    .unwrap();
    assert!(
        client
            .download(update, &AtomicBool::new(false), |_| {})
            .is_err()
    );
}
