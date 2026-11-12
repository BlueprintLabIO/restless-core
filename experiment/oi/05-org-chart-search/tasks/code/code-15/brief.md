# CLI decisions report unavailable data instead of claiming nothing needs you

A reproduced 503 containing the text 404 previously succeeded with 'Nothing needs you.' Preserve the HTTP status, reject malformed company/attention lists, and fail incomplete reads. Read the catalog once. Verified 31 CLI unit tests and two executable CLI scenarios covering unavailable, malformed, empty and removed companies.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`crates/restless/tests/remote_decisions.rs`

```
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output};
use std::sync::{ atomic::{AtomicBool, Ordering}, Arc, };
use std::time::Duration;
use serde_json::{json, Value};
```
