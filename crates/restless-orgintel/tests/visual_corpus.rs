use sha2::Digest as _;

#[test]
fn sprint_33_visual_corpus_is_frozen_product_grounded_and_channel_complete() {
    let bytes = include_bytes!("../../../docs/dogfood/company-identity/s33-visual-corpus.json");
    let digest = format!("{:x}", sha2::Sha256::digest(bytes));
    assert_eq!(
        digest,
        "09a5d8cf2f1251e145f54e7a8a8875cd21430adbe0fc73fe11ca7c6f3f8af6a6"
    );
    let corpus: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    assert_eq!(corpus["targets"].as_array().unwrap().len(), 4);
    assert_eq!(corpus["anti_examples"].as_array().unwrap().len(), 4);
    assert!(!corpus["product_truth"]["version"]
        .as_str()
        .unwrap()
        .is_empty());
    for target in corpus["targets"].as_array().unwrap() {
        assert!(!target["native_constraints"].as_array().unwrap().is_empty());
        assert!(!target["control"].as_str().unwrap().is_empty());
        assert!(!target["effect_candidate"].as_str().unwrap().is_empty());
    }
}
