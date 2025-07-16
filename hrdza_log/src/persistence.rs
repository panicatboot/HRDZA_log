// src/persistence.rs

use rusqlite::{Connection, params};
use crate::sources::LogEvent;
use chrono::{DateTime, Local};
use anyhow::Result;

/// Initialise la base `db_path` et crée les tables :
/// - `logs` + FTS5 `logs_fts`
/// - `app_usage`
pub fn init_db(db_path: &str) -> Result<()> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS logs (
            id        INTEGER PRIMARY KEY,
            timestamp TEXT NOT NULL,
            source    TEXT NOT NULL,
            line      TEXT NOT NULL
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS logs_fts
            USING fts5(line, content='logs', content_rowid='id');
        CREATE TABLE IF NOT EXISTS app_usage (
            ts_start     TEXT NOT NULL,
            ts_end       TEXT NOT NULL,
            pid          INTEGER,
            app_name     TEXT,
            window_title TEXT
        );
        "#,  // <-- fermeture de la raw string ici
    )?;
    Ok(())
}

/// Insère un LogEvent dans `logs` et dans l’index FTS5.
/// Ouvre une connexion éphémère à chaque appel (thread-safe).
pub fn insert_log(db_path: &str, ev: &LogEvent) -> Result<()> {
    let conn = Connection::open(db_path)?;
    let ts = DateTime::<Local>::from(ev.timestamp).to_rfc3339();
    conn.execute(
        "INSERT INTO logs (timestamp, source, line) VALUES (?1, ?2, ?3)",
        params![ts, ev.source, ev.line],
    )?;
    conn.execute(
        "INSERT INTO logs_fts (rowid, line) VALUES (last_insert_rowid(), ?1)",
        params![ev.line],
    )?;
    Ok(())
}

/// Agrège la durée totale par application (en heures) depuis la table `app_usage`.
pub fn summarize_app_usage(db_path: &str) -> Result<Vec<(String, f64)>> {
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT ts_start, ts_end, app_name FROM app_usage")?;
    let mut rows = stmt.query([])?;
    use std::collections::HashMap;
    let mut sum = HashMap::<String, f64>::new();
    while let Some(r) = rows.next()? {
        let s: String = r.get(0)?;
        let e: String = r.get(1)?;
        let app: String = r.get(2)?;
        let d1 = DateTime::parse_from_rfc3339(&s)?.with_timezone(&Local);
        let d2 = DateTime::parse_from_rfc3339(&e)?.with_timezone(&Local);
        let secs = (d2 - d1).num_seconds() as f64;
        *sum.entry(app).or_insert(0.0) += secs;
    }
    let mut v: Vec<_> = sum.into_iter().collect();
    v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    Ok(v)
}
