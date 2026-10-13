// sqlx::migrate! embeds the migrations at compile time; a new migration file alone must still
// rebuild this crate, or a binary built after it silently lacks the migration.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
