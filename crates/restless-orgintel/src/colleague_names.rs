//! Existing fictional characters, ordered within each team by their readable name prefix.
//! Stable actor IDs own Work and messages; display changes are audited once.
use super::*;

// One current catalogue for both company-wide lead letters and team members. Distinct
// first-word prefixes make the same short mark readable in the roster and conversation.
const NAMES: [&[&str]; 26] = [
    &["Aang", "Abu", "Ace", "Ada Wong", "Aerith"],
    &["Baloo", "Beast", "Bilbo", "Blossom", "Boba Fett"],
    &[
        "Calcifer",
        "Cecil Harvey",
        "Chewbacca",
        "Cinderella",
        "Cloud Strife",
    ],
    &["Daria", "Della Duck", "Dipper Pines", "Dobby", "Drax"],
    &["Eames", "Ebenezer Scrooge", "Echo", "Edna Mode", "Eeyore"],
    &["Falkor", "Ferdinand", "Finn", "Flik", "Frodo"],
    &["Gandalf", "Geralt", "Gimli", "Goku", "Grinch"],
    &[
        "Harry Potter",
        "He-Man",
        "Hiccup",
        "Hobbes",
        "Huckleberry Finn",
    ],
    &["Iago", "Ichabod Crane", "Igor", "Illyana Rasputin", "Iroh"],
    &["Jack Sparrow", "Jekyll", "Jinx", "John Wick", "Judy Hopps"],
    &["Katniss", "Kermit", "Kiki", "Korra", "Kuzco"],
    &["Lara Croft", "Leia", "Lilo", "Loki", "Luigi"],
    &["Mario", "Megamind", "Mickey Mouse", "Moana", "Mulan"],
    &["Nala", "Nemo", "Nick Wilde", "Nobita", "Nurse Joy"],
    &["Obi-Wan", "Odie", "Olaf", "Optimus Prime", "Oswald"],
    &[
        "Paddington",
        "Peter Pan",
        "Pikachu",
        "Popeye",
        "Princess Bubblegum",
    ],
    &["Q", "Q*bert", "Qiqi", "Qrow Branwen", "Quasimodo"],
    &["R2-D2", "Rapunzel", "Remy", "Robin Hood", "Ryu"],
    &["Samwise Gamgee", "Scooby-Doo", "Shrek", "Simba", "Sonic"],
    &["Tarzan", "Teela", "Tiana", "Totoro", "Trinity"],
    &["Uhtred", "Ultron", "Underdog", "Ursula", "Usagi Tsukino"],
    &["Vash", "Velma", "Violet Parr", "Voldemort", "Vulture"],
    &[
        "Wall-E",
        "Wendy Darling",
        "Winnie the Pooh",
        "Woody",
        "Wreck-It Ralph",
    ],
    &["X", "Xander Harris", "Xena", "Xion", "Xûr"],
    &["Yamcha", "Yennefer", "Ymir", "Yoda", "Yugi"],
    &["Zazu", "Zelda", "Zoidberg", "Zuko", "Zyra"],
];

fn initial(name: &str) -> Option<char> {
    name.chars()
        .next()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_uppercase())
}

fn numbered(base: &str, round: u64) -> String {
    if round == 0 {
        base.to_owned()
    } else {
        format!("{base} {}", round + 1)
    }
}

fn is_current_name(name: &str) -> bool {
    let Some(letter) = initial(name) else {
        return false;
    };
    let base = match name.rsplit_once(' ') {
        Some((base, number)) if number.parse::<u64>().is_ok_and(|round| round >= 2) => base,
        _ => name,
    };
    NAMES[(letter as u8 - b'A') as usize].contains(&base)
}

