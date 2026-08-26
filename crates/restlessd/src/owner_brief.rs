/// One shared capability for every accountable actor. It is included in both
/// lead and Exec context; the owner BFF never invokes a model to rewrite it.
pub(crate) const PRESENT_TO_OWNER: &str = include_str!("../prompts/present-to-owner.md");

/// Ordinary conversation is a different surface from Attention: the owner
/// should receive the answer, not the actor's operating transcript.
pub(crate) const CONVERSE_WITH_OWNER: &str = include_str!("../prompts/converse-with-owner.md");

/// What the owner actually reads. Work titles, outcomes, resolutions, Attempt
/// summaries, artifact labels and gate names are rendered to the owner exactly
/// as an actor wrote them, and were being authored purely as instructions to a
/// model (S19-T4). This is deliberately a writing instruction rather than a
/// second owner-facing field: if actors comply, no schema, no second writer and
/// no renderer are needed at all.
pub(crate) const WRITING_WHAT_THE_OWNER_READS: &str =
    include_str!("../prompts/writing-what-the-owner-reads.md");
