//! Whether a document an agent hands to a person reads as finished work.
//!
//! Owners were sent documents that quoted internal ids ("Work ebceef7f", "message 963"), Restless
//! commands (`restless credential check`), unfilled placeholders (`[LINK TO … CONCEPT PAGE]`) and
//! opened with notes about their own revisions before the content. This is the pure check run
//! when an agent shares a document with a person or asks one to review it; owner-authored
//! documents are never checked. It looks only at the structured body, so it is deterministic and
//! has no I/O.
//!
//! Ordinary code stays legitimate: a technical document may show `npm install` or `git log` in a
//! code block. Only things that are internal to Restless (its own CLI, its runtime paths, its ids)
//! and drafting leftovers are rejected.
use serde_json::Value;

/// What kind of leftover makes a document not ready for a person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OwnerReadinessIssue {
    /// A Work, Attempt or other record id, or a raw UUID.
    InternalId,
    /// An internal message number such as "message 963".
    MessageNumber,
    /// A `restless …` command or a path inside the company computer.
    InternalCommand,
    /// `[LINK TO …]`, `[TODO]`, `TODO:` and similar drafting placeholders.
    Placeholder,
    /// Notes about the document's own revisions, before the content or as a section.
    RevisionNotes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerReadinessProblem {
    pub issue: OwnerReadinessIssue,
    /// The offending words exactly as the document has them (shortened when long).
    pub excerpt: String,
}

/// Every problem in a structured document body (`{"type":"doc",…}`), in document order, without
/// duplicates.
pub fn owner_readiness_problems(content: &Value) -> Vec<OwnerReadinessProblem> {
    let mut blocks = Vec::new();
    collect_blocks(content, &mut blocks);
    let mut problems: Vec<OwnerReadinessProblem> = Vec::new();
    let mut push = |issue, excerpt: String| {
        let problem = OwnerReadinessProblem {
            issue,
            excerpt: shorten(excerpt.trim()),
        };
        if !problems.contains(&problem) {
            problems.push(problem);
        }
    };
    let mut leading = 0usize;
    for block in &blocks {
        if block.text.trim().is_empty() {
            continue;
        }
        if block.kind == BlockKind::Code {
            for line in block.text.lines() {
                for found in restless_commands(line, true) {
                    push(OwnerReadinessIssue::InternalCommand, found);
                }
                for found in internal_paths(line) {
                    push(OwnerReadinessIssue::InternalCommand, found);
                }
            }
            leading += 1;
            continue;
        }
        if is_revision_note(block, leading < 3) {
            push(OwnerReadinessIssue::RevisionNotes, block.text.clone());
        }
        leading += 1;
        for found in internal_ids(&block.text) {
            push(OwnerReadinessIssue::InternalId, found);
        }
        for found in message_numbers(&block.text) {
            push(OwnerReadinessIssue::MessageNumber, found);
        }
        for span in &block.code_spans {
            for found in restless_commands(span, true) {
                push(OwnerReadinessIssue::InternalCommand, found);
            }
        }
        for found in restless_commands(&block.prose, false) {
            push(OwnerReadinessIssue::InternalCommand, found);
        }
        for found in internal_paths(&block.text) {
            push(OwnerReadinessIssue::InternalCommand, found);
        }
        for found in placeholders(&block.text) {
            push(OwnerReadinessIssue::Placeholder, found);
        }
    }
    problems
}

/// The actionable refusal an agent receives, or `None` when the document is ready.
pub fn owner_readiness_error(content: &Value) -> Option<String> {
    const SHOWN: usize = 8;
    let problems = owner_readiness_problems(content);
    if problems.is_empty() {
        return None;
    }
    let mut message = String::from(
        "This document is not ready for a person to read yet, so it was not shared or sent for \
         review. Edit the document to fix each of these, then try again:",
    );
    for problem in problems.iter().take(SHOWN) {
        let advice = match problem.issue {
            OwnerReadinessIssue::InternalId => {
                "an internal id. Say what the work or record is in words, or link the document \
                 or file it produced"
            }
            OwnerReadinessIssue::MessageNumber => {
                "an internal message number. Say who said what, and when, instead"
            }
            OwnerReadinessIssue::InternalCommand => {
                "a Restless command or company-computer path. Say what was checked and what it \
                 showed, in plain words"
            }
            OwnerReadinessIssue::Placeholder => {
                "an unfilled placeholder. Put in the real link or text, or remove it"
            }
            OwnerReadinessIssue::RevisionNotes => {
                "notes about this document's own revisions. Start with the content itself; \
                 record what changed and who changed it as the version note (`restless document \
                 checkpoint DOCUMENT --snapshot-file PATH --reason \"what changed\"`), which \
                 people see in the document's history"
            }
        };
        message.push_str(&format!("\n- \"{}\": {advice}.", problem.excerpt));
    }
    if problems.len() > SHOWN {
        message.push_str(&format!(
            "\n- and {} more like these.",
            problems.len() - SHOWN
        ));
    }
    Some(message)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Heading,
    Paragraph,
    Code,
}

