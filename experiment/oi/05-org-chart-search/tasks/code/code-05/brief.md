# Documents: the company owner can retitle a document Exec or Staff wrote

update_document_metadata lets an active company owner (or the local
"owner" actor) change only the title of any document they can see, as
set_document_archived already lets them archive it. Kind, visibility and
room links still need the document's own owner with edit access, and the
body stays read-only to them.

Checks: cargo check -p restless-orgintel --tests (clean);
cargo test -p restless-orgintel --test documents the_company_owner_may_retitle
compiled and ran, but SKIPPED its body: RESTLESS_TEST_DATABASE_URL is unset
on this host, so the scenario still needs a Postgres-backed run.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`crates/restless-orgintel/tests/documents.rs`

```
use restless_orgintel::{ DocumentAccess, DocumentError, DocumentKind, DocumentStatus, DocumentVisibility, ImportDocumentMarkdown, ImportRuntimeDocument, NativeDocumentPortableEnvelope, NewDocument, NewNamedDocumentVersion, OrgIntel, RemoveDocumentParticipant, RequestDocumentReview, RestoreDocumentVersion, RoomKind, SetDocumentParticipant, UpdateDocumentMetadata, NATIVE_DOCUMENT_PORTABLE_ENVELOPE_VERSION, };
use serde_json::{json, Value};
use sqlx::{Connection as _, PgConnection};
use std::time::Duration;
use tokio::time::{sleep, timeout};
use uuid::Uuid;
```
