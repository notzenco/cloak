use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use image::GenericImageView;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Paragraph, Tabs, Wrap},
};

use cloak_core::analysis::{self, AnalysisResult, BitPlane};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    BitPlane,
    Histogram,
    Analysis,
}

impl Tab {
    const ALL: [Tab; 3] = [Tab::BitPlane, Tab::Histogram, Tab::Analysis];

    fn title(&self) -> &str {
        match self {
            Tab::BitPlane => "Bit Planes",
            Tab::Histogram => "Histogram",
            Tab::Analysis => "Analysis",
        }
    }

    fn index(&self) -> usize {
        match self {
            Tab::BitPlane => 0,
            Tab::Histogram => 1,
            Tab::Analysis => 2,
        }
    }

    fn next(&self) -> Self {
        match self {
            Tab::BitPlane => Tab::Histogram,
            Tab::Histogram => Tab::Analysis,
            Tab::Analysis => Tab::BitPlane,
        }
    }

    fn prev(&self) -> Self {
        match self {
            Tab::BitPlane => Tab::Analysis,
            Tab::Histogram => Tab::BitPlane,
            Tab::Analysis => Tab::Histogram,
        }
    }
}

struct AppState {
    image_data: Vec<u8>,
    filename: String,
    width: u32,
    height: u32,
    analysis: AnalysisResult,
    active_tab: Tab,
    // Bit plane controls
    bp_channel: usize, // 0=R, 1=G, 2=B
    bp_bit: u8,        // 0-7
    bit_plane: BitPlane,
    // Capacity
    capacity: usize,
}

impl AppState {
    fn new(image_data: Vec<u8>, filename: String) -> Result<Self, Box<dyn std::error::Error>> {
        let img = image::load_from_memory(&image_data)?;
        let (width, height) = img.dimensions();
        let analysis_result = analysis::analyze_image(&image_data)?;
        let bit_plane = analysis::extract_bit_plane(&image_data, 0, 0)?;
        let capacity = cloak_core::capacity(&image_data, Some(&filename)).unwrap_or(0);

        Ok(Self {
            image_data,
            filename,
            width,
            height,
            analysis: analysis_result,
            active_tab: Tab::BitPlane,
            bp_channel: 0,
            bp_bit: 0,
            bit_plane,
            capacity,
        })
    }

    fn update_bit_plane(&mut self) {
        if let Ok(bp) = analysis::extract_bit_plane(&self.image_data, self.bp_channel, self.bp_bit)
        {
            self.bit_plane = bp;
        }
    }

    fn channel_name(&self) -> &str {
        match self.bp_channel {
            0 => "Red",
            1 => "Green",
            2 => "Blue",
            _ => unreachable!(),
        }
    }
}

pub fn run_tui(image_data: &[u8], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = AppState::new(image_data.to_vec(), filename.to_string())?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|frame| draw(frame, &state))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Tab | KeyCode::Right => {
                    state.active_tab = state.active_tab.next();
                }
                KeyCode::BackTab | KeyCode::Left => {
                    state.active_tab = state.active_tab.prev();
                }
                // Bit-plane controls
                KeyCode::Char('r') => {
                    state.bp_channel = 0;
                    state.update_bit_plane();
                }
                KeyCode::Char('g') => {
                    state.bp_channel = 1;
                    state.update_bit_plane();
                }
                KeyCode::Char('b') => {
                    state.bp_channel = 2;
                    state.update_bit_plane();
                }
                KeyCode::Up => {
                    if state.bp_bit < 7 {
                        state.bp_bit += 1;
                        state.update_bit_plane();
                    }
                }
                KeyCode::Down => {
                    if state.bp_bit > 0 {
                        state.bp_bit -= 1;
                        state.update_bit_plane();
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn draw(frame: &mut ratatui::Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tabs
            Constraint::Length(5), // metadata bar
            Constraint::Min(10),   // main content
            Constraint::Length(1), // help bar
        ])
        .split(frame.area());

    draw_tabs(frame, state, chunks[0]);
    draw_metadata(frame, state, chunks[1]);

    match state.active_tab {
        Tab::BitPlane => draw_bit_plane(frame, state, chunks[2]),
        Tab::Histogram => draw_histogram(frame, state, chunks[2]),
        Tab::Analysis => draw_analysis(frame, state, chunks[2]),
    }

    draw_help(frame, state, chunks[3]);
}

fn draw_tabs(frame: &mut ratatui::Frame, state: &AppState, area: Rect) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|t| Line::from(Span::raw(t.title())))
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("cloak"))
        .select(state.active_tab.index())
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, area);
}