#[derive(Debug)]
struct TextBlock {
    kind: BlockKind,
    /// Every word as the reader sees it, inline code included.
    text: String,
    /// The same words with inline code blanked out.
    prose: String,
    code_spans: Vec<String>,
}

fn node_type(node: &Value) -> &str {
    node.get("type").and_then(Value::as_str).unwrap_or_default()
}

fn is_inline(node: &Value) -> bool {
    matches!(
        node_type(node),
        "text" | "hardBreak" | "mention" | "reference"
    )
}

fn collect_blocks(node: &Value, out: &mut Vec<TextBlock>) {
    let children = node
        .get("content")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    if node_type(node) == "codeBlock" {
        out.push(TextBlock {
            kind: BlockKind::Code,
            text: children
                .iter()
                .filter_map(|child| child.get("text").and_then(Value::as_str))
                .collect(),
            prose: String::new(),
            code_spans: Vec::new(),
        });
        return;
    }
    if children.iter().any(is_inline) {
        let mut text = String::new();
        let mut prose = String::new();
        let mut code_spans = Vec::new();
        for child in children {
            let attr = |name: &str| {
                child
                    .get("attrs")
                    .and_then(|attrs| attrs.get(name))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string()
            };
            match node_type(child) {
                "text" => {
                    let raw = child
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let code = child
                        .get("marks")
                        .and_then(Value::as_array)
                        .is_some_and(|marks| marks.iter().any(|mark| node_type(mark) == "code"));
                    if code {
                        code_spans.push(raw.to_string());
                        prose.push(' ');
                    } else {
                        prose.push_str(raw);
                    }
                    text.push_str(raw);
                }
                "hardBreak" => {
                    text.push('\n');
                    prose.push('\n');
                }
                "mention" => {
                    let label = format!("@{}", attr("label"));
                    text.push_str(&label);
                    prose.push_str(&label);
                }
                "reference" => {
                    text.push_str(&attr("label"));
                    prose.push_str(&attr("label"));
                }
                _ => {}
            }
        }
        out.push(TextBlock {
            kind: if node_type(node) == "heading" {
                BlockKind::Heading
            } else {
                BlockKind::Paragraph
            },
            text,
            prose,
            code_spans,
        });
        return;
    }
    for child in children {
        collect_blocks(child, out);
    }
}

fn shorten(text: &str) -> String {
    const LIMIT: usize = 70;
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= LIMIT {
        return flat;
    }
    format!("{}…", flat.chars().take(LIMIT).collect::<String>())
}

/// Punctuation that may wrap a token in prose: quotes, brackets, backticks, sentence marks.
fn trim_token(token: &str) -> &str {
    token.trim_matches(|c: char| {
        matches!(
            c,
            '`' | '"'
                | '\''
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | ','
                | '.'
                | ';'
                | ':'
                | '!'
                | '?'
                | '“'
                | '”'
                | '‘'
                | '’'
                | '*'
                | '_'
        )
    })
}

fn is_uuid(token: &str) -> bool {
    let bytes = token.as_bytes();
    token.len() == 36
        && bytes.iter().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                *byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

/// A short or full lower-case hex id with at least one digit and one letter, like `ebceef7f`.
fn is_short_hex_id(token: &str) -> bool {
    let token = token.trim_start_matches('#');
    (6..=36).contains(&token.len())
        && token
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c) || c == '-')
        && token.chars().any(|c| c.is_ascii_digit())
        && token.chars().any(|c| ('a'..='f').contains(&c))
}

