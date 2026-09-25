use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::app::AppState;

pub fn render_dir_browser_modal(frame: &mut Frame, area: Rect, app: &mut AppState) {
    let modal_area = centered_rect(80, 75, area);
    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" 📂 Select Download Directory ")
        .title_alignment(Alignment::Center);

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Root bookmarks selector
            Constraint::Length(2), // Current full path
            Constraint::Min(6),    // Subdirectory list
            Constraint::Length(2), // Help footer
        ])
        .split(inner);

    // 1. ルートディレクトリスイッチ
    let root_spans: Vec<Span> = app.config.bookmarks.iter().enumerate().flat_map(|(idx, bm)| {
        let is_active = idx == app.current_bookmark_index;
        let style = if is_active {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(Color::Rgb(40, 40, 50))
        };

        vec![
            Span::styled(format!(" [{}] {} ", idx + 1, bm.name), style),
            Span::raw(" "),
        ]
    }).collect();

    let root_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Registered Root Dirs (Press 1-9 to switch root) ");
    let root_p = Paragraph::new(Line::from(root_spans)).block(root_block);
    frame.render_widget(root_p, chunks[0]);

    // 2. 現在のパス表示
    let current_path_str = app.dir_browser.current_path.display().to_string();
    let path_line = Line::from(vec![
        Span::styled("Current Destination: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(current_path_str, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]);
    frame.render_widget(Paragraph::new(path_line), chunks[1]);

    // 3. サブディレクトリ一覧
    let items: Vec<ListItem> = app.dir_browser.entries.iter().enumerate().map(|(idx, item)| {
        let is_selected = idx == app.dir_browser.selected_index;
        let icon = if item.is_parent { "⤴️ " } else { "📁 " };
        let content = format!("{}{}", icon, item.name);

        let style = if is_selected {
            Style::default().bg(Color::Rgb(45, 55, 75)).fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else if item.is_parent {
            Style::default().fg(Color::LightCyan)
        } else {
            Style::default().fg(Color::White)
        };

        ListItem::new(content).style(style)
    }).collect();

    let list_title = if app.dir_browser.entries.is_empty() {
        " Subdirectories (Empty / Press 'n' to create one) "
    } else {
        " Subdirectories (Enter to open, Space to select as destination) "
    };

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(list_title);

    let list = List::new(items)
        .block(list_block)
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(Some(app.dir_browser.selected_index));
    frame.render_stateful_widget(list, chunks[2], &mut state);

    // 4. フッター
    let help_line = Line::from(vec![
        Span::styled("[h/l] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("Parent/Child (yazi)  "),
        Span::styled("[j/k] ", Style::default().fg(Color::Yellow)),
        Span::raw("Up/Down  "),
        Span::styled("[Space] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw("Select This Folder  "),
        Span::styled("[n] ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        Span::raw("New Folder (mkdir)  "),
        Span::styled("[1-9] ", Style::default().fg(Color::Yellow)),
        Span::raw("Root  "),
        Span::styled("[Esc] ", Style::default().fg(Color::DarkGray)),
        Span::raw("Close"),
    ]);
    frame.render_widget(Paragraph::new(help_line), chunks[3]);

    // 5. 新規フォルダ作成プロンプト（表示中の場合）
    if app.dir_browser.is_creating_dir {
        render_create_dir_prompt(frame, modal_area, app);
    }
}

fn render_create_dir_prompt(frame: &mut Frame, area: Rect, app: &AppState) {
    let prompt_area = centered_rect(55, 30, area);
    frame.render_widget(Clear, prompt_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta))
        .title(" 📁 Create New Subdirectory (mkdir) ")
        .title_alignment(Alignment::Center);

    let inner = block.inner(prompt_area);
    frame.render_widget(block, prompt_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Description
            Constraint::Length(3), // Input box
            Constraint::Min(1),    // Error or guide
        ])
        .split(inner);

    let desc = Paragraph::new(format!("Parent: {}", app.dir_browser.current_path.display()))
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(desc, chunks[0]);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Folder Name ");
    let input_text = format!(" {}█", app.dir_browser.new_dir_input);
    let input_p = Paragraph::new(input_text)
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .block(input_block);
    frame.render_widget(input_p, chunks[1]);

    let guide_text = if let Some(ref err) = app.dir_browser.error_message {
        Line::from(vec![
            Span::styled("❌ Error: ", Style::default().fg(Color::Red)),
            Span::styled(err, Style::default().fg(Color::Red)),
        ])
    } else {
        Line::from(vec![
            Span::styled("[Enter] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("Create & Select  "),
            Span::styled("[Esc] ", Style::default().fg(Color::DarkGray)),
            Span::raw("Cancel"),
        ])
    };
    frame.render_widget(Paragraph::new(guide_text), chunks[2]);
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
