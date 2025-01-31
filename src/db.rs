pub(crate) fn open_db(reset: bool) -> rusqlite::Connection {
    let db = rusqlite::Connection::open("./db.sqlite").expect("Failed to open db");
    db.execute(
        "CREATE TABLE IF NOT EXISTS stories (id INTEGER PRIMARY KEY)",
        (),
    )
    .expect("Failed to create stories table");

    if reset {
        db.execute("DELETE FROM stories", ())
            .expect("Failed to delete stories");
        tracing::info!("Reset DB");
    }

    db
}

pub(crate) fn get_processed_stories(db: &rusqlite::Connection) -> Vec<i64> {
    let mut stmt = db
        .prepare("SELECT id FROM stories")
        .expect("Failed to prepare statement");

    stmt.query_map([], |row| Ok(row.get(0).expect("To get id")))
        .expect("Failed to query stories")
        .map(|row| row.expect("To get id"))
        .collect()
}

pub(crate) fn insert_stories(db: &rusqlite::Connection, stories: Vec<crate::Story>) {
    let mut stmt = db
        .prepare("INSERT INTO stories (id) VALUES (?)")
        .expect("Failed to prepare statement");

    for story in stories {
        stmt.execute((story.id,)).expect("Failed to insert story");
    }
}
