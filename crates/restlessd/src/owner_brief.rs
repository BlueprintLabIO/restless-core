/// One shared capability for every accountable actor. It is included in both
/// lead and Exec context; the owner BFF never invokes a model to rewrite it.
pub(crate) const PRESENT_TO_OWNER: &str = include_str!("../prompts/present-to-owner.md");

/// Ordinary conversation is a different surface from Attention: the owner
/// should receive the answer, not the actor's operating transcript.
pub(crate) const CONVERSE_WITH_OWNER: &str = include_str!("../prompts/converse-with-owner.md");
