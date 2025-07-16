use crate::sources::LogEvent;
use chrono::{DateTime, Local, Timelike};

/// compute a 7×24 heatmap of event counts over the last 7 days
pub fn compute_heatmap(events: &[LogEvent]) -> [[u32; 24]; 7] {
    let now: DateTime<Local> = Local::now();
    let today = now.date_naive();
    let mut matrix = [[0u32; 24]; 7];
    for ev in events {
        let dt: DateTime<Local> = DateTime::from(ev.timestamp);
        let days_ago = (today - dt.date_naive()).num_days();
        if (0..7).contains(&days_ago) {
            matrix[(6 - days_ago) as usize][dt.hour() as usize] += 1;
        }
    }
    matrix
}
