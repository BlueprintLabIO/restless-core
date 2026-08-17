//! The OrgIntel → owner-surface type seam.
//!
//! `web/` is TypeScript and OrgIntel is Rust, so the read model crosses a
//! language boundary. Hand-maintaining both sides is how the two silently
//! disagree — the field is renamed in Rust, the surface keeps rendering the
//! old name, and nothing fails until an owner reads a blank cell.
//!
//! So the Rust types are the single writer and the TypeScript is generated.
//! This test regenerates and compares:
//!
//! - `cargo test -p restless-orgintel` fails when the committed bindings no
//!   longer match the Rust types;
//! - `RESTLESS_WRITE_BINDINGS=1 cargo test -p restless-orgintel` rewrites them.
//!
//! It is a test rather than a build script on purpose: a build script would
//! silently rewrite a checked-in file during an ordinary `cargo build`, which
//! hides drift instead of reporting it.

use restless_orgintel::{
    ActorRow, ArtifactRefRow, ArtifactRefState, EventRow, GoalRow, MessageRow,
    OwnerHandoffCategory, OwnerHandoffRow, OwnerHandoffState, ScheduleRow, TeamRow,
    WorkAttemptFeedbackRow, WorkAttemptInputRow, WorkAttemptRow, WorkAttemptState, WorkEdgeKind,
    WorkEdgeRow, WorkGateRow, WorkGateRunRow, WorkGraphSnapshot, WorkRow, WorkStatus,
    WorkspaceSpec,
};
use ts_rs::TS;

const BINDINGS: &str = "../../web/src/lib/model/generated/orgintel.ts";

/// Declaration order is fixed here, not derived from a set, so regeneration is
/// byte-stable and a diff means a real type change.
fn render() -> String {
    // `i64` → `number`, not ts-rs's default `bigint`. These rows arrive through
    // `JSON.parse`, which cannot produce a bigint — serde writes a bare JSON
    // number and the browser reads one back. `bigint` would typecheck against a
    // value that never exists at runtime. Bigserial ids stay exact well past
    // any company's message count.
    let cfg = ts_rs::Config::new().with_large_int("number");
    let mut out = String::from(
        "// GENERATED — do not edit.\n\
         //\n\
         // Source: crates/restless-orgintel/src/lib.rs (the single writer).\n\
         // Regenerate: RESTLESS_WRITE_BINDINGS=1 cargo test -p restless-orgintel\n\
         //\n\
         // These are OrgIntel rows as they cross the wire, not the owner-surface\n\
         // view model. `$lib/model/view.ts` stays hand-written: it is a contract\n\
         // in its own right (what the surfaces need), and these are its inputs.\n\
         \n",
    );
    for decl in [
        // `EventRow.body` is an untyped JSON blob; emit the type it refers to
        // rather than leaving the file referencing an undeclared name.
        serde_json::Value::decl(&cfg),
        WorkStatus::decl(&cfg),
        WorkEdgeKind::decl(&cfg),
        WorkAttemptState::decl(&cfg),
        ArtifactRefState::decl(&cfg),
        OwnerHandoffCategory::decl(&cfg),
        OwnerHandoffState::decl(&cfg),
        WorkspaceSpec::decl(&cfg),
        TeamRow::decl(&cfg),
        ActorRow::decl(&cfg),
        GoalRow::decl(&cfg),
        WorkRow::decl(&cfg),
        WorkEdgeRow::decl(&cfg),
        WorkAttemptRow::decl(&cfg),
        WorkAttemptInputRow::decl(&cfg),
        WorkAttemptFeedbackRow::decl(&cfg),
        ArtifactRefRow::decl(&cfg),
        WorkGateRow::decl(&cfg),
        WorkGateRunRow::decl(&cfg),
        OwnerHandoffRow::decl(&cfg),
        WorkGraphSnapshot::decl(&cfg),
        ScheduleRow::decl(&cfg),
        MessageRow::decl(&cfg),
        EventRow::decl(&cfg),
    ] {
        out.push_str("export ");
        for (index, line) in decl.lines().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            out.push_str(line.trim_end());
        }
        out.push_str("\n\n");
    }
    out
}

#[test]
fn typescript_bindings_match_the_rust_types() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(BINDINGS);
    let rendered = render();

    if std::env::var_os("RESTLESS_WRITE_BINDINGS").is_some() {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).expect("create bindings directory");
        }
        std::fs::write(&path, &rendered).expect("write bindings");
        return;
    }

    let committed = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "{}: {error}\nRegenerate with: RESTLESS_WRITE_BINDINGS=1 cargo test -p restless-orgintel",
            path.display()
        )
    });

    if committed == rendered {
        return;
    }

    // Report the first differing line rather than dumping both files: the
    // whole point is that a founder can see *what* changed at a glance.
    let (line, was, now) = committed
        .lines()
        .zip(rendered.lines())
        .enumerate()
        .find(|(_, (a, b))| a != b)
        .map(|(index, (a, b))| (index + 1, a.to_string(), b.to_string()))
        .unwrap_or_else(|| {
            let at = committed.lines().count().min(rendered.lines().count()) + 1;
            (
                at,
                committed
                    .lines()
                    .nth(at - 1)
                    .unwrap_or("<end of file>")
                    .to_string(),
                rendered
                    .lines()
                    .nth(at - 1)
                    .unwrap_or("<end of file>")
                    .to_string(),
            )
        });

    panic!(
        "\n{} is stale — the Rust types changed and the TypeScript did not.\n\n\
         line {line}\n  committed: {was}\n  generated: {now}\n\n\
         Regenerate with: RESTLESS_WRITE_BINDINGS=1 cargo test -p restless-orgintel\n",
        path.display()
    );
}
