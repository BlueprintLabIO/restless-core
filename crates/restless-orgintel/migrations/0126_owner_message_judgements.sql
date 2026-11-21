-- Exec's judgement of each owner message (21 November 2026, docs/specs/owner-interaction.md):
-- answering the owner, Exec puts every class that applies, whether the company could have spared
-- them the message, the main gap and the repair on its reply, and the runtime stores them here.
-- Fixed rules remain the fallback for a message Exec did not judge; `owner_intent_source` says
-- which one labelled the row, and `owner_intent` stays the main class.
ALTER TABLE messages
  ADD COLUMN owner_intent_source text CHECK (owner_intent_source IN ('exec','rules')),
  ADD COLUMN owner_intents text[]
    CHECK (owner_intents <@ ARRAY['instruction','outcome_only','question','redirect',
                                  'status_check','approval','stop','other']::text[]),
  ADD COLUMN owner_avoidable text CHECK (owner_avoidable IN ('yes','partly','no')),
  ADD COLUMN owner_gap text
    CHECK (owner_gap IN ('check','told','handoff','look','memory','dropped','misread','idle')),
  ADD COLUMN owner_could_have_known text,
  ADD COLUMN owner_repair text,
  ADD COLUMN owner_repair_lead text;

-- Everything classified before this was classified by the rules.
UPDATE messages SET owner_intent_source='rules', owner_intents=ARRAY[owner_intent]
 WHERE owner_intent IS NOT NULL;