/// Record names agents use when quoting an id ("Work ebceef7f", "Attempt 3f2a91c0").
const ID_NOUNS: &[&str] = &[
    "work", "attempt", "task", "room", "thread", "review", "goal", "job", "document", "doc",
    "actor", "handoff", "decision", "effect", "receipt", "permit", "id",
];

fn internal_ids(text: &str) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut found = Vec::new();
    for (index, word) in words.iter().enumerate() {
        let token = trim_token(word);
        if is_uuid(token) {
            found.push(token.to_string());
            continue;
        }
        if index == 0 {
            continue;
        }
        let noun = trim_token(words[index - 1]).to_lowercase();
        if ID_NOUNS.contains(&noun.as_str()) && is_short_hex_id(token) {
            found.push(format!("{} {token}", trim_token(words[index - 1])));
        }
    }
    found
}

/// "message 963", "message #12", "msg 4521". Small numbers ("Message 2" of an email sequence)
/// are content, so a bare number needs three or more digits.
fn message_numbers(text: &str) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut found = Vec::new();
    for pair in words.windows(2) {
        let noun = trim_token(pair[0]).to_lowercase();
        if !matches!(noun.as_str(), "message" | "messages" | "msg" | "mail") {
            continue;
        }
        let raw = pair[1].trim_end_matches([',', '.', ';', ':', ')', '!', '?']);
        let (hashed, digits) = match raw.strip_prefix('#') {
            Some(rest) => (true, rest),
            None => (false, raw),
        };
        if !digits.is_empty()
            && digits.chars().all(|c| c.is_ascii_digit())
            && (hashed || digits.len() >= 3)
        {
            found.push(format!("{} {raw}", trim_token(pair[0])));
        }
    }
    found
}

/// Subcommands of the `restless` CLI. In prose only those that are not also ordinary English
/// after the word "restless" count; inside code every one does.
const RESTLESS_SUBCOMMANDS: &[&str] = &[
    "login",
    "logout",
    "use",
    "contexts",
    "whoami",
    "companies",
    "decisions",
    "ask",
    "ssh",
    "connect",
    "appliance",
    "open",
    "company",
    "credential",
    "legal",
    "setting",
    "finance",
    "publish",
    "room",
    "document",
    "apps",
    "direction",
    "charter",
    "request",
    "sheet",
    "decide",
    "job",
    "tools",
    "up",
    "down",
    "sleep",
    "status",
    "doctor",
    "orgintel-init",
    "identity",
    "attention",
    "browser",
    "wake",
    "tell",
    "chat",
    "watch",
    "attach",
    "people",
    "teams",
    "schedule",
    "judgement",
    "receipts",
    "spend",
    "telemetry",
    "spend-correct",
    "goal",
    "mandate",
    "email",
    "skill",
    "work",
    "commission",
    "events",
    "inbox",
    "message",
    "react",
    "clear-poison",
    "approve",
    "effect",
    "effect-reconcile",
];

const RESTLESS_PROSE_SUBCOMMANDS: &[&str] = &[
    "whoami",
    "contexts",
    "appliance",
    "credential",
    "orgintel-init",
    "judgement",
    "receipts",
    "telemetry",
    "spend-correct",
    "mandate",
    "commission",
    "clear-poison",
    "effect-reconcile",
    "inbox",
    "charter",
];

