//! Fictional display names; stable actor IDs continue to own work and messages.
use super::*;

const NAMES: [&[&str]; 26] = [
    &["Alice", "Aragorn", "Aang", "Ariel", "Anakin"],
    &["Bart", "Batman", "Bilbo", "Buffy", "Bugs Bunny"],
    &[
        "Coraline",
        "Charlie Brown",
        "Chewbacca",
        "Cinderella",
        "Chihiro",
    ],
    &["Daria", "Dobby", "Daphne", "Dexter", "Daenerys"],
    &["Elsa", "Elmo", "Eowyn", "Eeyore", "Edna Mode"],
    &["Frodo", "Fiona", "Fry", "Fred Flintstone", "Fozzie"],
    &["Gandalf", "Garfield", "Groot", "Goku", "Gon"],
    &["Hannah Montana", "Hermione", "Homer", "Hiccup", "Heidi"],
    &[
        "Ichabod Crane",
        "Indiana Jones",
        "Inigo Montoya",
        "Iroh",
        "Iron Man",
    ],
    &[
        "Jessie",
        "Jasmine",
        "Jinx",
        "Jiminy Cricket",
        "Jack Sparrow",
    ],
    &["Katniss", "Kiki", "Kermit", "Korra", "Kirby"],
    &["Lilo", "Leia", "Luigi", "Luna Lovegood", "Legolas"],
    &["Matilda", "Mulan", "Mario", "Moana", "Mowgli"],
    &["Nemo", "Naruto", "Nala", "Ned Flanders", "Neville"],
    &["Obi-Wan", "Olaf", "Obelix", "Odie", "Oswald"],
    &["Paddington", "Pikachu", "Popeye", "Pippin", "Peter Pan"],
    &[
        "Quasimodo",
        "Quark",
        "Qui-Gon Jinn",
        "Quint",
        "Quentin Coldwater",
    ],
    &["Rapunzel", "Ron Weasley", "R2-D2", "Robin Hood", "Remy"],
    &["Scooby-Doo", "Sonic", "Shrek", "Snoopy", "Samwise"],
    &["Totoro", "Tintin", "Tigger", "Tiana", "Toothless"],
    &["Ursula", "Uhura", "Usagi", "Uhtred", "Ultron"],
    &["Velma", "Violet Parr", "Vash", "Vegeta", "Vivi"],
    &[
        "Winnie the Pooh",
        "Wall-E",
        "Wendy Darling",
        "Woody",
        "Willy Wonka",
    ],
    &["Xena", "Xander Harris", "Xion", "Xavier", "Xerxes"],
    &["Yoda", "Yoshi", "Yugi", "Yennefer", "Yor"],
    &["Zelda", "Zuko", "Zorro", "Zazu", "Zoidberg"],
];

fn initial(name: &str) -> Option<char> {
    name.chars()
        .next()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_uppercase())
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
    let mut ordinal = 0;
    loop {
        let name: String = if let Some(pool) = pool {
            let base = pool[ordinal % pool.len()];
            let round = ordinal / pool.len();
            ordinal += 1;
            if round == 0 {
                base.to_string()
            } else {
                format!("{base} {}", round + 1)
            }
        } else {
            sqlx::query_scalar("UPDATE colleague_name_allocator SET allocated=allocated+1 WHERE singleton RETURNING colleague_name(allocated)").fetch_one(&mut **tx).await?
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

pub(crate) async fn align_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    team: Option<Uuid>,
) -> Result<()> {
    lock_names(tx).await?;
    let members = sqlx::query(
        "SELECT a.id,a.display,l.display AS lead_display FROM actors a \
         JOIN teams t ON t.id=a.team_id JOIN actors l ON l.id=t.lead_actor_id \
         WHERE a.kind='staff' AND a.retired_at IS NULL AND t.disbanded_at IS NULL AND a.id<>t.lead_actor_id \
         AND ($1::uuid IS NULL OR t.id=$1) ORDER BY a.created_at,a.id FOR UPDATE OF a",
    )
    .bind(team)
    .fetch_all(&mut **tx)
    .await?;
    for row in members {
        let id: String = row.get("id");
        let before: String = row.get("display");
        let lead: String = row.get("lead_display");
        if initial(&lead).is_none() || initial(&before) == initial(&lead) {
            continue;
        }
        let after = allocate_name(tx, Some(&lead)).await?;
        sqlx::query("UPDATE actors SET display=$2 WHERE id=$1")
            .bind(&id)
            .bind(&after)
            .execute(&mut **tx)
            .await?;
        sqlx::query("INSERT INTO events (kind,actor_id,body) VALUES ('actor_display_changed',$1,$2)")
            .bind(&id).bind(serde_json::json!({"actor_id":id,"from_display":before,"to_display":after,"reason":"Team members share their lead's initial"})).execute(&mut **tx).await?;
    }
    Ok(())
}

impl OrgIntel {
    /// Idempotent reconciliation for existing rosters when the company is opened.
    pub async fn align_team_member_names(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        align_in_tx(&mut tx, None).await?;
        tx.commit().await?;
        Ok(())
    }
}