pub(crate) async fn lock_names(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<()> {
    sqlx::query("SELECT allocated FROM colleague_name_allocator WHERE singleton FOR UPDATE")
        .fetch_one(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn allocate_name(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    lead_name: Option<&str>,
) -> Result<String> {
    lock_names(tx).await?;
    let pool = lead_name
        .and_then(initial)
        .map(|c| NAMES[(c as u8 - b'A') as usize]);
    let mut ordinal = 0u64;
    loop {
        let name = if let Some(pool) = pool {
            let name = numbered(
                pool[(ordinal % pool.len() as u64) as usize],
                ordinal / pool.len() as u64,
            );
            ordinal += 1;
            name
        } else {
            // The SQL function remains in migration history for older released binaries;
            // the current runtime allocates every name from this one catalogue.
            let allocated: i64 = sqlx::query_scalar("UPDATE colleague_name_allocator SET allocated=allocated+1 WHERE singleton RETURNING allocated")
                .fetch_one(&mut **tx).await?;
            let ordinal = (allocated - 1) as u64;
            numbered(NAMES[(ordinal % 26) as usize][0], ordinal / 26)
        };
        let used: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE display=$1 UNION ALL \
             SELECT 1 FROM events WHERE kind='actor_display_changed' \
             AND (body->>'from_display'=$1 OR body->>'to_display'=$1))",
        )
        .bind(&name)
        .fetch_one(&mut **tx)
        .await?;
        if !used {
            return Ok(name);
        }
    }
}

async fn rename_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: &str,
    before: &str,
    after: &str,
) -> Result<()> {
    sqlx::query("UPDATE actors SET display=$2 WHERE id=$1")
        .bind(id)
        .bind(after)
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO events (kind,actor_id,body) VALUES ('actor_display_changed',$1,$2)")
        .bind(id)
        .bind(
            serde_json::json!({"actor_id":id,"from_display":before,"to_display":after,
            "reason":"Alphabetical fictional characters within each team"}),
        )
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn align_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    team: Option<Uuid>,
) -> Result<()> {
    lock_names(tx).await?;
    // Convert old ordinary names before reading the team's lead. Already allocated
    // characters retain their name, including a newly promoted lead or an unassigned worker.
    let roots = sqlx::query(
        "SELECT a.id,a.display FROM actors a WHERE a.kind='staff' AND a.retired_at IS NULL \
         AND (($1::uuid IS NULL AND a.team_id IS NULL) OR EXISTS(SELECT 1 FROM teams t \
         WHERE t.lead_actor_id=a.id AND t.disbanded_at IS NULL AND ($1::uuid IS NULL OR t.id=$1))) \
         ORDER BY a.created_at,a.id FOR UPDATE OF a",
    )
    .bind(team)
    .fetch_all(&mut **tx)
    .await?;
    for row in roots {
        let id: String = row.get("id");
        let before: String = row.get("display");
        if is_current_name(&before) {
            continue;
        }
        let after = allocate_name(tx, Some(&before)).await?;
        rename_in_tx(tx, &id, &before, &after).await?;
    }
    let members = sqlx::query(
        "SELECT a.id,a.display,l.display AS lead_display FROM actors a \
         JOIN teams t ON t.id=a.team_id JOIN actors l ON l.id=t.lead_actor_id \
         WHERE a.kind='staff' AND a.retired_at IS NULL AND t.disbanded_at IS NULL AND a.id<>t.lead_actor_id \
         AND ($1::uuid IS NULL OR t.id=$1) ORDER BY a.created_at,a.id FOR UPDATE OF a",
    ).bind(team).fetch_all(&mut **tx).await?;
    for row in members {
        let id: String = row.get("id");
        let before: String = row.get("display");
        let lead: String = row.get("lead_display");
        if initial(&lead).is_none()
            || (initial(&before) == initial(&lead) && is_current_name(&before))
        {
            continue;
        }
        let after = allocate_name(tx, Some(&lead)).await?;
        rename_in_tx(tx, &id, &before, &after).await?;
    }
    Ok(())
}

impl OrgIntel {
    /// Idempotent reconciliation when an existing company is opened.
    pub async fn align_colleague_names(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        align_in_tx(&mut tx, None).await?;
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_has_ordered_distinct_marks_in_every_letter_family() {
        for (index, pool) in NAMES.iter().enumerate() {
            let mut marks = std::collections::BTreeSet::new();
            let mut previous = String::new();
            for name in *pool {
                assert_eq!(initial(name), Some((b'A' + index as u8) as char));
                let mark = name
                    .split_whitespace()
                    .next()
                    .unwrap()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .take(2)
                    .collect::<String>();
                assert!(marks.insert(mark.clone()), "duplicate {mark} in {pool:?}");
                assert!(mark >= previous, "out-of-order {name} in {pool:?}");
                previous = mark;
                assert!(is_current_name(name));
                assert!(is_current_name(&numbered(name, 1)));
            }
        }
        assert!(!is_current_name("Elif"));
        assert!(!is_current_name("Eames 1"));
        assert!(!is_current_name("Eames 0"));
        assert!(!is_current_name("Eames 18446744073709551616"));
    }
}