fn restless_commands(text: &str, in_code: bool) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut found = Vec::new();
    for (index, word) in words.iter().enumerate() {
        if trim_token(word) != "restless" {
            continue;
        }
        let Some(next) = words.get(index + 1) else {
            continue;
        };
        let sub = next.trim_end_matches(['`', '"', '\'', ',', '.', ';', ':', ')']);
        let flagged = words
            .iter()
            .skip(index + 2)
            .take(3)
            .any(|after| after.trim_start_matches('`').starts_with("--"));
        let command = if in_code {
            RESTLESS_SUBCOMMANDS.contains(&sub)
        } else {
            RESTLESS_PROSE_SUBCOMMANDS.contains(&sub)
                || (RESTLESS_SUBCOMMANDS.contains(&sub) && flagged)
        };
        if command {
            let end = (index + 4).min(words.len());
            found.push(
                words[index..end]
                    .iter()
                    .map(|word| trim_token(word))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
    }
    found
}

/// Paths inside the company computer or a Restless home: meaningless to a reader.
fn internal_paths(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(trim_token)
        .filter(|token| {
            token.starts_with("/company/")
                || token.starts_with("~/.restless")
                || token.starts_with(".restless/")
                || token.starts_with("/home/agent/")
        })
        .map(str::to_string)
        .collect()
}

const PLACEHOLDER_OPENINGS: &[&str] = &[
    "link to",
    "link",
    "todo",
    "tbd",
    "tk",
    "insert",
    "placeholder",
    "fixme",
    "xxx",
    "add link",
    "add url",
    "url",
    "citation needed",
];

fn placeholders(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else {
            break;
        };
        let inner = after[..close].trim();
        let lower = inner.to_lowercase();
        let opens = PLACEHOLDER_OPENINGS.iter().any(|opening| {
            lower == *opening
                || lower
                    .strip_prefix(opening)
                    .is_some_and(|tail| tail.starts_with([' ', ':', '-', '…', '.', '/']))
        });
        if opens {
            found.push(format!("[{inner}]"));
        }
        rest = &after[close + 1..];
    }
    for word in text.split_whitespace() {
        if word.starts_with('[') {
            continue;
        }
        let token = trim_token(word);
        if matches!(token, "TODO" | "FIXME" | "XXX" | "TKTK") {
            found.push(token.to_string());
        }
    }
    found
}

/// Section titles that are only ever about the document itself, wherever they appear.
const REVISION_TITLES: &[&str] = &[
    "revision notes",
    "revision note",
    "revision history",
    "revision log",
    "changes in this revision",
    "changes in this version",
    "changes in this draft",
    "notes on this revision",
    "about this revision",
    "version history",
    "edit history",
];

/// Titles that are about the document when it opens with them, but may be content further down
/// (a report can have a "What changed" section about the market).
const LEADING_REVISION_TITLES: &[&str] = &[
    "changes since",
    "changes from",
    "what changed",
    "what's changed",
    "what's new in this version",
    "what's new in this draft",
    "change log",
    "changelog",
    "editor's note",
    "update notes",
];

/// How a leading paragraph about the document's own revisions begins.
const REVISION_OPENINGS: &[&str] = &[
    "this revision",
    "in this revision",
    "this version",
    "in this version",
    "revised by",
    "rewritten by",
    "updated by",
    "edited by",
    "revision:",
    "changes:",
    "changes since",
    "changes from",
    "since the last version",
    "since the previous version",
    "since the last draft",
    "since the previous draft",
    "since last time",
    "what changed",
    "this is a revised",
    "this is revision",
    "this is version",
    "this is the revised",
    "this is the updated",
];

fn titled(text: &str, titles: &[&str]) -> bool {
    titles.iter().any(|candidate| {
        text == *candidate
            || text
                .strip_prefix(candidate)
                .is_some_and(|tail| tail.starts_with([' ', '(', ':', '-', '—', '–']))
    })
}

