//! Finding sheets by what is in them: the text of their cells, as last saved, and the comments on
//! them. Only sheets the person may open are searched.
use crate::sheets::SheetError;
use crate::OrgIntel;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

type Result<T> = std::result::Result<T, SheetError>;

/// A sheet whose cells hold every word searched for, with the first cell that matched.
#[derive(Debug, Clone, Serialize)]
pub struct SheetCellHit {
    pub sheet_id: Uuid,
    pub title: String,
    /// The tab and cell of the first match, as `Sheet1` and `B4`.
    pub tab: Option<String>,
    pub cell: Option<String>,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SheetCommentHit {
    pub sheet_id: Uuid,
    pub title: String,
    pub thread_id: Uuid,
    pub comment_id: Uuid,
    pub author_actor_id: String,
    pub snippet: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SheetSearch {
    pub cells: Vec<SheetCellHit>,
    pub comments: Vec<SheetCommentHit>,
}

/// Each word as a case-insensitive pattern, its LIKE wildcards taken literally.
fn patterns(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .take(8)
        .map(|word| word.chars().take(100).collect::<String>())
        .map(|word| {
            let escaped = word
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            format!("%{escaped}%")
        })
        .collect()
}

/// The line of a sheet's text that best shows why it matched: one holding every word, else the
/// first holding any.
fn first_match(text: &str, words: &[String]) -> (Option<String>, Option<String>, String) {
    let lower: Vec<String> = words.iter().map(|word| word.to_lowercase()).collect();
    let lines: Vec<&str> = text.lines().collect();
    let line = lines
        .iter()
        .find(|line| {
            let line = line.to_lowercase();
            lower.iter().all(|word| line.contains(word.as_str()))
        })
        .or_else(|| {
            lines.iter().find(|line| {
                let line = line.to_lowercase();
                lower.iter().any(|word| line.contains(word.as_str()))
            })
        });
    let Some(line) = line else {
        return (None, None, String::new());
    };
    // "Tab!B4 content": the tab may hold spaces or "!", so the place is the first "!" followed by a
    // cell name and a space.
    for (at, _) in line.match_indices('!') {
        let rest = &line[at + 1..];
        let letters = rest.bytes().take_while(u8::is_ascii_uppercase).count();
        let digits = rest[letters..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
        if letters > 0 && digits > 0 && rest[letters + digits..].starts_with(' ') {
            let content = &rest[letters + digits + 1..];
            return (
                Some(line[..at].to_string()),
                Some(rest[..letters + digits].to_string()),
                content.chars().take(160).collect(),
            );
        }
    }
    (None, None, line.chars().take(160).collect())
}

impl OrgIntel {
    /// Sheets whose cells hold every word of the query, and comments on sheets that do, among the
    /// sheets this person can open now.
    pub async fn search_sheets(&self, actor: &str, query: &str) -> Result<SheetSearch> {
        let like = patterns(query);
        if like.is_empty() {
            return Ok(SheetSearch::default());
        }
        let words: Vec<String> = query
            .split_whitespace()
            .take(8)
            .map(str::to_string)
            .collect();
        let readable: Vec<Uuid> = self
            .list_sheets(actor)
            .await?
            .into_iter()
            .map(|row| row.id)
            .collect();
        if readable.is_empty() {
            return Ok(SheetSearch::default());
        }
        let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT id,title,search_text FROM native_sheets \
             WHERE id=ANY($1) AND search_text ILIKE ALL($2) \
             ORDER BY updated_at DESC LIMIT 8",
        )
        .bind(&readable)
        .bind(&like)
        .fetch_all(&self.pool)
        .await?;
        let cells = rows
            .into_iter()
            .map(|(sheet_id, title, text)| {
                let (tab, cell, snippet) = first_match(&text, &words);
                SheetCellHit {
                    sheet_id,
                    title,
                    tab,
                    cell,
                    snippet,
                }
            })
            .collect();
        let comments = sqlx::query_as(
            "SELECT c.sheet_id,s.title,c.thread_id,c.id AS comment_id,c.author_actor_id, \
               left(c.body,200) AS snippet,c.created_at \
             FROM native_sheet_comments c JOIN native_sheets s ON s.id=c.sheet_id \
             WHERE c.sheet_id=ANY($1) AND c.deleted_at IS NULL AND c.body ILIKE ALL($2) \
             ORDER BY c.created_at DESC LIMIT 6",
        )
        .bind(&readable)
        .bind(&like)
        .fetch_all(&self.pool)
        .await?;
        Ok(SheetSearch { cells, comments })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_match_names_its_tab_and_cell() {
        let text = "Budget!A1 Item\nBudget!A4 Ads spend\nQ 2 plan!C9 Ads";
        let words = vec!["ads".to_string(), "spend".to_string()];
        assert_eq!(
            first_match(text, &words),
            (Some("Budget".into()), Some("A4".into()), "Ads spend".into())
        );
        let words = vec!["plan".to_string()];
        assert_eq!(
            first_match(text, &words),
            (Some("Q 2 plan".into()), Some("C9".into()), "Ads".into())
        );
    }

    #[test]
    fn wildcards_are_taken_literally() {
        assert_eq!(patterns("50% off_peak"), vec!["%50\\%%", "%off\\_peak%"]);
    }
}
