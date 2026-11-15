//! What a piece of Work rests on (Sprint 117). A question about direction has to come from
//! something written down, and nothing was: assumptions and unknowns lived only in a model's
//! head for one turn.
//!
//! The carrier is the Work's own outcome text, which the lead and every worker on that Work
//! already read. Whoever commissions it ends the outcome with:
//!
//! ```text
//! Assumptions:
//! - Local firms answer cold email.
//! - Twelve prospects is enough to learn from.
//! Least sure of: whether price or timing decides it.
//! ```
//!
//! No table, no statement kinds, no confidence and no expiry. This module only reads that
//! section back, without a model, so the weekly direction session can put it in front of Exec.

/// How the assumptions section starts, on a line of its own or followed by one assumption.
pub const ASSUMPTIONS_HEADING: &str = "Assumptions:";
/// How the one key uncertainty is written, on one line.
pub const LEAST_SURE_PREFIX: &str = "Least sure of:";
/// Assumptions kept from one Work; more than this is a list nobody will read.
pub const MAX_ASSUMPTIONS: usize = 3;
const STATEMENT_CHARS: usize = 200;

/// What one Work's outcome says it rests on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkAssumptions {
    /// At most [`MAX_ASSUMPTIONS`], in the order written.
    pub assumptions: Vec<String>,
    /// The one thing whoever commissioned it was least sure of.
    pub least_sure: Option<String>,
}

impl WorkAssumptions {
    pub fn is_empty(&self) -> bool {
        self.assumptions.is_empty() && self.least_sure.is_none()
    }
}

fn strip_prefix_ignoring_case<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let head = line.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| line[prefix.len()..].trim())
}

/// A list item's text: `- text`, `* text`, `• text` or `1. text`.
fn bullet(line: &str) -> Option<&str> {
    if let Some(rest) = ["- ", "* ", "• "]
        .iter()
        .find_map(|mark| line.strip_prefix(mark))
    {
        return Some(rest.trim());
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    line[digits..]
        .strip_prefix(". ")
        .or_else(|| line[digits..].strip_prefix(") "))
        .map(str::trim)
}

fn statement(text: &str) -> Option<String> {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return None;
    }
    let mut kept: String = text.chars().take(STATEMENT_CHARS).collect();
    if text.chars().count() > STATEMENT_CHARS {
        kept.push('…');
    }
    Some(kept)
}

/// Read the assumptions and the key uncertainty out of a Work outcome. Anything not in the
/// shape above is ignored: this never guesses at prose.
pub fn read_assumptions(outcome: &str) -> WorkAssumptions {
    let mut found = WorkAssumptions::default();
    let mut listing = false;
    for line in outcome.lines().map(str::trim) {
        if let Some(rest) = strip_prefix_ignoring_case(line, LEAST_SURE_PREFIX) {
            listing = false;
            if found.least_sure.is_none() {
                found.least_sure = statement(rest);
            }
        } else if let Some(rest) = strip_prefix_ignoring_case(line, ASSUMPTIONS_HEADING) {
            listing = true;
            found.assumptions.extend(statement(rest));
        } else if listing {
            match bullet(line) {
                Some(item) => found.assumptions.extend(statement(item)),
                // A blank line inside the list is tolerated; any other line ends it.
                None if line.is_empty() => {}
                None => listing = false,
            }
        }
    }
    found.assumptions.truncate(MAX_ASSUMPTIONS);
    found
}

/// What whoever commissions Work is told, once, when its outcome carries no assumptions. It is
/// a note on a successful command: the Work is created and runs either way.
pub const MISSING_ASSUMPTIONS_NOTE: &str =
    "This Work was created, but its outcome does not say what it rests on. \
     From the next one on, end the outcome with a line `Assumptions:`, up to three lines each \
     starting `- `, and one line `Least sure of: …`, so the company can bring them to the owner \
     as questions about direction. Nothing is blocked.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_section_at_the_end_of_an_outcome_is_read_back() {
        let outcome = "Twelve local firms written to, with replies logged.\n\n\
                       Assumptions:\n\
                       - Local firms answer cold email.\n\
                       - Twelve prospects is enough to learn from.\n\
                       Least sure of: whether price or timing decides it.";
        assert_eq!(
            read_assumptions(outcome),
            WorkAssumptions {
                assumptions: vec![
                    "Local firms answer cold email.".into(),
                    "Twelve prospects is enough to learn from.".into(),
                ],
                least_sure: Some("whether price or timing decides it.".into()),
            }
        );
    }

    #[test]
    fn other_list_marks_case_and_spacing_are_tolerated_and_three_is_the_limit() {
        let outcome = "Outcome.\nassumptions:\n\n  1. One\n  2) Two\n  * Three\n  • Four\n\
                       Then ordinary prose.\n- not an assumption\nLEAST SURE OF:   the   price  ";
        let read = read_assumptions(outcome);
        assert_eq!(read.assumptions, ["One", "Two", "Three"]);
        assert_eq!(read.least_sure.as_deref(), Some("the price"));
        // One assumption on the heading's own line.
        assert_eq!(
            read_assumptions("Assumptions: the owner wants local clients only").assumptions,
            ["the owner wants local clients only"]
        );
    }

    #[test]
    fn prose_is_never_guessed_at() {
        for outcome in [
            "",
            "A site that assumes nothing and is least sure of nothing.",
            "We assume the client pays on time.\n- a bullet that belongs to nothing",
            "Assumptions:\nLeast sure of:",
        ] {
            assert!(read_assumptions(outcome).is_empty(), "{outcome:?}");
        }
        // Long statements are cut, never dropped.
        let long = format!("Assumptions:\n- {}", "word ".repeat(80));
        let read = read_assumptions(&long);
        assert_eq!(read.assumptions[0].chars().count(), STATEMENT_CHARS + 1);
        assert!(read.assumptions[0].ends_with('…'));
    }

    #[test]
    fn the_note_names_the_exact_shape_and_never_blocks() {
        assert!(MISSING_ASSUMPTIONS_NOTE.contains(ASSUMPTIONS_HEADING));
        assert!(MISSING_ASSUMPTIONS_NOTE.contains(LEAST_SURE_PREFIX));
        assert!(MISSING_ASSUMPTIONS_NOTE.contains("Nothing is blocked"));
    }
}
