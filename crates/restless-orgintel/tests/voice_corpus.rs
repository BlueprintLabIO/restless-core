#[test]
fn sprint_32_voice_corpus_is_frozen_and_channel_complete() {
    let bytes = include_bytes!("../../../docs/dogfood/company-identity/s32-channel-corpus.json");
    let digest = format!("{:x}", sha2::Sha256::digest(bytes));
    assert_eq!(
        digest,
        "167b1768853e0b7cbfae466a838303cc3ab3bd7de59480ad4e58b335cf470c39"
    );
    let corpus: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let channels = corpus["channels"].as_array().unwrap();
    assert_eq!(channels.len(), 6);
    for channel in channels {
        for field in [
            "channel",
            "author",
            "reader",
            "plain_control",
            "ai_heavy_negative",
            "failure",
        ] {
            assert!(!channel[field]
                .as_str()
                .unwrap_or_default()
                .trim()
                .is_empty());
        }
    }
}

use sha2::Digest as _;
