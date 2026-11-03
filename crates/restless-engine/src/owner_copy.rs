//! Owner-facing copy guard, the engine's half of `web/src/lib/model/owner-copy.ts`.
//!
//! The owner reads plain words. A record an agent or the system wrote (a reason, a resume
//! condition, a provider's error) can carry ids, hashes, message numbers, agent handles,
//! snake_case names and raw provider errors. The platform's own sentences never do, and an
//! agent's own words are kept but scrubbed of those before they reach a card. When what is left
//! still reads as internals, the platform's own line stands in.

/// Engine nouns an owner never needs to read (the same list as the web guard).
const INTERNAL_NOUNS: &[&str] = &[
    "delivery node",
    "work node",
    "digest",
    "digests",
    "bound to",
    "handoff",
    "attempt_id",
    "attempt id",
    "work_id",
    "work id",
    "orgintel",
    "authority record",
];

/// The role an agent handle ends in: `reno-supervisor`, `ops-lead`.
const HANDLE_ROLES: &[&str] = &[
    "supervisor",
    "lead",
    "exec",
    "staff",
    "agent",
    "worker",
    "writer",
    "reviewer",
    "bot",
    "ops",
];

/// Words that introduce a status code in a provider's error.
const STATUS_WORDS: &[&str] = &["http", "status", "code", "error", "returned"];

const EDGE: &[char] = &[
    '.', ',', ';', ':', '!', '?', '(', ')', '[', ']', '{', '}', '"', '\'', '`', '“', '”', '‘', '’',
];

fn core(word: &str) -> &str {
    word.trim_matches(EDGE)
}

/// 8+ hex characters with a digit and a letter among them (a commit, a short or full uuid), never
/// a word or a plain number such as a phone number.
fn is_hex_id(word: &str) -> bool {
    let hex: String = word.chars().filter(|c| *c != '-').collect();
    hex.len() >= 8
        && word.split('-').all(|part| !part.is_empty())
        && hex.chars().all(|c| c.is_ascii_hexdigit())
        && hex.chars().any(|c| c.is_ascii_digit())
        && hex.chars().any(|c| c.is_ascii_alphabetic())
}

/// `msg12`, `msg#12`, `message#12`.
fn is_glued_message_number(word: &str) -> bool {
    let lower = word.to_ascii_lowercase();
    ["msg", "message"].iter().any(|prefix| {
        lower.strip_prefix(prefix).is_some_and(|rest| {
            let digits = rest.strip_prefix('#').unwrap_or(rest);
            !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
        })
    })
}

fn is_message_word(word: &str) -> bool {
    matches!(word.to_ascii_lowercase().as_str(), "msg" | "message")
}

