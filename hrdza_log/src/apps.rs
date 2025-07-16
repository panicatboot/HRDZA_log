use crate::sources::LogEvent;
use rusqlite::{Connection as SqlConnection, params};
use chrono::{DateTime, Local};
use x11rb::{
    connection::Connection as X11Connection,
    protocol::xproto::{ConnectionExt, AtomEnum, Window},
    rust_connection::RustConnection,
};
use std::{thread, time::Duration, fs};
use tokio::sync::mpsc::Sender;
use std::time::SystemTime;

/// watch active X11 window every 5 seconds and log usage sessions
pub async fn watch_apps(tx: Sender<LogEvent>) -> anyhow::Result<()> {
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let db_path = "hrdza_log.db";
        if let Some(dir) = fs::canonicalize(db_path).ok().and_then(|p| p.parent().map(|p| p.to_path_buf())) {
            let _ = fs::create_dir_all(&dir);
        }
        let conn = SqlConnection::open(db_path)?;
        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS app_usage (
                ts_start     TEXT NOT NULL,
                ts_end       TEXT NOT NULL,
                pid          INTEGER,
                app_name     TEXT,
                window_title TEXT
            );
        "#)?;

        let mut current_win: Option<Window> = None;
        let mut session_start = Local::now();
        let mut current_pid = 0u32;
        let mut current_app = String::new();
        let mut current_title = String::new();

        loop {
            let (conn_x, screen_num) = RustConnection::connect(None)?;
            let screen = &conn_x.setup().roots[screen_num];

            let atom_win = conn_x.intern_atom(false, b"_NET_ACTIVE_WINDOW")?.reply()?.atom;
            let prop = conn_x.get_property(false, screen.root, atom_win, AtomEnum::WINDOW, 0, 1)?
                .reply()?;
            let win_opt = prop.value32().and_then(|mut v| v.next());

            if win_opt != current_win {
                if current_win.is_some() && !current_app.is_empty() {
                    let ts_end = Local::now();
                    conn.execute(
                        "INSERT INTO app_usage (ts_start, ts_end, pid, app_name, window_title)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![
                            session_start.to_rfc3339(),
                            ts_end.to_rfc3339(),
                            current_pid,
                            current_app,
                            current_title
                        ],
                    )?;
                    let _ = tx.blocking_send(LogEvent {
                        timestamp: SystemTime::now(),
                        line: format!(
                            "App '{}' (pid {}) used from {} to {}",
                            current_app,
                            current_pid,
                            session_start.format("%H:%M:%S"),
                            ts_end.format("%H:%M:%S")
                        ),
                        source: "APPS".into(),
                    });
                }
                session_start = Local::now();
                current_win = win_opt;

                if let Some(w) = win_opt {
                    let atom_pid = conn_x.intern_atom(false, b"_NET_WM_PID")?.reply()?.atom;
                    let pid_prop = conn_x.get_property(false, w, atom_pid, AtomEnum::CARDINAL, 0, 1)?
                        .reply()?;
                    current_pid = pid_prop.value32().and_then(|mut v| v.next()).unwrap_or(0);

                    let atom_name = conn_x.intern_atom(false, b"_NET_WM_NAME")?.reply()?.atom;
                    let name_prop = conn_x.get_property(false, w, atom_name, AtomEnum::STRING, 0, 1024)?
                        .reply()?;
                    current_title = String::from_utf8_lossy(&name_prop.value).to_string();

                    current_app = std::fs::read_link(format!("/proc/{}/exe", current_pid))
                        .ok()
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                        .unwrap_or_default();

                    let _ = tx.blocking_send(LogEvent {
                        timestamp: SystemTime::now(),
                        line: format!(
                            "App '{}' (pid {}), title: {}",
                            current_app, current_pid, current_title
                        ),
                        source: "APPS".into(),
                    });
                }
            }

            thread::sleep(Duration::from_secs(5));
        }
    })
    .await??;
    Ok(())
}

/// summarize total usage time per application
pub fn summarize_app_usage() -> anyhow::Result<Vec<(String, f64)>> {
    let conn = SqlConnection::open("hrdza_log.db")?;
    let mut stmt = conn.prepare("SELECT ts_start, ts_end, app_name FROM app_usage")?;
    let mut rows = stmt.query([])?;
    use std::collections::HashMap;
    let mut sum: HashMap<String, f64> = HashMap::new();
    while let Some(r) = rows.next()? {
        let ts1: String = r.get(0)?;
        let ts2: String = r.get(1)?;
        let app: String = r.get(2)?;
        let d1 = DateTime::parse_from_rfc3339(&ts1)?.with_timezone(&Local);
        let d2 = DateTime::parse_from_rfc3339(&ts2)?.with_timezone(&Local);
        let secs = (d2 - d1).num_seconds() as f64;
        *sum.entry(app).or_insert(0.0) += secs;
    }
    let mut v: Vec<_> = sum.into_iter().collect();
    v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    Ok(v)
}
