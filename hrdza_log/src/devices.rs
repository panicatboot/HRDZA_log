use crate::sources::LogEvent;
use std::time::SystemTime;
use tokio::sync::mpsc::Sender;
use tokio::process::Command;
use udev::MonitorBuilder;

/// monitor udev for USB and video4linux events
pub async fn watch_udev(tx: Sender<LogEvent>) -> anyhow::Result<()> {
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let monitor = MonitorBuilder::new()?
            .match_subsystem("usb")?
            .match_subsystem("video4linux")?
            .listen()?;
        for event in monitor {
            let devnode = event.devnode()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let msg = format!("{:?} {}", event.event_type(), devnode);
            let _ = tx.blocking_send(LogEvent {
                timestamp: SystemTime::now(),
                line: msg,
                source: "UDEV".into(),
            });
        }
        Ok(())
    }).await??;
    Ok(())
}

/// poll CUPS printer status every 10 seconds
pub async fn watch_printers(tx: Sender<LogEvent>) -> anyhow::Result<()> {
    tokio::spawn(async move {
        loop {
            let out = match Command::new("lpstat").arg("-p").output().await {
                Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
                Err(e) => format!("lpstat error: {}", e),
            };
            for line in out.lines() {
                let _ = tx.send(LogEvent {
                    timestamp: SystemTime::now(),
                    line: line.to_string(),
                    source: "CUPS".into(),
                }).await;
            }
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });
    Ok(())
}

/// poll PulseAudio sink inputs every 5 seconds
pub async fn watch_pulseaudio(tx: Sender<LogEvent>) -> anyhow::Result<()> {
    tokio::spawn(async move {
        loop {
            let out = match Command::new("pactl").args(&["list","sink-inputs"]).output().await {
                Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
                Err(e) => format!("pactl error: {}", e),
            };
            for line in out.lines() {
                let _ = tx.send(LogEvent {
                    timestamp: SystemTime::now(),
                    line: line.to_string(),
                    source: "PULSE".into(),
                }).await;
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
    Ok(())
}