fn is_number(word: &str) -> bool {
    let digits = word.strip_prefix('#').unwrap_or(word);
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

/// An agent's handle: lowercase and hyphenated, ending in a role.
pub fn is_agent_handle(word: &str) -> bool {
    let Some(first) = word.chars().next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && word.contains('-')
        && word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && word.split('-').all(|part| !part.is_empty())
        && word
            .rsplit('-')
            .next()
            .is_some_and(|role| HANDLE_ROLES.contains(&role))
}

/// A code name such as `work_id` or `exec_startup_blocked`.
fn is_snake_case(word: &str) -> bool {
    word.contains('_')
        && word.starts_with(|c: char| c.is_ascii_lowercase())
        && !word.ends_with('_')
        && word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// A provider's or the runtime's own error rather than a sentence: a `[kind]` tag, a JSON body,
/// an `error:` label, a request id or an HTTP status code.
fn is_raw_error(text: &str) -> bool {
    let trimmed = text.trim_start();
    if trimmed.starts_with('[')
        && trimmed.find(']').is_some_and(|end| {
            trimmed[1..end]
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_' || c == '-')
        })
    {
        return true;
    }
    let lower = text.to_lowercase();
    if text.contains('{') || text.contains('}') || lower.contains("request id") {
        return true;
    }
    if lower.contains("error:") || lower.contains("error=") || lower.contains("exception") {
        return true;
    }
    let words: Vec<&str> = text.split_whitespace().map(core).collect();
    words.iter().enumerate().any(|(at, word)| {
        let status = word.len() == 3
            && word.chars().all(|c| c.is_ascii_digit())
            && matches!(word.as_bytes()[0], b'4' | b'5');
        status && (at == 0 || STATUS_WORDS.contains(&words[at - 1].to_ascii_lowercase().as_str()))
    })
}

/// True when a line carries ids, hashes, message numbers, agent handles, code names, engine
/// nouns or a raw provider error. `actor_ids` are the company's own ids, said only by name.
pub fn leaks_internals(text: &str, actor_ids: &[&str]) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    for (at, word) in words.iter().enumerate() {
        if word.contains("://") {
            continue;
        }
        let word = core(word);
        if is_hex_id(word)
            || is_glued_message_number(word)
            || is_agent_handle(word)
            || is_snake_case(word)
            || (is_message_word(word)
                && words.get(at + 1).is_some_and(|next| is_number(core(next))))
        {
            return true;
        }
        if actor_ids
            .iter()
            .any(|id| (id.contains('-') || id.contains('_')) && word == *id)
        {
            return true;
        }
    }
    // An address is read whole by the owner's browser, so its path is not a word they read.
    let plain = words
        .iter()
        .filter(|word| !word.contains("://"))
        .copied()
        .collect::<Vec<_>>()
        .join(" ");
    let lower = plain.to_lowercase();
    INTERNAL_NOUNS
        .iter()
        .any(|noun| contains_phrase(&lower, noun))
        || is_raw_error(&plain)
}

fn contains_phrase(lower: &str, phrase: &str) -> bool {
    lower.match_indices(phrase).any(|(at, _)| {
        let before = lower[..at].chars().next_back();
        let after = lower[at + phrase.len()..].chars().next();
        !before.is_some_and(|c| c.is_alphanumeric() || c == '_')
            && !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

/// Drops `( … )` and `[ … ]` asides that carry only internals: "(msg 1513)", "[44b3016c]".
fn without_internal_asides(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find(['(', '[']) {
        let close_char = if rest[open..].starts_with('(') {
            ')'
        } else {
            ']'
        };
        let Some(len) = rest[open + 1..].find(close_char) else {
            break;
        };
        let inner = &rest[open + 1..open + 1 + len];
        out.push_str(&rest[..open]);
        if leaks_internals(inner, &[]) && !inner.contains("://") {
            while out.ends_with(' ') {
                out.pop();
            }
        } else {
            out.push_str(&rest[open..open + 2 + len]);
        }
        rest = &rest[open + 2 + len..];
    }
    out.push_str(rest);
    out
}

/// An agent's own words with ids, hashes, message numbers and handles taken out. Known actor ids
/// become their names (`name_of`); an unknown handle becomes "a teammate"; a code name reads as
/// words. Addresses are left whole.
pub fn scrub(text: &str, name_of: &dyn Fn(&str) -> Option<String>) -> String {
    let text = without_internal_asides(text);
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    let mut skip_next = false;
    for (at, word) in words.iter().enumerate() {
        if skip_next {
            skip_next = false;
            // The number's own trailing punctuation stays with the sentence.
            let tail = word.trim_start_matches(|c: char| c == '#' || c.is_ascii_digit());
            if let Some(last) = out.last_mut() {
                last.push_str(tail);
            }
            continue;
        }
        if word.contains("://") {
            out.push((*word).to_string());
            continue;
        }
        let bare = core(word);
        let start = word.find(bare).unwrap_or(0);
        let (lead, tail) = (&word[..start], &word[start + bare.len()..]);
        let dropped = is_hex_id(bare) || is_glued_message_number(bare);
        if dropped {
            if let Some(last) = out.last_mut() {
                last.push_str(tail.trim_start_matches([')', ']']));
            }
            continue;
        }
        if is_message_word(bare) && words.get(at + 1).is_some_and(|next| is_number(core(next))) {
            skip_next = true;
            continue;
        }
        // Only an id-shaped word is a name to look up: "the owner" stays "the owner".
        let id_shaped = bare.contains('-') || bare.contains('_');
        let said = if let Some(name) = id_shaped.then(|| name_of(bare)).flatten() {
            name
        } else if is_agent_handle(bare) {
            "a teammate".into()
        } else if is_snake_case(bare) {
            bare.replace('_', " ")
        } else {
            bare.to_string()
        };
        out.push(format!("{lead}{said}{tail}"));
    }
    tidy(&out.join(" "))
}

fn tidy(text: &str) -> String {
    let mut said = text.to_string();
    for (from, to) in [
        (" ,", ","),
        (" .", "."),
        (" ;", ";"),
        (" :", ":"),
        ("()", ""),
        ("[]", ""),
        (",,", ","),
        (",.", "."),
        (":.", "."),
        ("  ", " "),
    ] {
        while said.contains(from) {
            said = said.replace(from, to);
        }
    }
    let said = said.trim().trim_start_matches([',', ';', ':', '.']).trim();
    let said = said.trim_end_matches([',', ';', ':']).trim_end();
    said.to_string()
}

/// A line an agent or a provider wrote, scrubbed for the owner, or the platform's own line when
/// nothing plain is left.
pub fn owner_text(text: &str, fallback: &str, name_of: &dyn Fn(&str) -> Option<String>) -> String {
    let said = scrub(text, name_of);
    if said.is_empty() || leaks_internals(&said, &[]) {
        fallback.to_string()
    } else {
        said
    }
}

/// [`owner_text`] for a line with no actor directory at hand.
pub fn owner_line(text: &str, fallback: &str) -> String {
    owner_text(text, fallback, &|_| None)
}

/// A readable name for a model id (`openai-codex/gpt-5.6-sol` → "GPT-5.6 Sol"), the same
/// fallback the web's `modelLabel` uses when the catalogue does not know it.
pub fn model_label(model: &str) -> String {
    let bare = model.split_once('/').map_or(model, |(_, rest)| rest);
    if bare.starts_with("native-codex-") || bare.starts_with("native-claude-") {
        return "the default model".into();
    }
    let parts: Vec<&str> = bare.split('-').filter(|part| !part.is_empty()).collect();
    let mut words: Vec<String> = Vec::new();
    let mut at = 0;
    while at < parts.len() {
        let part = parts[at];
        // "opus-5-5" -> "Opus 5.5"
        if part.chars().all(|c| c.is_ascii_digit())
            && parts
                .get(at + 1)
                .is_some_and(|next| next.chars().all(|c| c.is_ascii_digit()))
        {
            words.push(format!("{part}.{}", parts[at + 1]));
            at += 2;
            continue;
        }
        words.push(
            if matches!(part.to_ascii_lowercase().as_str(), "gpt" | "glm" | "oss") {
                part.to_ascii_uppercase()
            } else {
                let mut letters = part.chars();
                letters
                    .next()
                    .map(|first| first.to_uppercase().collect::<String>() + letters.as_str())
                    .unwrap_or_default()
            },
        );
        at += 1;
    }
    let label = words.join(" ");
    match label.strip_prefix("GPT ") {
        Some(rest) if rest.starts_with(|c: char| c.is_ascii_digit()) => format!("GPT-{rest}"),
        _ => label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /* The record that reached the owner on 3 Nov 2026, word for word (owner-copy.test.ts). */
    const ABANDONED: &str = "Withdrawn because Work was abandoned by reno-supervisor: Owner msg 1513 … supersedes the v3 digests this delivery was bound to; replaced by rewrite 44b3016c, check 4 a81cc8b2 and a new delivery node.";

    fn names(id: &str) -> Option<String> {
        (id == "reno-supervisor").then(|| "Reno".to_string())
    }

    fn assert_owner_safe(text: &str) {
        assert!(!leaks_internals(text, &[]), "leaks internals: {text:?}");
        for word in text.split_whitespace().map(core) {
            assert!(!is_hex_id(word), "hex id in {text:?}");
            assert!(!is_agent_handle(word), "handle in {text:?}");
            assert!(!is_snake_case(word), "code name in {text:?}");
        }
    }

    #[test]
    fn the_guard_sees_every_kind_of_internal() {
        for leak in [
            ABANDONED,
            "Waiting on reno-supervisor",
            "Reply to msg 42 first",
            "Reply to msg#42 first",
            "Rebased onto 44b3016c",
            "Blocked on 6f1d2c4e-1a2b-4c3d-9e8f-0a1b2c3d4e5f",
            "The exec_startup_blocked state",
            "[quota] provider refused the request",
            "402 Payment Required",
            "provider returned status 429",
            "{\"error\":{\"type\":\"invalid_request\"}}",
            "Error: connection reset",
        ] {
            assert!(leaks_internals(leak, &[]), "missed {leak:?}");
        }
        for plain in [
            "Approve sending the four prospect emails once sending is switched on",
            "An image is chosen for the landing page.",
            "Raise the spend limit to $40 so the launch can finish this week.",
            "Pay Acme Design for the logo work",
            "Decided in 2026 by the team",
            "Call them on 0412345678 after nine",
        ] {
            assert!(!leaks_internals(plain, &[]), "flagged {plain:?}");
        }
        assert!(leaks_internals("Ask human_bodhi", &["human_bodhi"]));
    }

    #[test]
    fn an_agents_words_stay_but_lose_ids_and_handles() {
        assert_eq!(
            scrub(
                "Raise the limit so reno-supervisor can finish (msg 1513).",
                &names
            ),
            "Raise the limit so Reno can finish."
        );
        assert_eq!(
            scrub(
                "Needed for the launch post, rewrite 44b3016c, by Friday",
                &|_| None
            ),
            "Needed for the launch post, rewrite, by Friday"
        );
        assert_eq!(
            scrub("Ops-lead note: ask ops-lead before paying", &|_| None),
            "Ops-lead note: ask a teammate before paying"
        );
        assert_eq!(
            scrub("Waiting for the image_choice step", &|_| None),
            "Waiting for the image choice step"
        );
        // Only an id-shaped word is looked up: "owner" is a word, not the actor `owner`.
        assert_eq!(
            scrub("The owner confirmed the step", &|id| Some(format!(
                "<{id}>"
            ))),
            "The owner confirmed the step"
        );
        assert!(leaks_internals("a teammate confirms attempt id", &[]));
        assert_eq!(
            scrub(
                "See https://example.com/a1b2c3d4e5 for msg 12 details",
                &|_| None
            ),
            "See https://example.com/a1b2c3d4e5 for details"
        );
    }

    #[test]
    fn nothing_plain_left_falls_back_to_the_platforms_line() {
        let said = owner_text(ABANDONED, "Withdrawn: no longer needed.", &names);
        assert_eq!(said, "Withdrawn: no longer needed.");
        assert_owner_safe(&said);
        assert_eq!(
            owner_line(
                "[quota] provider refused: 402 AI credit is used up",
                "Out of credit."
            ),
            "Out of credit."
        );
        assert_eq!(owner_line("   ", "Fallback."), "Fallback.");
        assert_eq!(owner_line("(44b3016c)", "Fallback."), "Fallback.");
    }

    #[test]
    fn a_model_id_reads_as_its_name() {
        assert_eq!(model_label("openai-codex/gpt-5.6-sol"), "GPT-5.6 Sol");
        assert_eq!(model_label("anthropic/claude-opus-5-5"), "Claude Opus 5.5");
        assert_eq!(model_label("native-codex-default"), "the default model");
    }
}
