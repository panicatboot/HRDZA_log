use crate::sources::LogEvent;
use chrono::{DateTime, Local, NaiveDate};

// Configuration for filtering log events.
// Each field is optional; only set filters will be applied.
#[derive(Debug, Default, Clone)]
pub struct FilterConfig {
    pub keyword: Option<String>,
    pub date:    Option<NaiveDate>,
    pub user:    Option<String>,
    pub process: Option<String>,
    pub level:   Option<String>,
    pub ip:      Option<String>,
}

// Apply the configured filters to a slice of LogEvent and eturns only those events that match all enabled criteria.
pub fn apply_filters(events: &[LogEvent], cfg: &FilterConfig) -> Vec<LogEvent> {
    events.iter()
        .cloned()
        .filter(|e| {
            // Convert stored timestamp to local date for date filtering
            let dt: DateTime<Local> = DateTime::from(e.timestamp);
            // Check each filter in turn; bail out early on mismatch
            if let Some(ref kw) = cfg.keyword   { if !e.line.contains(kw)  { return false; } }
            if let Some(d)      = cfg.date      { if dt.date_naive() != d  { return false; } }
            if let Some(ref u)  = cfg.user      { if !e.line.contains(u)   { return false; } }
            if let Some(ref p)  = cfg.process   { if !e.line.contains(p)   { return false; } }
            if let Some(ref lvl)= cfg.level     { if !e.line.contains(lvl) { return false; } }
            if let Some(ref ip) = cfg.ip        { if !e.line.contains(ip)  { return false; } }
            true
        })
        .collect()
}

// prse a date string in YYYY-MM-DD format into a NaiveDate and returns None if parsing fails.
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}
