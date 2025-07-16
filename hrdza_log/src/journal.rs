#![allow(deprecated)]

use tokio::task;
use systemd::journal::{Journal, JournalFiles, JournalSeek};
use tokio::sync::mpsc::Sender;
use crate::sources::LogEvent;
use std::time::SystemTime;
use anyhow::Context;

/// watch systemd journal entries and emit each as LogEvent
pub async fn watch_journal(tx: Sender<LogEvent>) -> anyhow::Result<()> {
    let _ = task::spawn_blocking(move || -> anyhow::Result<()> {
        let mut journal = Journal::open(JournalFiles::All, false, false)
            .context("Failed to open systemd journal")?;
        journal.seek(JournalSeek::Tail)
            .context("Failed to seek to journal tail")?;

        loop {
            journal.wait(None).context("Journal wait failed")?;
            while let Some(entry) = journal.next_entry().context("Failed to read next journal entry")? {
                let msg = entry.get("MESSAGE").cloned().unwrap_or_default();
                let _ = tx.blocking_send(LogEvent {
                    timestamp: SystemTime::now(),
                    line: msg,
                    source: "JOURNALD".into(),
                });
            }
        }
    })
    .await?;
    Ok(())
}
