# A person keeps their Actor when their issuer moves address

Cloud's issuer moved from app.restless.run to restless.run, and each plane
bound everyone who entered afterwards to a fresh Actor: the founding owner
became a "Company member" and Reno refused their first message to Exec.
The same subject arriving with the same membership under the issuer a plane
now trusts takes its binding along; a fresh Actor already bound for them
under the new issuer gives way and is retired.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`crates/restless-orgintel/tests/access.rs`

```
use chrono::{DateTime, Duration, Utc};
use restless_orgintel::{ CompanyAccessIdentity, ExternalMembershipControlContext, ExternalMembershipStatus, HumanAccessContext, MembershipControlOutcome, OrgIntel, OrgIntelError, MEMBERSHIP_CONTROL_CONTRACT_VERSION, };
use sqlx::{Connection as _, PgConnection};
use uuid::Uuid;
```
