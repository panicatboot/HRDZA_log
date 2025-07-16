use std::{env, path::PathBuf};
use anyhow::{Context, Result};
use genpdf::{
    Document,
    fonts,
    style::Style,
    PaperSize,
    SimplePageDecorator,
    elements::{LinearLayout, Paragraph},
    Element,          
};
use crate::{sources::LogEvent, filters::FilterConfig};


/// Generate a PDF report in $HOME/hrdza_logs.pdf using the DejaVuSans font.
pub fn generate_pdf(events: &[LogEvent], filters: &FilterConfig) -> Result<PathBuf> {
    // Load DejaVuSans from the standard Debian/Ubuntu fonts directory
    let font_family = fonts::from_files(
        "/usr/share/fonts/truetype/dejavu",
        "DejaVuSans",
        None,
    )
    .context("Failed to load DejaVuSans font from /usr/share/fonts/truetype/dejavu")?;

    let mut doc = Document::new(font_family);
    doc.set_paper_size(PaperSize::A4);
    doc.set_page_decorator(SimplePageDecorator::new());
    doc.set_title("HRDZA Logs Report");

    let mut layout = LinearLayout::vertical();
    // Now styled() is available
    layout.push(Paragraph::new("HRDZA Logs Report").styled(Style::new().bold()));
    layout.push(Paragraph::new(format!("Applied filters: {:?}", filters)));
    layout.push(Paragraph::new(""));

    for ev in events.iter().rev() {
        let ts: chrono::DateTime<chrono::Local> = ev.timestamp.into();
        layout.push(Paragraph::new(format!(
            "[{}][{}] {}",
            ts.format("%Y-%m-%d %H:%M:%S"),
            ev.source,
            ev.line
        )));
    }

    let home = env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    let out_path = home.join("hrdza_logs.pdf");
    doc.render_to_file(&out_path)
        .with_context(|| format!("Failed to write PDF to {}", out_path.display()))?;

    Ok(out_path)
}
