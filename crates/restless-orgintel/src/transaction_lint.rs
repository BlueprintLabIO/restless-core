//! A transaction that returns early must end itself first (PR #653 follow-up, 4 November 2026).
//!
//! Dropping an open sqlx transaction only queues its ROLLBACK; it is sent later, from a task the
//! pool spawns when the connection comes back. Until then the rows it locked stay locked, and the
//! workers that claim with `SKIP LOCKED` (the scheduler, Undo windows, schedules, deliveries) pass
//! them over. That is how a refused `resume_after_interruption` or approval `cancel` made the
//! scheduler skip ready Work.
//!
//! The rule this test enforces in OrgIntel and the engine: inside a transaction a function began
//! itself (`let mut tx = ….begin().await…;`), every explicit early exit (`return`, `bail!`,
//! `ensure!`) first ends it with `tx.rollback().await?` (or commits). Exits through `?` are not
//! checked: those are mostly database errors on a connection that is already failing.

use std::path::{Path, PathBuf};

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether `line` ends the transaction named `var` (`var.commit()` or `var.rollback()`).
fn ends(line: &str, var: &str) -> bool {
    ["commit()", "rollback()"].iter().any(|call| {
        let needle = format!("{var}.{call}");
        line.match_indices(&needle)
            .any(|(at, _)| !line[..at].chars().next_back().is_some_and(is_ident))
    })
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Where an early exit starts in a trimmed line, if the line has one.
fn early_exit(trimmed: &str) -> Option<usize> {
    let line = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    let starts = [
        "return",
        "bail!",
        "anyhow::bail!",
        "ensure!",
        "anyhow::ensure!",
    ];
    for start in starts {
        if let Some(rest) = line.strip_prefix(start) {
            if start.ends_with('!') || !rest.starts_with(is_ident) {
                return Some(0);
            }
        }
    }
    let inline = [
        "=> return",
        "{ return",
        "{return",
        "=> bail!",
        "=> anyhow::bail!",
        "{ bail!",
        "{bail!",
    ];
    inline
        .iter()
        .filter_map(|marker| line.find(marker))
        .min()
        .and_then(|at| {
            let word = if line[at..].contains("return") {
                "return"
            } else {
                "bail!"
            };
            trimmed.find(word).or(Some(at))
        })
}

/// Whether the transaction was already ended on the way to the exit at `exit` (line index).
fn ended_before(lines: &[&str], begin: usize, exit: usize, at: usize, var: &str) -> bool {
    let head = &lines[exit].trim_start()[..at];
    if ends(head, var) {
        return true;
    }
    let mut depth = indent(lines[exit]);
    for line in lines[begin + 1..exit].iter().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let here = indent(line);
        if here < depth && trimmed.ends_with('{') {
            // The block around the exit opens here; earlier statements of the outer block run
            // before it, a sibling branch's do not.
            if ends(line, var) {
                return true;
            }
            depth = here;
            continue;
        }
        if here <= depth && ends(line, var) {
            return true;
        }
    }
    false
}

/// Early exits that leave a transaction open, as `file:line: code`.
fn violations(path: &Path) -> Vec<String> {
    let source = std::fs::read_to_string(path).expect("read source");
    let mut lines: Vec<&str> = source.lines().collect();
    if let Some(tests) = lines.windows(2).position(|pair| {
        pair[0].trim() == "#[cfg(test)]" && {
            let next = pair[1].trim();
            next.starts_with("mod tests") || next.starts_with("pub mod tests")
        }
    }) {
        lines.truncate(tests);
    }
    let mut found = Vec::new();
    for (begin, line) in lines.iter().enumerate() {
        let Some(rest) = line.trim_start().strip_prefix("let ") else {
            continue;
        };
        let rest = rest.strip_prefix("mut ").unwrap_or(rest);
        let var: String = rest.chars().take_while(|c| is_ident(*c)).collect();
        if var.is_empty() {
            continue;
        }
        let statement: String = lines[begin..lines.len().min(begin + 6)]
            .concat()
            .split(';')
            .next()
            .unwrap_or_default()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        if !statement.contains(".begin().await") {
            continue;
        }
        let scope = indent(line);
        for exit in begin + 1..lines.len() {
            let current = lines[exit];
            let trimmed = current.trim();
            if !trimmed.is_empty() && indent(current) < scope && trimmed.starts_with('}') {
                break;
            }
            if indent(current) == scope && ends(current, &var) {
                break;
            }
            if trimmed.starts_with("//") {
                continue;
            }
            if let Some(at) = early_exit(trimmed) {
                if !ended_before(&lines, begin, exit, at, &var) {
                    found.push(format!("{}:{}: {trimmed}", path.display(), exit + 1));
                }
            }
        }
    }
    found
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read source directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path
            .file_name()
            .is_some_and(|name| name == "transaction_lint.rs")
        {
            // Its samples are open on purpose.
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn every_early_exit_ends_its_transaction_first() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for name in ["restless-orgintel", "restless-engine"] {
        rust_files(&crates.join(name).join("src"), &mut files);
    }
    files.sort();
    assert!(
        files.len() > 50,
        "the lint found too few sources to be scanning the crates"
    );
    let found: Vec<String> = files.iter().flat_map(|file| violations(file)).collect();
    assert!(
        found.is_empty(),
        "these early exits leave a transaction open. A dropped transaction only queues its \
         ROLLBACK, so its row locks outlive the call and SKIP LOCKED workers pass the rows over. \
         End it first with `tx.rollback().await?;`:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_lint_sees_an_open_transaction_and_accepts_an_ended_one() {
    let dir = std::env::temp_dir().join(format!("restless-tx-lint-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("sample.rs");
    std::fs::write(
        &file,
        "async fn refuse(pool: &PgPool) -> Result<bool> {
    let mut tx = pool
        .begin()
        .await?;
    lock(&mut tx).await?;
    if busy {
        return Ok(false);
    }
    if gone {
        tx.rollback().await?;
        return Ok(false);
    }
    match state {
        None => bail!(\"no row\"),
        Some(_) => {}
    }
    if done {
        tx.commit().await?;
        if again {
            return Ok(true);
        }
    }
    tx.commit().await?;
    Ok(true)
}
",
    )
    .unwrap();
    let found = violations(&file);
    std::fs::remove_dir_all(&dir).ok();
    let lines: Vec<&str> = found
        .iter()
        .map(|line| line.rsplit_once(".rs:").unwrap().1)
        .collect();
    assert_eq!(
        lines,
        ["7: return Ok(false);", "14: None => bail!(\"no row\"),"]
    );
}
