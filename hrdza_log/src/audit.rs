use audit::new_connection;
use futures::StreamExt;
use tokio::sync::mpsc::Sender;
use crate::sources::LogEvent;
use std::time::SystemTime;

/// stream auditd events via netlink and emit each as LogEvent
pub async fn watch_audit(tx: Sender<LogEvent>) -> anyhow::Result<()> {
    let (_conn, mut handle, mut messages) = new_connection()?;
    handle.enable_events().await?;
    while let Some((msg, _)) = messages.next().await {
        let _ = tx.send(LogEvent {
            timestamp: SystemTime::now(),
            line: format!("{:?}", msg),
            source: "AUDIT".into(),
        }).await;
    }
    Ok(())
}
