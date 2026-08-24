-- S16-T1. A Work may explicitly require an owner review of its produced
-- outcome. The target remains an ordinary artifact reference and the review
-- remains an ordinary owner_handoff; this flag only prevents completion from
-- silently bypassing the owner when that boundary was declared up front.

ALTER TABLE work
  ADD COLUMN owner_review_required BOOLEAN NOT NULL DEFAULT FALSE;