fn draw_metadata(frame: &mut ratatui::Frame, state: &AppState, area: Rect) {
    let format = cloak_core::ImageFormat::detect(&state.image_data, Some(&state.filename))
        .map(|f| format!("{f:?}"))
        .unwrap_or_else(|_| "Unknown".into());

    let text = vec![
        Line::from(vec![
            Span::styled("File: ", Style::default().fg(Color::DarkGray)),
            Span::raw(&state.filename),
            Span::raw("  "),
            Span::styled("Format: ", Style::default().fg(Color::DarkGray)),
            Span::raw(&format),
        ]),
        Line::from(vec![
            Span::styled("Size: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}x{}", state.width, state.height)),
            Span::raw("  "),
            Span::styled("Pixels: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}", state.analysis.pixel_count)),
            Span::raw("  "),
            Span::styled("Capacity: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{} bytes", state.capacity)),
        ]),
    ];

    let block = Block::default().borders(Borders::ALL).title("Image Info");
    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_bit_plane(frame: &mut ratatui::Frame, state: &AppState, area: Rect) {
    let bp = &state.bit_plane;

    // Show a text-based visualization of the bit plane
    let max_cols = (area.width as usize).saturating_sub(2);
    let max_rows = (area.height as usize).saturating_sub(2);

    let step_x = (bp.width as usize).max(1) / max_cols.max(1);
    let step_y = (bp.height as usize).max(1) / max_rows.max(1);
    let step_x = step_x.max(1);
    let step_y = step_y.max(1);

    let display_w = (bp.width as usize / step_x).min(max_cols);
    let display_h = (bp.height as usize / step_y).min(max_rows);

    let mut lines = Vec::with_capacity(display_h);
    for row in 0..display_h {
        let y = row * step_y;
        let mut chars = String::with_capacity(display_w);
        for col in 0..display_w {
            let x = col * step_x;
            let idx = y * bp.width as usize + x;
            if idx < bp.data.len() {
                chars.push(if bp.data[idx] == 1 { '\u{2588}' } else { ' ' });
            }
        }
        lines.push(Line::from(chars));
    }

    let title = format!(
        "Bit Plane — {} channel, bit {} ({})",
        state.channel_name(),
        state.bp_bit,
        if state.bp_bit == 0 {
            "LSB"
        } else if state.bp_bit == 7 {
            "MSB"
        } else {
            ""
        }
    );

    let block = Block::default().borders(Borders::ALL).title(title);
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_histogram(frame: &mut ratatui::Frame, state: &AppState, area: Rect) {
    let hist = &state.analysis.histogram;
    let max_val = *hist.iter().max().unwrap_or(&1) as f64;

    // Group into 16 bins of 16 values each for display
    let bin_count = 16;
    let bin_size = 256 / bin_count;
    let bars: Vec<Bar> = (0..bin_count)
        .map(|i| {
            let start = i * bin_size;
            let end = start + bin_size;
            let sum: u64 = hist[start..end].iter().sum();
            let label = format!("{start}");
            Bar::default()
                .value(sum)
                .label(Line::from(label))
                .style(Style::default().fg(Color::Cyan))
        })
        .collect();

    let bar_chart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Pixel Value Histogram (R+G+B)"),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(3)
        .bar_gap(1)
        .max(max_val as u64);

    frame.render_widget(bar_chart, area);
}

fn draw_analysis(frame: &mut ratatui::Frame, state: &AppState, area: Rect) {
    let a = &state.analysis;

    let verdict = if a.p_value < 0.01 {
        Span::styled(
            "LIKELY CONTAINS HIDDEN DATA",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )
    } else if a.p_value < 0.05 {
        Span::styled(
            "POSSIBLY CONTAINS HIDDEN DATA",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            "NO STRONG EVIDENCE OF HIDDEN DATA",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    };

    let confidence = ((1.0 - a.p_value) * 100.0).min(99.99);

    let mut text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Chi-Square Statistic: ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(format!("{:.4}", a.chi_square)),
        ]),
        Line::from(vec![
            Span::styled(
                "P-Value:              ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(format!("{:.6}", a.p_value)),
        ]),
        Line::from(vec![
            Span::styled(
                "Confidence:           ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(format!("{confidence:.2}%")),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Verdict: ", Style::default().fg(Color::DarkGray)),
            verdict,
        ]),
        Line::from(""),
    ];

    if let Some(rs) = &a.rs {
        text.push(Line::from(vec![
            Span::styled("RS Analysis Rate:     ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{:.4}", rs.estimated_rate)),
        ]));
        text.push(Line::from(vec![
            Span::styled("  R_m: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{:.4}", rs.r_m)),
            Span::styled("  S_m: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{:.4}", rs.s_m)),
            Span::styled("  R-m: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{:.4}", rs.r_neg_m)),
            Span::styled("  S-m: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{:.4}", rs.s_neg_m)),
        ]));
    }

    if let Some(sp) = &a.sample_pairs {
        text.push(Line::from(vec![
            Span::styled("Sample Pairs Rate:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{:.4}", sp.estimated_rate)),
        ]));
        text.push(Line::from(vec![
            Span::styled("  Total: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}", sp.total_pairs)),
            Span::styled("  Close: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}", sp.close_pairs)),
        ]));
    }

    text.extend([
        Line::from(""),
        Line::from(Span::styled(
            "The chi-square test examines whether LSB values show",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "patterns inconsistent with natural image data.",
            Style::default().fg(Color::DarkGray),
        )),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Steganalysis Results");
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_help(frame: &mut ratatui::Frame, _state: &AppState, area: Rect) {
    let help = Line::from(vec![
        Span::styled("Tab", Style::default().fg(Color::Cyan)),
        Span::raw(": switch panel  "),
        Span::styled("r/g/b", Style::default().fg(Color::Cyan)),
        Span::raw(": channel  "),
        Span::styled("\u{2191}/\u{2193}", Style::default().fg(Color::Cyan)),
        Span::raw(": bit  "),
        Span::styled("q", Style::default().fg(Color::Cyan)),
        Span::raw(": quit"),
    ]);

    frame.render_widget(Paragraph::new(help), area);
}
