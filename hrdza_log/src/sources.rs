use std::{future, fs, path::Path, time::SystemTime};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc::Sender;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct LogEvent {
    pub timestamp: SystemTime,
    pub line:      String,
    pub source:    String,
}

/// watch selected text files and emit new lines as LogEvent
pub async fn watch_logs(tx: Sender<LogEvent>) -> notify::Result<()> {
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(evt) = res {
                for path in evt.paths {
                    if let Ok(content) = fs::read_to_string(&path) {
                        for line in content.lines().rev().take(10) {
                            let _ = tx.try_send(LogEvent {
                                timestamp: SystemTime::now(),
                                line:      line.to_string(),
                                source:    path.to_string_lossy().into_owned(),
                            });
                        }
                    }
                }
            }
        },
        Config::default(),
    )?;

    // Extend this list with as many files as you need:
    for file in &[
        "/var/log/auth.log",
        "/var/log/syslog",
        "/var/log/kern.log",
        "/var/log/daemon.log",
        "/var/log/dpkg.log",
        "/tmp/custom.log",
        "/var/log/messages",
    ] {
        // only watch if the file exists and is readable
        if Path::new(file).exists() {
            watcher.watch(Path::new(file), RecursiveMode::NonRecursive)?;
        }
    }

    future::pending::<()>().await;
    Ok(())
}
