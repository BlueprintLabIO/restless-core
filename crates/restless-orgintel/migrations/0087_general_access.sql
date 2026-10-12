-- General access: one choice per document and sheet for the company's people.
--
-- Private stays what participant visibility always meant: only the people added, and the
-- members of a linked room the document inherits. A company-visible document or sheet now
-- also says what every person may do without being added: view, comment (documents only) or
-- edit. Each one keeps edit, which is what people have had since they became teammates; each
-- participants-only one reads as private. The column is consulted only for company
-- visibility, and only for people: Exec and Staff never gain access this way, they are added.
-- Explicit participant grants still win.
ALTER TABLE native_documents
  ADD COLUMN company_access native_document_access NOT NULL DEFAULT 'edit';

ALTER TABLE native_sheets
  ADD COLUMN company_access text NOT NULL DEFAULT 'edit'
    CHECK (company_access IN ('read','edit'));
