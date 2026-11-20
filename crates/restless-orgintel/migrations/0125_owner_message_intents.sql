-- What kind of message the owner sent (20 November 2026, docs/specs/owner-interaction.md D3):
-- instruction, outcome-only request, question, redirect, status check, approval, stop, or other.
-- Fixed rules in restless-orgintel put each human message in one class the first time the
-- company's numbers are read; agent messages stay NULL. The company counts its owner's friction
-- from these classes (D7).
ALTER TABLE messages
  ADD COLUMN owner_intent text
    CHECK (owner_intent IN ('instruction','outcome_only','question','redirect',
                            'status_check','approval','stop','other'));

-- Where classifying left off, and the classified messages the friction numbers read.
CREATE INDEX messages_owner_intent ON messages (id) WHERE owner_intent IS NOT NULL;
