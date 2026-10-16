# One Inbox action per prepared decision

Observed: the owner saw a stale conversational question after answering its prepared decision. Inspecting the queue confirmed two independent lifecycles for the same input. A multi-agent regression also exposed the old lead-only conversation filter.

Layer: OrgIntel read projection and transcript metadata. Existing handoffs remain the decision owner; this introduces no second workflow or permission writer.

Acceptance: an explicit existing handoff identity supplies up to two newest, stably ordered context details inside one item. A resolved source leaves no conversational duplicate. Missing/unknown provenance never hides an independent question. Every active agent's direct question can reach the Inbox, with the same canonical Room/participation/revision/deletion checks as conversation reads. Other humans' Rooms remain private.

Makes deletable: the lead-only filter and one-query-per-actor fan-out; separate actionable reminders for the same prepared decision.
