use std::{io::stdout, time::Duration};
use anyhow::Result;
use crossterm::{
    event::{poll, read, Event as CEvent, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Row, Table},
    Terminal,
};
use tokio::sync::mpsc::Receiver;
use tokio::task::spawn_blocking;

use crate::{
    apps::summarize_app_usage,
    export,
    filters::{apply_filters, parse_date, FilterConfig},
    graphs::compute_heatmap,
    sources::LogEvent,
};

#[derive(Clone, Copy)]
enum InputField {
    Keyword,
    Date,
    User,
    Process,
    Level,
    Ip,
}
#[derive(Clone, Copy)]
enum Mode {
    Normal,
    Editing(InputField),
    Heatmap,
    Apps,
}

pub async fn start_ui(mut rx: Receiver<LogEvent>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut term = Terminal::new(CrosstermBackend::new(stdout))?;

    let mut logs: Vec<LogEvent> = Vec::new();
    let mut filters = FilterConfig::default();
    let mut mode = Mode::Normal;
    let mut input = String::new();
    let mut last_export: Option<String> = None;
    // menu height = 3 text lines + 2 borders
    let menu_h = 3 + 2;

    loop {
        // Fetch all incoming events
        while let Ok(ev) = rx.try_recv() {
            logs.push(ev);
            if logs.len() > 500 {
                logs.drain(0..logs.len() - 500);
            }
        }

        // Draw the UI
        term.draw(|f| {
            let size = f.size();

            // Clear entire screen before drawing
            f.render_widget(Clear, size);

            let mut y = 0;

            // Status area (PDF export in progress)
            if let Some(msg) = &last_export {
                let area = Rect {
                    x: 0,
                    y,
                    width: size.width,
                    height: 3,
                };
                let status = Paragraph::new(msg.as_str())
                    .block(Block::default().borders(Borders::ALL).title("Status"));
                f.render_widget(status, area);
                y += 3;
            }

            // Apps mode
            if let Mode::Apps = mode {
                let area = Rect {
                    x: 0,
                    y,
                    width: size.width,
                    height: size.height.saturating_sub(y),
                };
                let summary = summarize_app_usage().unwrap_or_default();
                let rows: Vec<Row> = summary
                    .iter()
                    .map(|(app, secs)| {
                        Row::new(vec![app.clone(), format!("{:.2} h", secs / 3600.0)])
                    })
                    .collect();
                let table = Table::new(rows)
                    .header(Row::new(vec!["Application", "Heures"]))
                    .block(Block::default().borders(Borders::ALL).title("App Usage (a=back)"))
                    .widths(&[Constraint::Percentage(70), Constraint::Percentage(30)]);
                f.render_widget(table, area);
            }
            // Heatmap mode
            else if let Mode::Heatmap = mode {
                let area = Rect {
                    x: 0,
                    y,
                    width: size.width,
                    height: size.height.saturating_sub(y),
                };
                let matrix = compute_heatmap(&logs);
                let blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇'];
                let max = matrix.iter().flatten().copied().max().unwrap_or(1);
                let lines: Vec<Spans> = matrix
                    .iter()
                    .map(|row| {
                        let s: String = row
                            .iter()
                            .map(|&c| {
                                let lvl = if max > 0 {
                                    (c * (blocks.len() as u32 - 1) / max) as usize
                                } else {
                                    0
                                };
                                blocks[lvl]
                            })
                            .collect();
                        Spans::from(s)
                    })
                    .collect();
                let heat = Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Heatmap (h=back)"));
                f.render_widget(heat, area);
            }
            // Normal / Editing mode
            else {
                let rem = Rect {
                    x: 0,
                    y,
                    width: size.width,
                    height: size.height.saturating_sub(y),
                };
                let chunks = if matches!(mode, Mode::Editing(_)) {
                    Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(menu_h),
                            Constraint::Min(1),
                            Constraint::Length(3),
                        ])
                        .split(rem)
                } else {
                    Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(menu_h), Constraint::Min(1)])
                        .split(rem)
                };

                // Shortcut menu
                let w = size.width.saturating_sub(2) as usize;
                let cw = w / 4;
                let m1 = format!(
                    "{:<cw$}{:<cw$}{:<cw$}{:<cw$}",
                    "f: key words",
                    "d: date",
                    "u: user",
                    "p: proc",
                    cw = cw
                );
                let m2 = format!(
                    "{:<cw$}{:<cw$}{:<cw$}{:<cw$}",
                    "l: lvl",
                    "i: IP",
                    "g/h: heatmap",
                    "a: apps",
                    cw = cw
                );
                let m3 = format!(
                    "{:<cw$}{:<cw$}{:<cw$}{:<cw$}",
                    "e: export",
                    "c: clear",
                    "q: quit",
                    "",
                    cw = cw
                );
                let pink = Color::Rgb(255, 173, 214);
                let menu = Paragraph::new(vec![Spans::from(m1), Spans::from(m2), Spans::from(m3)])
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(Span::styled(
                                "Filter & Graph Menu",
                                Style::default()
                                    .fg(pink)
                                    .add_modifier(Modifier::BOLD),
                            ))
                            .border_style(Style::default().fg(pink)),
                    );
                f.render_widget(menu, chunks[0]);

                // Render filtered logs
                let disp = apply_filters(&logs, &filters);
                let hdr = {
                    let mut v = Vec::new();
                    if let Some(ref kw) = filters.keyword {
                        v.push(format!("kw={}", kw));
                    }
                    if let Some(d) = filters.date {
                        v.push(format!("date={}", d));
                    }
                    if let Some(ref u) = filters.user {
                        v.push(format!("user={}", u));
                    }
                    if let Some(ref p) = filters.process {
                        v.push(format!("proc={}", p));
                    }
                    if let Some(ref lvl) = filters.level {
                        v.push(format!("lvl={}", lvl));
                    }
                    if let Some(ref ip) = filters.ip {
                        v.push(format!("ip={}", ip));
                    }
                    if v.is_empty() {
                        "Logs".into()
                    } else {
                        format!("Logs [{}]", v.join(", "))
                    }
                };
                let list = List::new(
                    disp.iter()
                        .rev()
                        .map(|e| ListItem::new(format!("[{}] {}", e.source, e.line)))
                        .collect::<Vec<_>>(),
                )
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(Span::styled(
                            hdr,
                            Style::default()
                                .fg(pink)
                                .add_modifier(Modifier::BOLD),
                        )),
                );
                f.render_widget(list, chunks[1]);

                // Editing input area if needed
                if let Mode::Editing(field) = mode {
                    let area = chunks[2];
                    let prompt = match field {
                        InputField::Keyword => "Enter keyword:",
                        InputField::Date => "Enter date:",
                        InputField::User => "Enter user:",
                        InputField::Process => "Enter process:",
                        InputField::Level => "Enter level:",
                        InputField::Ip => "Enter IP:",
                    };
                    let edit = Paragraph::new(Spans::from(format!("{} {}", prompt, input)))
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("Editing (Enter=OK, Esc=Cancel)")
                                .style(Style::default().add_modifier(Modifier::BOLD)),
                        );
                    f.render_widget(edit, area);
                }
            }
        })?;

        // Handle keyboard input
        if poll(Duration::from_millis(100))? {
            if let CEvent::Key(KeyEvent { code, modifiers, .. }) = read()? {
                match mode {
                    Mode::Normal => match (code, modifiers) {
                        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            break
                        }
                        (KeyCode::Char('c'), _) => filters = FilterConfig::default(),
                        (KeyCode::Char('h'), _) | (KeyCode::Char('g'), _) => {
                            mode = Mode::Heatmap
                        }
                        (KeyCode::Char('a'), _) => mode = Mode::Apps,
                        (KeyCode::Char('e'), _) => {
                            let evs = logs.clone();
                            let flt = filters.clone();
                            last_export = Some("Exporting PDF…".into());
                            spawn_blocking(move || {
                                if let Err(e) = export::generate_pdf(&evs, &flt) {
                                    eprintln!("Export error: {}", e);
                                }
                            });
                        }
                        (KeyCode::Char('f'), _) => {
                            mode = Mode::Editing(InputField::Keyword);
                            input.clear();
                        }
                        (KeyCode::Char('d'), _) => {
                            mode = Mode::Editing(InputField::Date);
                            input.clear();
                        }
                        (KeyCode::Char('u'), _) => {
                            mode = Mode::Editing(InputField::User);
                            input.clear();
                        }
                        (KeyCode::Char('p'), _) => {
                            mode = Mode::Editing(InputField::Process);
                            input.clear();
                        }
                        (KeyCode::Char('l'), _) => {
                            mode = Mode::Editing(InputField::Level);
                            input.clear();
                        }
                        (KeyCode::Char('i'), _) => {
                            mode = Mode::Editing(InputField::Ip);
                            input.clear();
                        }
                        _ => {}
                    },
                    Mode::Editing(field) => match code {
                        KeyCode::Enter => {
                            let v = input.trim();
                            match field {
                                InputField::Keyword => {
                                    filters.keyword = if v.is_empty() { None } else { Some(v.into()) }
                                }
                                InputField::Date => filters.date = parse_date(v),
                                InputField::User => {
                                    filters.user = if v.is_empty() { None } else { Some(v.into()) }
                                }
                                InputField::Process => {
                                    filters.process =
                                        if v.is_empty() { None } else { Some(v.into()) }
                                }
                                InputField::Level => {
                                    filters.level =
                                        if v.is_empty() { None } else { Some(v.into()) }
                                }
                                InputField::Ip => {
                                    filters.ip = if v.is_empty() { None } else { Some(v.into()) }
                                }
                            }
                            mode = Mode::Normal;
                        }
                        KeyCode::Esc => mode = Mode::Normal,
                        KeyCode::Backspace => {
                            input.pop();
                        }
                        KeyCode::Char(c) => {
                            input.push(c);
                        }
                        _ => {}
                    },
                    Mode::Heatmap | Mode::Apps => match (code, modifiers) {
                        (KeyCode::Char('h'), _) | (KeyCode::Char('g'), _) | (KeyCode::Char('a'), _) => {
                            mode = Mode::Normal
                        }
                        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            break
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    // Clean up and exit
    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
