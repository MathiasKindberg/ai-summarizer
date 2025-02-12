pub(crate) fn open_db(reset: bool) -> anyhow::Result<rusqlite::Connection> {
    let db = rusqlite::Connection::open("./db.sqlite")?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS stories (id INTEGER PRIMARY KEY)",
        (),
    )?;

    if reset {
        db.execute("DELETE FROM stories", ())?;
        tracing::info!("Reset DB");
    }

    Ok(db)
}

pub(crate) fn get_processed_stories(db: &rusqlite::Connection) -> anyhow::Result<Vec<i64>> {
    let mut stmt = db.prepare("SELECT id FROM stories")?;

    let mut ids = Vec::new();
    for row in stmt.query_map([], |row| row.get(0))? {
        ids.push(row?);
    }
    Ok(ids)
}

pub(crate) fn insert_stories(
    db: &rusqlite::Connection,
    stories: Vec<crate::Story>,
) -> anyhow::Result<()> {
    let mut stmt = db.prepare("INSERT INTO stories (id) VALUES (?) ON CONFLICT(id) DO NOTHING")?;

    for story in stories {
        stmt.execute((story.id,))?;
    }

    Ok(())
}
