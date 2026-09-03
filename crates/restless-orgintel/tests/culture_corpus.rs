use sha2::Digest as _;
#[test]
fn sprint_34_corpus_is_frozen_consequential_and_surveillance_free() {
    let bytes = include_bytes!("../../../docs/dogfood/company-identity/s34-behaviour-corpus.json");
    assert_eq!(
        format!("{:x}", sha2::Sha256::digest(bytes)),
        "391aff6707a5fc18e3cff62230103e4d418d415778841060afbaa7653177fe6f"
    );
    let corpus: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    assert_eq!(corpus["cases"].as_array().unwrap().len(), 5);
    assert_eq!(corpus["prohibited_inference"].as_array().unwrap().len(), 5);
    for case in corpus["cases"].as_array().unwrap() {
        for key in [
            "case",
            "consequence",
            "authority",
            "evidence",
            "decision",
            "outcome",
        ] {
            assert!(!case[key].as_str().unwrap_or_default().trim().is_empty());
        }
    }
}
