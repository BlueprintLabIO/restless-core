use sha2::{Digest as _, Sha256};

#[test]
fn sprint_31_identity_corpus_is_frozen_and_source_complete() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/dogfood/company-identity/s31-corpus.json");
    let bytes = std::fs::read(&path).expect("read frozen identity corpus");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "55900fd6ffa69384e16367ed54f65bf3f6ca6fa80c64d7f799b12cee06be4627",
        "identity evaluation inputs changed after the implementation began"
    );
    let corpus: serde_json::Value = serde_json::from_slice(&bytes).expect("valid corpus JSON");
    let entries = corpus["entries"].as_array().expect("entries array");
    assert_eq!(entries.len(), 8);
    for entry in entries {
        for field in [
            "id",
            "kind",
            "pillar",
            "claim_key",
            "statement",
            "author",
            "source",
            "authority",
            "scope",
            "observed_at",
            "evidence_locator",
            "expected_judgement",
        ] {
            assert!(
                entry[field].as_str().is_some_and(|value| !value.is_empty()),
                "corpus entry is missing {field}: {entry}"
            );
        }
    }
}
