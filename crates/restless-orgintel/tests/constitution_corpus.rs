use sha2::{Digest as _, Sha256};
use std::collections::HashSet;

#[test]
fn sprint_35_constitution_corpus_is_frozen_complete_and_transferable() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/dogfood/company-identity/s35-integrated-corpus.json");
    let bytes = std::fs::read(&path).expect("read frozen constitution corpus");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "2279516e1403349b8618d33520ef58ccfb28ad53b3dff4518b54a6708c33548f",
        "constitution evaluation inputs changed after implementation began"
    );

    let corpus: serde_json::Value = serde_json::from_slice(&bytes).expect("valid corpus JSON");
    assert_eq!(corpus["publication_authority"], false);

    let company_a = &corpus["company_a"];
    let assets = company_a["assets"].as_array().expect("assets array");
    assert_eq!(assets.len(), 9);
    let mut channels = HashSet::new();
    for asset in assets {
        for field in [
            "id",
            "channel",
            "audience",
            "author",
            "native_environment",
            "truth_dependency",
            "effect_boundary",
            "headline",
            "body",
        ] {
            assert!(
                asset[field].as_str().is_some_and(|value| !value.is_empty()),
                "asset is missing {field}: {asset}"
            );
        }
        assert!(channels.insert(asset["channel"].as_str().unwrap()));
    }
    assert!(channels.contains("blog"));
    assert!(channels.contains("product_ui"));
    assert!(channels.contains("founder_email"));

    for pillar in ["truth", "voice", "visual", "culture"] {
        assert!(
            company_a["identity"][pillar]
                .as_str()
                .is_some_and(|value| !value.is_empty()),
            "Restless is missing the {pillar} account"
        );
    }

    let correction = &corpus["product_change"];
    for field in ["fact", "correction", "expected_stale_dependency"] {
        assert!(correction[field]
            .as_str()
            .is_some_and(|value| !value.is_empty()));
    }

    let company_b = &corpus["company_b"];
    assert_ne!(company_a["name"], company_b["name"]);
    for pillar in ["truth", "voice", "visual", "culture"] {
        assert_ne!(
            company_a["identity"][pillar], company_b["identity"][pillar],
            "held-back company must remain distinct for {pillar}"
        );
    }
}
