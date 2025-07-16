mod ui;
mod sources;
mod audit;
mod journal;
mod filters;
mod graphs;
mod db;
mod export;
mod devices;
mod apps;

use anyhow::Result;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    // initialize SQLite with schema, indexes, and full-text search
    let _ = db::init_db("hrdza_log.db")?;

    // create a channel for log events
    let (tx, rx) = mpsc::channel(500);

    // spawn watcher for text log files
    {
        let tx = tx.clone();
        tokio::spawn(async move {
            let _ = sources::watch_logs(tx).await;
        });
    }

    // spawn watcher for auditd events
    {
        let tx = tx.clone();
        tokio::spawn(async move {
            let _ = audit::watch_audit(tx).await;
        });
    }

    // spawn watcher for systemd journal entries
    {
        let tx = tx.clone();
        tokio::spawn(async move {
            let _ = journal::watch_journal(tx).await;
        });
    }

    // spawn watchers for devices (udev, CUPS, PulseAudio)
    {
        let tx = tx.clone();
        tokio::spawn(async move {
            let _ = devices::watch_udev(tx.clone()).await;
            let _ = devices::watch_printers(tx.clone()).await;
            let _ = devices::watch_pulseaudio(tx).await;
        });
    }

    // spawn watcher for active window usage via X11
    {
        let tx = tx.clone();
        tokio::spawn(async move {
            let _ = apps::watch_apps(tx).await;
        });
    }

    // start the terminal UI (blocks until exit)
    ui::start_ui(rx).await?;

    Ok(())
}
