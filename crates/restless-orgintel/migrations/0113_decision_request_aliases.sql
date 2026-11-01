-- A second client's equivalent request retains the first receipt after Undo or apply.
CREATE TABLE owner_handoff_decision_requests (
    id UUID PRIMARY KEY,
    decision_id UUID NOT NULL REFERENCES owner_handoff_decisions(id) ON DELETE CASCADE
);
INSERT INTO owner_handoff_decision_requests SELECT id,id FROM owner_handoff_decisions;
CREATE TABLE owner_document_decision_requests (
    id UUID PRIMARY KEY,
    decision_id UUID NOT NULL REFERENCES owner_document_decisions(id) ON DELETE CASCADE
);
INSERT INTO owner_document_decision_requests SELECT id,id FROM owner_document_decisions;