fn is_revision_note(block: &TextBlock, leading: bool) -> bool {
    let normal = block
        .text
        .trim()
        .to_lowercase()
        .replace(['’', '‘'], "'")
        .trim_start_matches(['#', '*', '_', ' '])
        .to_string();
    let title = normal.trim_end_matches([':', '.', ' ', '*', '_']);
    match block.kind {
        BlockKind::Heading => {
            titled(title, REVISION_TITLES) || (leading && titled(title, LEADING_REVISION_TITLES))
        }
        BlockKind::Paragraph => {
            // A bold label "Revision notes:" opening a paragraph is a section in disguise.
            let label = normal
                .split_once(':')
                .map(|(head, _)| head.trim_end_matches(['*', '_', ' ']))
                .unwrap_or_default();
            REVISION_TITLES.contains(&label)
                || (leading
                    && (titled(title, REVISION_TITLES)
                        || titled(title, LEADING_REVISION_TITLES)
                        || REVISION_OPENINGS
                            .iter()
                            .any(|opening| normal.starts_with(opening))))
        }
        BlockKind::Code => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn text(value: &str) -> Value {
        json!({"type":"text","text":value})
    }

    fn code(value: &str) -> Value {
        json!({"type":"text","text":value,"marks":[{"type":"code"}]})
    }

    fn paragraph(children: Vec<Value>) -> Value {
        json!({"type":"paragraph","attrs":{"block_id":"p"},"content":children})
    }

    fn heading(value: &str) -> Value {
        json!({"type":"heading","attrs":{"block_id":"h","level":2},"content":[text(value)]})
    }

    fn code_block(value: &str) -> Value {
        json!({"type":"codeBlock","attrs":{"block_id":"c","language":"sh"},"content":[text(value)]})
    }

    fn doc(blocks: Vec<Value>) -> Value {
        json!({"type":"doc","content":blocks})
    }

    fn issues(content: &Value) -> Vec<(OwnerReadinessIssue, String)> {
        owner_readiness_problems(content)
            .into_iter()
            .map(|problem| (problem.issue, problem.excerpt))
            .collect()
    }

    #[test]
    fn finished_documents_pass_including_ordinary_code() {
        let content = doc(vec![
            heading("Outreach emails"),
            paragraph(vec![text(
                "Message 1 goes to the café owners on Monday. Message 2 follows a week later.",
            )]),
            paragraph(vec![
                text("Install it with "),
                code("npm install"),
                text(" and read the restless work of the night shift."),
            ]),
            code_block("git log --oneline\ncargo test -p shop"),
            paragraph(vec![text(
                "Hi [First name], we opened in 2026 and work 7 days a week. See [1].",
            )]),
            json!({"type":"bulletList","content":[{"type":"listItem","content":[
                paragraph(vec![text("Revised prices start in March.")])
            ]}]}),
        ]);
        assert_eq!(issues(&content), vec![]);
        assert_eq!(owner_readiness_error(&content), None);
    }

    #[test]
    fn the_owners_reported_leftovers_are_each_caught() {
        let content = doc(vec![
            paragraph(vec![text(
                "Following Work ebceef7f and message 963, the copy is final.",
            )]),
            paragraph(vec![
                text("I ran "),
                code("restless credential check"),
                text(" and "),
                code("restless mandate list"),
                text(" returns []."),
            ]),
            paragraph(vec![text("Read more on [LINK TO … CONCEPT PAGE].")]),
        ]);
        let found = issues(&content);
        assert!(found.contains(&(OwnerReadinessIssue::InternalId, "Work ebceef7f".into())));
        assert!(found.contains(&(OwnerReadinessIssue::MessageNumber, "message 963".into())));
        assert!(found.contains(&(
            OwnerReadinessIssue::InternalCommand,
            "restless credential check".into()
        )));
        assert!(found.contains(&(
            OwnerReadinessIssue::InternalCommand,
            "restless mandate list".into()
        )));
        assert!(found.contains(&(
            OwnerReadinessIssue::Placeholder,
            "[LINK TO … CONCEPT PAGE]".into()
        )));
        let error = owner_readiness_error(&content).unwrap();
        assert!(error.contains("\"Work ebceef7f\": an internal id"));
        assert!(error.contains("not ready for a person"));
    }

    #[test]
    fn ids_split_across_marks_and_raw_uuids_are_caught() {
        let content = doc(vec![paragraph(vec![
            text("See Attempt "),
            json!({"type":"text","text":"3f2a91c0","marks":[{"type":"bold"}]}),
            text(" (record 0b5c9a39-4c0e-4d7b-9a63-2b6f0a6a1f11)."),
        ])]);
        let found = issues(&content);
        assert_eq!(
            found,
            vec![
                (OwnerReadinessIssue::InternalId, "Attempt 3f2a91c0".into()),
                (
                    OwnerReadinessIssue::InternalId,
                    "0b5c9a39-4c0e-4d7b-9a63-2b6f0a6a1f11".into()
                ),
            ]
        );
    }

    #[test]
    fn message_numbers_need_a_hash_or_three_digits() {
        let content = doc(vec![paragraph(vec![text(
            "Message 3 is short. As agreed in message #12 and msg 4521.",
        )])]);
        assert_eq!(
            issues(&content),
            vec![
                (OwnerReadinessIssue::MessageNumber, "message #12".into()),
                (OwnerReadinessIssue::MessageNumber, "msg 4521".into()),
            ]
        );
    }

    #[test]
    fn restless_commands_and_paths_are_caught_in_code_blocks_but_other_code_is_not() {
        let content = doc(vec![code_block(
            "npm run build\nrestless work show abc\ncat /company/run/jobs/1.log",
        )]);
        let found = issues(&content);
        assert_eq!(
            found,
            vec![
                (
                    OwnerReadinessIssue::InternalCommand,
                    "restless work show abc".into()
                ),
                (
                    OwnerReadinessIssue::InternalCommand,
                    "/company/run/jobs/1.log".into()
                ),
            ]
        );
    }

    #[test]
    fn prose_commands_need_an_unambiguous_subcommand_or_a_flag() {
        let ordinary = doc(vec![paragraph(vec![text(
            "Customers grow restless people say, and restless work wears staff down.",
        )])]);
        assert_eq!(issues(&ordinary), vec![]);
        let command = doc(vec![paragraph(vec![text(
            "Then run restless work list --status open to see it.",
        )])]);
        assert_eq!(
            issues(&command),
            vec![(
                OwnerReadinessIssue::InternalCommand,
                "restless work list --status".into()
            )]
        );
    }

    #[test]
    fn drafting_placeholders_are_caught() {
        let content = doc(vec![paragraph(vec![text(
            "Pricing [TODO]. Case study [link]. [Insert testimonial here] TODO: confirm. [TBD: owner]",
        )])]);
        let found: Vec<_> = issues(&content).into_iter().map(|(_, text)| text).collect();
        assert_eq!(
            found,
            vec![
                "[TODO]",
                "[link]",
                "[Insert testimonial here]",
                "[TBD: owner]",
                "TODO"
            ]
        );
    }

    #[test]
    fn revision_notes_before_the_content_are_caught() {
        let leading_heading = doc(vec![
            heading("Changes in this revision"),
            paragraph(vec![text("Tightened the subject lines.")]),
            heading("Email 1"),
        ]);
        assert_eq!(
            issues(&leading_heading),
            vec![(
                OwnerReadinessIssue::RevisionNotes,
                "Changes in this revision".into()
            )]
        );
        let leading_paragraph = doc(vec![
            paragraph(vec![text(
                "Rewritten by Mia after your comments; since the last version the tone is warmer.",
            )]),
            heading("Email 1"),
        ]);
        assert_eq!(
            issues(&leading_paragraph)[0].0,
            OwnerReadinessIssue::RevisionNotes
        );
        let labelled = doc(vec![
            heading("Email 1"),
            paragraph(vec![text("Hello there.")]),
            paragraph(vec![
                json!({"type":"text","text":"Revision notes:","marks":[{"type":"bold"}]}),
                text(" moved the offer up."),
            ]),
        ]);
        assert_eq!(issues(&labelled)[0].0, OwnerReadinessIssue::RevisionNotes);
        let trailing_section = doc(vec![
            heading("Email 1"),
            paragraph(vec![text("Hello there.")]),
            paragraph(vec![text("Thanks.")]),
            paragraph(vec![text("Mia")]),
            heading("Revision history"),
        ]);
        assert_eq!(
            issues(&trailing_section)[0].0,
            OwnerReadinessIssue::RevisionNotes
        );
        let error = owner_readiness_error(&leading_heading).unwrap();
        assert!(error.contains("--reason"));
        assert!(error.contains("history"));
    }

    #[test]
    fn a_late_paragraph_mentioning_versions_is_content() {
        let content = doc(vec![
            heading("Launch plan"),
            paragraph(vec![text("We open in March.")]),
            paragraph(vec![text("The menu is set.")]),
            paragraph(vec![text(
                "This version of the menu drops the lunch special.",
            )]),
        ]);
        assert_eq!(issues(&content), vec![]);
    }

    #[test]
    fn the_refusal_is_bounded() {
        let many = (0..20)
            .map(|index| paragraph(vec![text(&format!("Pending [TODO {index}]"))]))
            .collect();
        let error = owner_readiness_error(&doc(many)).unwrap();
        assert_eq!(error.matches("\n- \"").count(), 8);
        assert!(error.ends_with("and 12 more like these."));
    }
}
