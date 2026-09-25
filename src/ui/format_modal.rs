use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::app::AppState;

pub fn render_format_modal(frame: &mut Frame, area: Rect, app: &mut AppState) {
    let modal_area = centered_rect(88, 80, area);
    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta))
        .title(" 🔍 Select Format Streams (yt-dlp -F) ")
        .title_alignment(Alignment::Center);

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let Some(ref info) = app.media_info else {
        let p = Paragraph::new("No format information available yet. Please wait for metadata.")
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(p, inner);
        return;
    };

    let formats = &info.formats;
    if formats.is_empty() {
        let p = Paragraph::new("No stream formats found.")
            .style(Style::default().fg(Color::Red));
        frame.render_widget(p, inner);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Selected info
            Constraint::Min(5),    // Table
            Constraint::Length(2), // Help
        ])
        .split(inner);

    // 選択中のストリーム情報
    let sel_v = app.custom_video_format.as_deref().unwrap_or("Auto / Best");
    let sel_a = app.custom_audio_format.as_deref().unwrap_or("Auto / Best");
    let sel_info = Paragraph::new(format!("Selected Video: [{}]   Selected Audio: [{}]", sel_v, sel_a))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    frame.render_widget(sel_info, chunks[0]);

    // テーブル行の構築
    let rows: Vec<Row> = formats.iter().enumerate().map(|(idx, f)| {
        let is_row_focused = idx == app.format_list_selected;
        let is_video_selected = app.custom_video_format.as_ref().map(|id| id == &f.format_id).unwrap_or(false);
        let is_audio_selected = app.custom_audio_format.as_ref().map(|id| id == &f.format_id).unwrap_or(false);

        let marker = if is_video_selected && is_audio_selected {
            "★ [V+A]"
        } else if is_video_selected {
            "🎬 [V]"
        } else if is_audio_selected {
            "🎵 [A]"
        } else {
            ""
        };

        let style = if is_row_focused {
            Style::default().bg(Color::Rgb(40, 45, 60)).fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else if is_video_selected || is_audio_selected {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let vcodec = f.vcodec.as_deref().unwrap_or("--");
        let acodec = f.acodec.as_deref().unwrap_or("--");
        let fps_str = f.fps.map(|v| format!("{:.0}", v)).unwrap_or_else(|| "--".to_string());

        Row::new(vec![
            Cell::from(f.format_id.clone()),
            Cell::from(f.ext.clone()),
            Cell::from(f.resolution_str()),
            Cell::from(fps_str),
            Cell::from(vcodec),
            Cell::from(acodec),
            Cell::from(f.bitrate_str()),
            Cell::from(f.filesize_str()),
            Cell::from(marker),
        ]).style(style)
    }).collect();

    let header = Row::new(vec![
        "ID", "EXT", "RESOLUTION", "FPS", "VCODEC", "ACODEC", "BITRATE", "SIZE", "SELECTED"
    ]).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let widths = [
        Constraint::Length(8),
        Constraint::Length(6),
        Constraint::Length(14),
        Constraint::Length(6),
        Constraint::Length(16),
        Constraint::Length(14),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .highlight_symbol("> ");

    let mut state = TableState::default();
    state.select(Some(app.format_list_selected));
    frame.render_stateful_widget(table, chunks[1], &mut state);

    let help = Paragraph::new(" [j/k] Navigate  [Space] Toggle Video  [a] Toggle Audio  [c] Clear Formats  [Enter] Confirm  [Esc] Back")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
