use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::app::AppState;

pub fn render_image_modal(frame: &mut Frame, area: Rect, app: &mut AppState) {
    let modal_area = centered_rect(82, 75, area);
    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" 🖼️ Select Artwork / Thumbnail to Download ")
        .title_alignment(Alignment::Center);

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let Some(ref info) = app.media_info else {
        let p = Paragraph::new("\n   No media metadata available.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(p, inner);
        return;
    };

    let thumbs = info.get_unique_thumbnails();
    if thumbs.is_empty() {
        let p = Paragraph::new("\n   No image / thumbnail candidates found.")
            .style(Style::default().fg(Color::Red));
        frame.render_widget(p, inner);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Info line
            Constraint::Min(5),    // Table
            Constraint::Length(2), // Help line
        ])
        .split(inner);

    // 1. 保存先と対象メディアの情報
    let title_display = if info.title.len() > 40 {
        format!("{}...", &info.title[..37])
    } else {
        info.title.clone()
    };
    let info_line = Line::from(vec![
        Span::styled("Media: ", Style::default().fg(Color::Cyan)),
        Span::styled(title_display, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw("   |   "),
        Span::styled("Save to: ", Style::default().fg(Color::Cyan)),
        Span::styled(format!("{}", app.current_dest_path.display()), Style::default().fg(Color::Yellow)),
    ]);
    frame.render_widget(Paragraph::new(info_line), chunks[0]);

    // 2. テーブル行の構築
    let rows: Vec<Row> = thumbs.iter().enumerate().map(|(idx, t)| {
        let is_focused = idx == app.image_list_selected;

        let style = if is_focused {
            Style::default().bg(Color::Rgb(40, 45, 60)).fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let num_str = format!("#{}", idx + 1);
        let res_str = t.resolution_str();
        let ext_str = t.ext_str().to_uppercase();
        let id_str = if t.id.is_empty() { "auto" } else { &t.id };
        let tag_str = if idx == 0 {
            "★ Best (Auto)"
        } else if res_str.contains("x") && res_str.split('x').next() == res_str.split('x').nth(1) {
            "🔲 Square (Cover)"
        } else {
            ""
        };

        Row::new(vec![
            Cell::from(num_str),
            Cell::from(res_str),
            Cell::from(ext_str),
            Cell::from(id_str.to_string()),
            Cell::from(tag_str.to_string()),
            Cell::from(t.url.clone()),
        ]).style(style)
    }).collect();

    let header = Row::new(vec![
        "NO", "RESOLUTION", "FORMAT", "ID / TYPE", "TAG", "URL"
    ]).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let widths = [
        Constraint::Length(5),
        Constraint::Length(14),
        Constraint::Length(8),
        Constraint::Length(16),
        Constraint::Length(18),
        Constraint::Min(20),
    ];

    let mut state = TableState::default();
    state.select(Some(app.image_list_selected));

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::NONE))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, chunks[1], &mut state);

    // 3. ヘルプライン
    let help_line = Line::from(vec![
        Span::styled("[Enter] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw("Save Selected  "),
        Span::styled("[a] ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        Span::raw("Auto Best (cover.jpg)  "),
        Span::styled("[j/k] ", Style::default().fg(Color::Yellow)),
        Span::raw("Select  "),
        Span::styled("[Esc/i] ", Style::default().fg(Color::DarkGray)),
        Span::raw("Close"),
    ]);
    frame.render_widget(Paragraph::new(help_line), chunks[2]);
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
