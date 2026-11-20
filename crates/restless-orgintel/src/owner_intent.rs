//! What kind of message the owner sent, and how much friction the owner has with the company
//! (migration 0125; docs/specs/owner-interaction.md D3 and D7).
//!
//! Every message from the owner (or another human in the company) is put in one class by fixed
//! rules: no model, no cost, the same answer every time. The class is stored on the message the
//! first time the company's numbers are read, so a new message is classified on the next turn
//! that reads them and an old one is never classified twice.
//!
//! The rules are deliberately narrow. A whole-message list catches what the owner actually types
//! ("progress?" 609 times in their own records); a phrase list catches the rest. A message the
//! rules cannot place is an `instruction` when it reads like a request and `other` when it does
//! not, so a wrong guess lands in the class that changes nothing downstream.
//!
//! The numbers are reads over those classes plus reopened and landed Work. Deciding when they
//! mean the company should review itself is the engine's (`restless_engine::org_review`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row as _;

use crate::{OrgIntel, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerIntent {
    /// A request with a method, or a plain one ("rename the button to Save").
    Instruction,
    /// An outcome with no method ("fix all root causes properly").
    OutcomeOnly,
    /// Why, how, or look and report.
    Question,
    /// A correction: "no", "that's wrong", "I said", "still broken".
    Redirect,
    /// "progress?", "status", "where are we", "eta".
    StatusCheck,
    /// "yes", "approved", "go", "do it", or a numbered answer.
    Approval,
    /// "stop", "don't code", "hold".
    Stop,
    Other,
}

impl OwnerIntent {
    pub const ALL: [OwnerIntent; 8] = [
        OwnerIntent::Instruction,
        OwnerIntent::OutcomeOnly,
        OwnerIntent::Question,
        OwnerIntent::Redirect,
        OwnerIntent::StatusCheck,
        OwnerIntent::Approval,
        OwnerIntent::Stop,
        OwnerIntent::Other,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            OwnerIntent::Instruction => "instruction",
            OwnerIntent::OutcomeOnly => "outcome_only",
            OwnerIntent::Question => "question",
            OwnerIntent::Redirect => "redirect",
            OwnerIntent::StatusCheck => "status_check",
            OwnerIntent::Approval => "approval",
            OwnerIntent::Stop => "stop",
            OwnerIntent::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Option<OwnerIntent> {
        OwnerIntent::ALL
            .into_iter()
            .find(|intent| intent.as_str() == value)
    }
}

/// The message as the rules read it: lower case, apostrophes dropped ("don't" is "dont"), every
/// other mark a space, one space between words.
fn normalise(body: &str) -> String {
    let lowered = body.trim().to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    for c in lowered.chars() {
        if c == '\'' || c == '\u{2019}' || c == '\u{2018}' {
            continue;
        }
        out.push(if c.is_alphanumeric() { c } else { ' ' });
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `phrase` appears in `text` as whole words.
fn has(text: &str, phrase: &str) -> bool {
    format!(" {text} ").contains(&format!(" {phrase} "))
}

fn starts(text: &str, phrase: &str) -> bool {
    text == phrase || text.starts_with(&format!("{phrase} "))
}

fn any_has(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| has(text, phrase))
}

fn any_starts(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| starts(text, phrase))
}

const STOP_WHOLE: &[&str] = &[
    "stop",
    "stop it",
    "stop now",
    "stop that",
    "stop please",
    "please stop",
    "stop stop",
    "hold",
    "hold on",
    "hold it",
    "pause",
    "pause it",
    "wait",
    "halt",
    "cancel",
    "cancel that",
    "cancel it",
    "enough",
    "stand down",
    "freeze",
];

const STOP_PHRASES: &[&str] = &[
    "stop coding",
    "stop building",
    "stop working",
    "stop everything",
    "stop all work",
    "stop editing",
    "stop making changes",
    "dont code",
    "do not code",
    "no coding",
    "dont build",
    "do not build",
    "dont touch",
    "do not touch",
    "dont change anything",
    "do not change anything",
    "dont make changes",
    "dont make any changes",
    "do not make any changes",
    "no changes yet",
    "hold off",
    "dont start",
    "dont do anything",
];

/// The owner asking where things are, as a whole message.
const STATUS_WHOLE: &[&str] = &[
    "progress",
    "any progress",
    "progress update",
    "hows progress",
    "status",
    "status update",
    "status report",
    "any update",
    "any updates",
    "updates",
    "hows it going",
    "how is it going",
    "how are we doing",
    "hows it looking",
    "how is it looking",
    "where are we",
    "where are we at",
    "where are we now",
    "wheres this at",
    "where is it at",
    "whats happening",
    "whats going on",
    "whats the status",
    "what is the status",
    "whats the progress",
    "eta",
    "whats the eta",
    "any eta",
    "how long",
    "how much longer",
    "how long left",
    "done yet",
    "are you done",
    "is it done",
    "finished yet",
    "is it ready",
    "ready yet",
    "any news",
    "still there",
    "are you there",
    "still working",
    "still working on it",
    "still waiting",
    "how far along",
    "sitrep",
];

/// Single words that are a status check only when asked: "done?" is, "done" is the owner saying
/// they did their part.
const STATUS_IF_ASKED: &[&str] = &["done", "finished", "ready", "update", "news", "report"];

/// Short questions about one thing's progress: "status of the deploy?", "eta on the fix?".
const STATUS_LEADS: &[&str] = &[
    "status of",
    "status on",
    "progress on",
    "progress with",
    "any update on",
    "any updates on",
    "update on",
    "eta on",
    "eta for",
    "eta of",
    "where are we on",
    "where are we with",
    "how far",
];

const REDIRECT_LEADS: &[&str] = &[
    "no",
    "nope",
    "nah",
    "wrong",
    "not that",
    "not quite",
    "thats wrong",
    "thats not",
    "that is not",
    "that isnt",
    "this is wrong",
    "this is not",
    "i said",
    "i told you",
    "i asked",
    "i meant",
    "i already",
    "actually",
    "undo",
    "revert that",
    "revert it",
    "you broke",
    "you didnt",
    "you did not",
    "you missed",
    "you forgot",
    "still",
    "it still",
    "its still",
    "didnt work",
    "doesnt work",
    "not working",
    "same problem",
    "same issue",
    "same error",
    "again",
    "dont do that",
    "instead",
];

/// "no" that is not a correction.
const NOT_REDIRECT: &[&str] = &[
    "no problem",
    "no worries",
    "no rush",
    "no thanks",
    "no thank you",
    "no need to rush",
    "still there",
    "still waiting",
    "still working",
];

const REDIRECT_PHRASES: &[&str] = &[
    "i said",
    "i told you",
    "i already said",
    "i already told",
    "as i said",
    "like i said",
    "thats wrong",
    "thats not what",
    "not what i asked",
    "not what i meant",
    "not what i said",
    "thats not right",
    "you broke",
    "still broken",
    "still not working",
    "still doesnt work",
    "still failing",
    "still the same",
    "didnt fix",
    "did not fix",
    "not fixed",
    "isnt fixed",
    "wasnt fixed",
    "you ignored",
    "why did you",
    "dont do that",
    "stop doing",
    "read what i",
];

const APPROVAL_WHOLE: &[&str] = &[
    "yes",
    "y",
    "yep",
    "yup",
    "yeah",
    "ya",
    "yes please",
    "yes do it",
    "yes go",
    "yes go ahead",
    "ok",
    "okay",
    "k",
    "kk",
    "ok go",
    "ok do it",
    "okay do it",
    "sure",
    "sure go ahead",
    "approved",
    "approve",
    "i approve",
    "go",
    "go go",
    "go ahead",
    "go for it",
    "do it",
    "do that",
    "ship it",
    "lgtm",
    "looks good",
    "looks good to me",
    "sounds good",
    "agreed",
    "agree",
    "proceed",
    "please proceed",
    "confirmed",
    "confirm",
    "accepted",
    "accept",
    "merge it",
    "yes merge",
    "both",
    "all of them",
];

/// What the owner says to go on with work already under way. In answer to a check-in it is an
/// approval; out of nowhere it is the owner pushing stalled work, an instruction.
const CARRY_ON: &[&str] = &[
    "continue",
    "keep going",
    "go on",
    "carry on",
    "resume",
    "proceed",
];

/// Acknowledgements and greetings: no request in them.
const OTHER_WHOLE: &[&str] = &[
    "thanks",
    "thank you",
    "thx",
    "ty",
    "cheers",
    "nice",
    "cool",
    "great",
    "awesome",
    "perfect",
    "good",
    "good job",
    "nice work",
    "well done",
    "love it",
    "lol",
    "haha",
    "hi",
    "hello",
    "hey",
    "morning",
    "good morning",
    "gm",
    "good night",
    "night",
    "brb",
    "ok thanks",
    "okay thanks",
    "ok cool",
    "wow",
    "hmm",
];

const WH_WORDS: &[&str] = &[
    "why", "how come", "what", "whats", "which", "who", "whose", "where", "wheres", "when", "how",
    "hows",
];

const AUX_WORDS: &[&str] = &[
    "is", "are", "was", "were", "does", "do", "did", "has", "have", "will", "should", "can",
    "could", "would", "isnt", "arent", "doesnt", "didnt",
];

/// A request phrased politely: "can you fix", "could we ship". A question only when what follows
/// is itself looking, not doing.
const POLITE: &[&str] = &[
    "can you",
    "could you",
    "would you",
    "will you",
    "can we",
    "could we",
    "please",
];

const LOOK_VERBS: &[&str] = &[
    "explain",
    "tell me",
    "look",
    "have a look",
    "take a look",
    "check why",
    "check whether",
    "check if",
    "investigate",
    "see why",
    "find out",
    "figure out why",
    "work out why",
    "confirm whether",
    "confirm if",
    "diagnose",
    "report",
];

/// Look and report, with no change asked for.
const QUESTION_PHRASES: &[&str] = &[
    "look and report",
    "report back",
    "tell me why",
    "tell me what",
    "tell me how",
    "tell me whether",
    "explain why",
    "explain how",
    "explain what",
    "investigate",
    "find out why",
    "find out what",
    "figure out why",
    "work out why",
    "find the root cause",
    "dont fix",
    "do not fix",
    "just look",
    "just check",
    "diagnose",
];

const AND_FIX: &[&str] = &["and fix", "then fix", "and then fix", "fix it", "and solve"];

/// An outcome asked for in words that name no method.
const OUTCOME_PHRASES: &[&str] = &[
    "properly",
    "root cause",
    "root causes",
    "fix all",
    "fix everything",
    "fix every",
    "all the bugs",
    "all bugs",
    "every bug",
    "everything works",
    "everything working",
    "make sure",
    "get it working",
    "get this working",
    "working properly",
    "not suck",
    "suck less",
    "robust",
    "reliable",
    "reliably",
    "bulletproof",
    "rock solid",
    "production ready",
    "end to end",
    "sort out",
    "sort it out",
    "clean up",
    "clean it up",
    "better",
    "improve",
    "eliminate",
    "once and for all",
    "for good",
    "the right way",
    "do it right",
    "polish",
    "world class",
    "seamless",
];

/// Words in a message that name how: a step, a tool, a place.
const METHOD_PHRASES: &[&str] = &[
    "by",
    "using",
    "via",
    "instead of",
    "use the",
    "with the",
    "rename",
    "replace",
    "change the",
    "set the",
    "step 1",
    "first",
    "then",
];

const VERBS: &[&str] = &[
    "fix",
    "add",
    "make",
    "build",
    "write",
    "create",
    "update",
    "change",
    "rename",
    "remove",
    "delete",
    "move",
    "run",
    "deploy",
    "ship",
    "test",
    "check",
    "send",
    "post",
    "publish",
    "draft",
    "merge",
    "release",
    "set",
    "install",
    "open",
    "close",
    "use",
    "try",
    "retry",
    "restart",
    "rebuild",
    "refactor",
    "implement",
    "finish",
    "start",
    "lets",
    "put",
    "push",
    "pull",
    "clean",
    "sort",
    "plan",
    "design",
    "research",
    "find",
    "get",
    "give",
    "show",
    "email",
    "reply",
    "schedule",
    "book",
    "call",
    "upgrade",
    "bump",
    "queue",
    "continue",
    "resume",
];

/// Something in the raw text that names a concrete thing: a path, code, a quoted string, an
/// issue number, an identifier.
fn names_a_thing(raw: &str) -> bool {
    if raw.contains('`')
        || raw.contains("::")
        || raw.contains('"')
        || raw.contains('\u{201c}')
        || raw.contains('/')
        || raw.contains('\\')
    {
        return true;
    }
    raw.split_whitespace().any(|token| {
        let token = token.trim_matches(|c: char| !c.is_alphanumeric());
        let chars: Vec<char> = token.chars().collect();
        let dotted = chars
            .windows(3)
            .any(|w| w[0].is_alphanumeric() && w[1] == '.' && w[2].is_alphabetic());
        let snake = chars
            .windows(3)
            .any(|w| w[0].is_alphanumeric() && w[1] == '_' && w[2].is_alphanumeric());
        let camel = chars
            .windows(2)
            .any(|w| w[0].is_lowercase() && w[1].is_uppercase());
        dotted || snake || camel
    }) || raw.split_whitespace().any(|token| {
        token
            .strip_prefix('#')
            .is_some_and(|rest| rest.chars().next().is_some_and(|c| c.is_ascii_digit()))
    })
}

/// A numbered answer as written: "1", "2.", "1) yes", "3: the old one".
fn numbered(raw: &str) -> bool {
    let digits = raw.chars().take_while(|c| c.is_ascii_digit()).count();
    digits > 0 && matches!(raw[digits..].chars().next(), None | Some('.' | ')' | ':'))
}

/// `make <a few words> work`: "make the importer work".
fn make_it_work(text: &str) -> bool {
    let words: Vec<&str> = text.split(' ').collect();
    words.iter().enumerate().any(|(at, word)| {
        *word == "make"
            && words[at + 1..]
                .iter()
                .take(4)
                .any(|next| matches!(*next, "work" | "works" | "working"))
    })
}

fn first_is_number(text: &str) -> bool {
    text.split(' ')
        .next()
        .is_some_and(|word| !word.is_empty() && word.chars().all(|c| c.is_ascii_digit()))
}

/// Put one owner message in one class. `answers_checkin` is true for the owner's first message
/// after a check-in Exec posted: "continue" or "1. yes" there answers it.
pub fn classify_owner_message(body: &str, answers_checkin: bool) -> OwnerIntent {
    let raw = body.trim();
    if matches!(raw, "\u{1f44d}" | "\u{2705}" | "\u{1f44c}") {
        return OwnerIntent::Approval;
    }
    let text = normalise(raw);
    if text.is_empty() {
        return OwnerIntent::Other;
    }
    let asked = raw.ends_with('?');
    let words = text.split(' ').count();

    let approval_words = APPROVAL_WHOLE.contains(&text.as_str())
        || ((starts(&text, "yes") || starts(&text, "approved") || starts(&text, "ok"))
            && words <= 4
            && !has(&text, "but")
            && !starts(&text, "ok so")
            && !OTHER_WHOLE.contains(&text.as_str()))
        || (starts(&text, "go with") && words <= 5)
        || (starts(&text, "option") && words <= 3);
    if answers_checkin
        && (approval_words
            || CARRY_ON.contains(&text.as_str())
            || first_is_number(&text)
            || OTHER_WHOLE.contains(&text.as_str()))
    {
        return OwnerIntent::Approval;
    }

    if STOP_WHOLE.contains(&text.as_str()) || any_has(&text, STOP_PHRASES) {
        return OwnerIntent::Stop;
    }
    if STATUS_WHOLE.contains(&text.as_str()) || (asked && STATUS_IF_ASKED.contains(&text.as_str()))
    {
        return OwnerIntent::StatusCheck;
    }
    if !any_starts(&text, NOT_REDIRECT)
        && (any_starts(&text, REDIRECT_LEADS) || any_has(&text, REDIRECT_PHRASES))
    {
        return OwnerIntent::Redirect;
    }
    if approval_words || (numbered(raw) && words <= 8) {
        return OwnerIntent::Approval;
    }
    if words <= 7
        && ((asked && any_starts(&text, STATUS_LEADS))
            || has(&text, "eta")
            || (asked
                && (starts(&text, "how is the") || starts(&text, "hows the"))
                && text.ends_with(" going")))
    {
        return OwnerIntent::StatusCheck;
    }

    let fixes_too = any_has(&text, AND_FIX);
    let polite = POLITE.iter().find(|lead| starts(&text, lead));
    let question = match polite {
        Some(lead) => {
            let rest = text[lead.len()..].trim_start();
            LOOK_VERBS.iter().any(|verb| starts(rest, verb))
        }
        None => {
            any_starts(&text, WH_WORDS)
                || (asked && any_starts(&text, AUX_WORDS))
                || LOOK_VERBS.iter().any(|verb| starts(&text, verb))
        }
    } || any_has(&text, QUESTION_PHRASES);
    if question && !fixes_too {
        return OwnerIntent::Question;
    }

    let outcome = any_has(&text, OUTCOME_PHRASES) || make_it_work(&text);
    if outcome && !names_a_thing(raw) && !any_has(&text, METHOD_PHRASES) {
        return OwnerIntent::OutcomeOnly;
    }

    if OTHER_WHOLE.contains(&text.as_str()) {
        return OwnerIntent::Other;
    }
    let first = text.split(' ').next().unwrap_or_default();
    if words >= 3 || VERBS.contains(&first) {
        OwnerIntent::Instruction
    } else {
        OwnerIntent::Other
    }
}

/// The owner's first message after a check-in Exec posted, once Exec has spoken since the
/// check-in: an answer to it, not a nudge they had to give. Reads `message` as the owner's row.
pub(crate) const ANSWERS_CHECKIN: &str = "EXISTS ( \
   SELECT 1 FROM events checkin \
   JOIN messages said ON said.from_actor='exec' \
     AND (said.to_actor IS NULL OR said.to_actor=message.from_actor) \
     AND said.id > (checkin.body->>'message_mark')::bigint \
     AND said.id < message.id \
   WHERE checkin.kind='exec_checkin' \
     AND NOT EXISTS (SELECT 1 FROM messages earlier \
       WHERE earlier.from_actor=message.from_actor AND earlier.id < message.id \
         AND earlier.id > (checkin.body->>'message_mark')::bigint))";

/// Messages written by the owner or another human. Reads `author` as the sender.
pub(crate) const BY_A_HUMAN: &str = "(author.kind='owner' OR author.actor_class='human')";

/// At most this many messages are classified per read; a company with a long history catches up
/// over a few reads instead of one long one.
const CLASSIFY_BATCH: i64 = 1000;
/// Ids are handed out before commit, so a message can commit behind one already classified.
/// Each read looks this far back from the newest classified message for any it skipped.
const CLASSIFY_SLACK: i64 = 200;

/// The owner's friction with the company over a window (D7). Counts are of owner messages unless
/// named otherwise. A run is a redirect whose next owner message on the same thread is another
/// redirect. `new_*` counts only what happened after the last review, so a number that stays
/// high does not make the company review itself again and again.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct OwnerFriction {
    pub owner_messages: i64,
    pub status_checks: i64,
    pub outcome_only: i64,
    /// Outcome-only requests whose next owner message on the thread was a redirect.
    pub outcome_only_redirected: i64,
    pub instructions: i64,
    pub instructions_redirected: i64,
    pub redirects: i64,
    pub redirect_runs: i64,
    pub work_reopened: i64,
    /// Work whose Attempt produced its outcome in the window and that is now done.
    pub work_landed: i64,
    pub new_status_checks: i64,
    pub new_redirect_runs: i64,
    pub new_work_reopened: i64,
}

impl OrgIntel {
    /// Classify owner messages not classified yet, oldest first. Returns how many were stored.
    pub async fn classify_owner_messages(&self) -> Result<usize> {
        let rows = sqlx::query(&format!(
            "SELECT message.id, message.body, {ANSWERS_CHECKIN} AS answers_checkin \
             FROM messages message \
             JOIN actors author ON author.id=message.from_actor \
             WHERE {BY_A_HUMAN} AND message.owner_intent IS NULL \
               AND message.deleted_at IS NULL \
               AND message.id > COALESCE((SELECT max(id) FROM messages \
                                          WHERE owner_intent IS NOT NULL), 0) - $1 \
             ORDER BY message.id LIMIT $2"
        ))
        .bind(CLASSIFY_SLACK)
        .bind(CLASSIFY_BATCH)
        .fetch_all(&self.pool)
        .await?;
        if rows.is_empty() {
            return Ok(0);
        }
        let mut ids = Vec::with_capacity(rows.len());
        let mut intents = Vec::with_capacity(rows.len());
        for row in &rows {
            ids.push(row.get::<i64, _>("id"));
            let body: String = row.get("body");
            let answers: bool = row.get("answers_checkin");
            intents.push(classify_owner_message(&body, answers).as_str().to_string());
        }
        let stored = sqlx::query(
            "UPDATE messages SET owner_intent=data.intent \
             FROM unnest($1::bigint[], $2::text[]) AS data(id, intent) \
             WHERE messages.id=data.id AND messages.owner_intent IS NULL",
        )
        .bind(&ids)
        .bind(&intents)
        .execute(&self.pool)
        .await?
        .rows_affected();
        Ok(stored as usize)
    }

    /// The stored class of one message, if it has been classified.
    pub async fn message_owner_intent(&self, message_id: i64) -> Result<Option<OwnerIntent>> {
        let value: Option<Option<String>> =
            sqlx::query_scalar("SELECT owner_intent FROM messages WHERE id=$1")
                .bind(message_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(value.flatten().as_deref().and_then(OwnerIntent::parse))
    }

    /// The owner's friction since `since`, with `new_*` counted after `after`. Classifies any
    /// owner message not classified yet first, so the numbers always include the newest.
    pub async fn owner_friction(
        &self,
        since: DateTime<Utc>,
        after: DateTime<Utc>,
    ) -> Result<OwnerFriction> {
        while self.classify_owner_messages().await? as i64 >= CLASSIFY_BATCH {}
        Ok(sqlx::query_as(&format!(
            "WITH said AS ( \
               SELECT message.created_at, message.owner_intent AS intent, \
                      LEAD(message.owner_intent) OVER thread AS next_intent, \
                      LEAD(message.created_at) OVER thread AS next_at \
               FROM messages message \
               JOIN actors author ON author.id=message.from_actor \
               WHERE {BY_A_HUMAN} AND message.owner_intent IS NOT NULL \
                 AND message.deleted_at IS NULL AND message.created_at >= $1 \
               WINDOW thread AS ( \
                 PARTITION BY COALESCE(message.room_id::text, 'to:' || COALESCE(message.to_actor, '')), \
                              message.thread_root_message_id \
                 ORDER BY message.id) \
             ) \
             SELECT COUNT(*) AS owner_messages, \
                    COUNT(*) FILTER (WHERE intent='status_check') AS status_checks, \
                    COUNT(*) FILTER (WHERE intent='outcome_only') AS outcome_only, \
                    COUNT(*) FILTER (WHERE intent='outcome_only' AND next_intent='redirect') \
                      AS outcome_only_redirected, \
                    COUNT(*) FILTER (WHERE intent='instruction') AS instructions, \
                    COUNT(*) FILTER (WHERE intent='instruction' AND next_intent='redirect') \
                      AS instructions_redirected, \
                    COUNT(*) FILTER (WHERE intent='redirect') AS redirects, \
                    COUNT(*) FILTER (WHERE intent='redirect' AND next_intent='redirect') \
                      AS redirect_runs, \
                    (SELECT COUNT(*) FROM events \
                      WHERE kind='work_reopened' AND created_at >= $1) AS work_reopened, \
                    (SELECT COUNT(DISTINCT attempt.work_id) FROM work_attempts attempt \
                      JOIN work ON work.id=attempt.work_id AND work.status='completed' \
                      WHERE attempt.state='produced' AND attempt.finished_at >= $1) AS work_landed, \
                    COUNT(*) FILTER (WHERE intent='status_check' AND created_at > $2) \
                      AS new_status_checks, \
                    COUNT(*) FILTER (WHERE intent='redirect' AND next_intent='redirect' \
                      AND next_at > $2) AS new_redirect_runs, \
                    (SELECT COUNT(*) FROM events \
                      WHERE kind='work_reopened' AND created_at > $2) AS new_work_reopened \
             FROM said"
        ))
        .bind(since)
        .bind(after)
        .fetch_one(&self.pool)
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use OwnerIntent::*;

    fn check(cases: &[(&str, OwnerIntent)]) {
        for (body, want) in cases {
            assert_eq!(
                classify_owner_message(body, false),
                *want,
                "{body:?} should be {want:?}"
            );
        }
    }

    #[test]
    fn the_reports_own_examples() {
        check(&[
            ("fix all root causes properly", OutcomeOnly),
            ("make onboarding not suck", OutcomeOnly),
            ("fix all the stuck jobs", OutcomeOnly),
            ("rename the button to Save", Instruction),
            ("progress?", StatusCheck),
            ("look and report", Question),
            ("why?", Question),
            ("stop coding", Stop),
            ("don't code", Stop),
            ("no", Redirect),
            ("that's wrong", Redirect),
            ("I said use the old token", Redirect),
            ("still broken", Redirect),
            ("status", StatusCheck),
            ("where are we", StatusCheck),
            ("eta", StatusCheck),
            ("yes", Approval),
            ("approved", Approval),
            ("go", Approval),
            ("do it", Approval),
            ("stop", Stop),
            ("hold", Stop),
        ]);
    }

    #[test]
    fn status_checks_are_short_and_about_where_things_are() {
        check(&[
            ("Progress?", StatusCheck),
            ("any update?", StatusCheck),
            ("how's it going?", StatusCheck),
            ("where are we at?", StatusCheck),
            ("ETA?", StatusCheck),
            ("eta on the fix", StatusCheck),
            ("status of the deploy?", StatusCheck),
            ("how is the launch going?", StatusCheck),
            ("done?", StatusCheck),
            ("still there?", StatusCheck),
            ("still waiting", StatusCheck),
            // Not status checks: a brief that mentions status, and "done" as in "I did it".
            ("send me the status report for Q3", Instruction),
            ("progress on the launch is what matters most", Instruction),
            ("done", Other),
        ]);
    }

    #[test]
    fn redirects_are_corrections_and_not_every_no() {
        check(&[
            ("No, the other repo", Redirect),
            ("nope", Redirect),
            ("that's not what I asked for", Redirect),
            ("you broke the login page", Redirect),
            ("it still fails on mobile", Redirect),
            ("still not working", Redirect),
            ("why did you delete the tests?", Redirect),
            ("actually use Postgres", Redirect),
            ("as I said, weekdays only", Redirect),
            ("no worries", Other),
            ("no rush", Other),
        ]);
    }

    #[test]
    fn approvals_and_numbered_answers() {
        check(&[
            ("Yes please", Approval),
            ("ok go", Approval),
            ("go ahead", Approval),
            ("LGTM", Approval),
            ("ship it", Approval),
            ("1", Approval),
            ("1. yes 2. wait for the new token", Approval),
            ("2) the old one", Approval),
            ("go with option 2", Approval),
            ("\u{1f44d}", Approval),
            // A yes with a change attached is an instruction, not a yes.
            ("yes but use the staging database first", Instruction),
        ]);
    }

    #[test]
    fn stops_end_work_and_a_stop_inside_a_brief_does_not() {
        check(&[
            ("STOP", Stop),
            ("wait", Stop),
            ("hold off on the release", Stop),
            ("don't touch the billing code", Stop),
            ("look into it but don't change anything", Stop),
            ("stop the stuck staging server", Instruction),
        ]);
    }

    #[test]
    fn questions_look_and_report() {
        check(&[
            ("why is the deploy failing?", Question),
            ("how does the importer pick a vendor?", Question),
            ("what broke overnight", Question),
            ("can you explain the spend spike?", Question),
            ("investigate the 502s", Question),
            ("is the cache warm?", Question),
            ("find the root cause of the timeouts", Question),
            ("look at the logs and report back", Question),
            // Looking with a fix attached is work.
            ("investigate the 502s and fix them", Instruction),
            // A polite request is a request.
            ("can you fix the login?", Instruction),
        ]);
    }

    #[test]
    fn an_outcome_with_a_method_is_an_instruction() {
        check(&[
            ("make the importer work", OutcomeOnly),
            ("make sure checkout is reliable", OutcomeOnly),
            ("clean up the onboarding", OutcomeOnly),
            ("fix all lint errors in src/main.rs", Instruction),
            ("fix all the failing tests by pinning chrono", Instruction),
            ("improve load time of `/pricing`", Instruction),
            ("fix #742 properly", Instruction),
            ("add a Save button to the settings page", Instruction),
            ("deploy", Instruction),
        ]);
    }

    #[test]
    fn an_answer_to_a_checkin_is_an_approval() {
        for body in ["continue", "go on", "1", "1. wait", "yes", "great"] {
            assert_eq!(classify_owner_message(body, true), Approval, "{body}");
        }
        // Out of nowhere, "continue" is the owner pushing stalled work.
        assert_eq!(classify_owner_message("continue", false), Instruction);
        // A real message after a check-in keeps its own class.
        assert_eq!(classify_owner_message("progress?", true), StatusCheck);
        assert_eq!(classify_owner_message("that's wrong", true), Redirect);
    }

    #[test]
    fn empty_and_chatter_is_other() {
        check(&[
            ("", Other),
            ("   ", Other),
            ("thanks!", Other),
            ("good morning", Other),
            ("hmm", Other),
        ]);
    }

    #[test]
    fn every_class_round_trips_through_its_stored_name() {
        for intent in OwnerIntent::ALL {
            assert_eq!(OwnerIntent::parse(intent.as_str()), Some(intent));
            assert_eq!(
                serde_json::to_value(intent).unwrap(),
                serde_json::Value::String(intent.as_str().into())
            );
        }
        assert_eq!(OwnerIntent::parse("nudge"), None);
    }
}
