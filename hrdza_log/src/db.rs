use rusqlite::{Connection, params};
use crate::sources::LogEvent;

/// initialize SQLite schema with a logs table, timestamp index, FTS5, and triggers
pub fn init_db(path: &str) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS logs (
            id          INTEGER PRIMARY KEY,
            timestamp   TEXT NOT NULL,
            level       TEXT,
            message     TEXT NOT NULL,
            metadata    TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_logs_ts ON logs(timestamp);
        CREATE VIRTUAL TABLE IF NOT EXISTS logs_fts
            USING fts5(message, content='logs', content_rowid='id');
        CREATE TRIGGER IF NOT EXISTS logs_ai AFTER INSERT ON logs BEGIN
          INSERT INTO logs_fts(rowid, message) VALUES (new.id, new.message);
        END;
        CREATE TRIGGER IF NOT EXISTS logs_ad AFTER DELETE ON logs BEGIN
          INSERT INTO logs_fts(logs_fts, rowid, message)
            VALUES('delete', old.id, old.message);
        END;
        CREATE TRIGGER IF NOT EXISTS logs_au AFTER UPDATE ON logs BEGIN
          INSERT INTO logs_fts(logs_fts, rowid, message)
            VALUES('delete', old.id, old.message);
          INSERT INTO logs_fts(rowid, message)
            VALUES (new.id, new.message);
        END;
    "#)?;
    Ok(conn)
}

#[allow(dead_code)]
pub fn insert_log(conn: &Connection, evt: &LogEvent, level: &str) {
    let ts: chrono::DateTime<chrono::Utc> = evt.timestamp.into();
    let _ = conn.execute(
        "INSERT INTO logs(timestamp, level, message, metadata) VALUES (?1,?2,?3,?4)",
        params![ts.to_rfc3339(), level, evt.line, evt.source],
    );
}
