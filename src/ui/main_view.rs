use ratatui::prelude::*;
use ratatui::widgets::*;
use ratatui_image::StatefulImage;
use crate::app::AppState;
use crate::ytdlp::download::DownloadMode;

const SPINNER_CHARS: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn render_main_view(frame: &mut Frame, area: Rect, app: &mut AppState) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // URL Input
            Constraint::Min(12),   // Middle: Preview + Info
            Constraint::Length(3), // Mode & Settings Selectors
            Constraint::Length(3), // Destination Dir
            Constraint::Length(3), // Progress Gauge
            Constraint::Length(1), // Status message
            Constraint::Length(1), // Keybindings Footer
        ])
        .split(area);

    render_url_bar(frame, main_chunks[0], app);
    render_preview_and_info(frame, main_chunks[1], app);
    render_selectors(frame, main_chunks[2], app);
    render_destination(frame, main_chunks[3], app);
    render_progress(frame, main_chunks[4], app);
    render_status(frame, main_chunks[5], app);
    render_footer(frame, main_chunks[6], app);
}

fn render_url_bar(frame: &mut Frame, area: Rect, app: &AppState) {
    let border_color = if app.is_editing_url { Color::Yellow } else { Color::Cyan };
    let title = if app.is_editing_url {
        " [URL Input Mode: Type URL and press Enter to inspect, Esc to cancel] "
    } else {
        " URL (Press 'u' or '/' to edit, 'v' to paste from clipboard) "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let display_text = if app.url_input.is_empty() {
        Span::styled(
            " https://www.youtube.com/watch?v=... (Press 'v' to paste)",
            Style::default().fg(Color::DarkGray)
        )
    } else {
        let cursor_char = if app.is_editing_url { "█" } else { "" };
        Span::styled(format!(" {}{}", app.url_input, cursor_char), Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
    };

    let p = Paragraph::new(display_text).block(block);
    frame.render_widget(p, area);
}

fn render_preview_and_info(frame: &mut Frame, area: Rect, app: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45), // Preview
            Constraint::Percentage(55), // Metadata
        ])
        .split(area);

    // --- 左カラム: 画像プレビュー ---
    let proto_name = app.preview_manager.protocol_name();
    let preview_title = format!(" 🖼️ Preview [{}] ", proto_name);
    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Blue))
        .title(preview_title);

    let preview_inner = preview_block.inner(chunks[0]);
    frame.render_widget(preview_block, chunks[0]);

    if let Some(proto) = app.preview_manager.protocol_mut() {
        let image_widget = StatefulImage::default();
        frame.render_stateful_widget(image_widget, preview_inner, proto);
    } else if app.is_fetching_meta {
        let spinner = SPINNER_CHARS[app.spinner_frame % SPINNER_CHARS.len()];
        let p = Paragraph::new(format!("\n\n      {} Fetching thumbnail...", spinner))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        frame.render_widget(p, preview_inner);
    } else {
        let empty = Paragraph::new("\n\n      No preview available.\n      Paste a URL ('v') to inspect.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, preview_inner);
    }

    // --- 右カラム: メディア詳細情報 ---
    let info_title = if app.media_info.as_ref().map(|m| m.playlist_count.is_some()).unwrap_or(false) {
        " ℹ️ Media Information (Album / Playlist) "
    } else {
        " ℹ️ Media Information "
    };

    let info_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(info_title);

    let info_inner = info_block.inner(chunks[1]);
    frame.render_widget(info_block, chunks[1]);

    if let Some(ref info) = app.media_info {
        let mut lines = Vec::new();

        if let Some(count) = info.playlist_count {
            let album_name = info.album.as_deref()
                .or(info.playlist_title.as_deref())
                .unwrap_or("Album/Playlist");
            lines.push(Line::from(vec![
                Span::styled("💽 Album:    ", Style::default().fg(Color::LightMagenta).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{} ({} Tracks)", album_name, count), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
        }

        lines.push(Line::from(vec![
            Span::styled("🎵 Title:    ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(&info.title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("👤 Artist:   ", Style::default().fg(Color::Cyan)),
            Span::styled(info.uploader_name(), Style::default().fg(Color::White)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("⏱️ Duration: ", Style::default().fg(Color::Cyan)),
            Span::styled(format!("{}  (Uploaded: {})", info.duration_formatted(), info.upload_date.as_deref().unwrap_or("Unknown")), Style::default().fg(Color::White)),
        ]));

        let thumb_count = info.get_unique_thumbnails().len();
        lines.push(Line::from(vec![
            Span::styled("🖼️ Artwork:  ", Style::default().fg(Color::Cyan)),
            Span::styled(format!("{} candidates ", thumb_count), Style::default().fg(Color::White)),
            Span::styled("(Press 'i' to select/save)", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("⚡ Streams:  ", Style::default().fg(Color::Cyan)),
            Span::styled(format!("{} available (Press 'F' to pick)", info.formats.len()), Style::default().fg(Color::Green)),
        ]));

        let dl_type = if app.custom_video_format.is_some() || app.custom_audio_format.is_some() {
            let v = app.custom_video_format.as_deref().unwrap_or("None");
            let a = app.custom_audio_format.as_deref().unwrap_or("None");
            Line::from(vec![
                Span::styled("🚀 Download: ", Style::default().fg(Color::Cyan)),
                Span::styled(format!("Custom [Video: {}, Audio: {}]", v, a), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ])
        } else {
            Line::from(vec![
                Span::styled("🚀 Download: ", Style::default().fg(Color::Cyan)),
                Span::styled("Automatic Best Quality (Press Enter to start)", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ])
        };
        lines.push(dl_type);

        if !info.playlist_tracks.is_empty() {
            let total = info.playlist_tracks.len();
            lines.push(Line::default());
            lines.push(Line::from(vec![
                Span::styled("📋 Track List: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(format!("(Total {} tracks)", total), Style::default().fg(Color::DarkGray)),
            ]));

            let current_lines_count = lines.len();
            let avail_lines = (info_inner.height as usize).saturating_sub(current_lines_count).max(6);
            let inner_width = info_inner.width as usize;

            if inner_width >= 52 && total > 5 {
                // 2列（左右並列）レイアウト
                let col_width = (inner_width.saturating_sub(2)) / 2;
                let rows_needed = (total + 1) / 2;
                let rows_to_display = rows_needed.min(avail_lines);
                let displayed_total = rows_to_display * 2;

                for row in 0..rows_to_display {
                    let idx1 = row;
                    let idx2 = row + rows_needed;

                    let mut spans = Vec::new();

                    // 左列
                    if idx1 < total {
                        let track_num = format!(" {:2}. ", idx1 + 1);
                        let num_w = track_num.len();
                        let max_title_w = col_width.saturating_sub(num_w + 1);
                        let title = &info.playlist_tracks[idx1];
                        let title_w = str_display_width(title);
                        let (disp_title, disp_w) = if title_w > max_title_w {
                            let truncated = truncate_to_display_width(title, max_title_w.saturating_sub(2));
                            let tw = str_display_width(&truncated) + 2;
                            (format!("{}..", truncated), tw)
                        } else {
                            (title.clone(), title_w)
                        };

                        let pad = col_width.saturating_sub(num_w + disp_w);
                        spans.push(Span::styled(track_num, Style::default().fg(Color::Yellow)));
                        spans.push(Span::styled(disp_title, Style::default().fg(Color::White)));
                        spans.push(Span::raw(" ".repeat(pad)));
                    }

                    // 右列
                    if idx2 < total {
                        let track_num = format!(" {:2}. ", idx2 + 1);
                        let num_w = track_num.len();
                        let max_title_w = col_width.saturating_sub(num_w + 1);
                        let title = &info.playlist_tracks[idx2];
                        let title_w = str_display_width(title);
                        let (disp_title, _) = if title_w > max_title_w {
                            let truncated = truncate_to_display_width(title, max_title_w.saturating_sub(2));
                            (format!("{}..", truncated), 0)
                        } else {
                            (title.clone(), title_w)
                        };

                        spans.push(Span::styled(track_num, Style::default().fg(Color::Yellow)));
                        spans.push(Span::styled(disp_title, Style::default().fg(Color::White)));
                    }

                    lines.push(Line::from(spans));
                }

                if total > displayed_total {
                    lines.push(Line::from(vec![
                        Span::styled(format!("   ... and {} more tracks", total - displayed_total), Style::default().fg(Color::DarkGray)),
                    ]));
                }
            } else {
                // 1列レイアウト
                let display_count = avail_lines.min(total);
                for (idx, track) in info.playlist_tracks.iter().take(display_count).enumerate() {
                    let track_num = format!("   {:2}. ", idx + 1);
                    let max_title_w = inner_width.saturating_sub(track_num.len() + 2);
                    let title_w = str_display_width(track);
                    let disp_title = if title_w > max_title_w {
                        let truncated = truncate_to_display_width(track, max_title_w.saturating_sub(2));
                        format!("{}..", truncated)
                    } else {
                        track.clone()
                    };
                    lines.push(Line::from(vec![
                        Span::styled(track_num, Style::default().fg(Color::Yellow)),
                        Span::styled(disp_title, Style::default().fg(Color::White)),
                    ]));
                }
                if total > display_count {
                    lines.push(Line::from(vec![
                        Span::styled(format!("   ... and {} more tracks", total - display_count), Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, info_inner);
    } else if app.is_fetching_meta {
        let spinner = SPINNER_CHARS[app.spinner_frame % SPINNER_CHARS.len()];
        let p = Paragraph::new(format!("\n\n   {} Inspecting media with yt-dlp...\n   Please wait for details before downloading.", spinner))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        frame.render_widget(p, info_inner);
    } else {
        let p = Paragraph::new("\n\n   No media loaded.\n   Press 'v' to paste URL from clipboard.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(p, info_inner);
    }
}

fn render_selectors(frame: &mut Frame, area: Rect, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33), // Mode
            Constraint::Percentage(34), // Resolution Limit
            Constraint::Percentage(33), // Audio Format
        ])
        .split(area);

    // 1. Mode
    let mode_str = match &app.mode {
        DownloadMode::Video => "🎬 Video (Max Res)",
        DownloadMode::Audio => "🎵 Audio (Picard Tagging)",
        DownloadMode::ThumbnailOnly => "🖼️ Thumbnail Only",
        DownloadMode::CustomFormats { .. } => "🔍 Custom Streams (-F)",
    };
    let mode_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" [Tab] Mode ");
    let mode_p = Paragraph::new(format!(" {}", mode_str))
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .block(mode_block);
    frame.render_widget(mode_p, chunks[0]);

    // 2. Resolution Limit
    let res_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" [r] Max Resolution ");
    let res_color = if matches!(app.mode, DownloadMode::Video) { Color::Green } else { Color::DarkGray };
    let res_p = Paragraph::new(format!(" {}", app.max_resolution.label()))
        .style(Style::default().fg(res_color).add_modifier(Modifier::BOLD))
        .block(res_block);
    frame.render_widget(res_p, chunks[1]);

    // 3. Audio Format
    let audio_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" [a] Audio Format ");
    let audio_color = if matches!(app.mode, DownloadMode::Audio) { Color::Magenta } else { Color::DarkGray };
    let audio_p = Paragraph::new(format!(" {} (320k/Best + Metadata)", app.audio_format.as_str().to_uppercase()))
        .style(Style::default().fg(audio_color).add_modifier(Modifier::BOLD))
        .block(audio_block);
    frame.render_widget(audio_p, chunks[2]);
}

fn render_destination(frame: &mut Frame, area: Rect, app: &AppState) {
    let bm_name = app.config.bookmarks.get(app.current_bookmark_index)
        .map(|b| b.name.as_str())
        .unwrap_or("Custom");

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" [d] Destination Folder (Press 'd' to browse subfolders / create new folder) ");

    let content = Line::from(vec![
        Span::styled(format!(" [{}] 📂 {} ", app.current_bookmark_index + 1, bm_name), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" ->  {} ", app.current_dest_path.display()), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]);

    let p = Paragraph::new(content).block(block);
    frame.render_widget(p, area);
}

fn render_progress(frame: &mut Frame, area: Rect, app: &AppState) {
    let percent = app.progress.percent.clamp(0.0, 100.0) as u16;
    let gauge_title = if app.is_downloading {
        format!(" Downloading: {}% (Speed: {}, ETA: {}) ", percent, app.progress.speed, app.progress.eta)
    } else if percent >= 100 {
        " Completed! ".to_string()
    } else {
        " Ready (Waiting for download trigger) ".to_string()
    };

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(gauge_title))
        .gauge_style(Style::default().fg(Color::Green).bg(Color::Rgb(30, 30, 40)))
        .percent(percent);

    frame.render_widget(gauge, area);
}

fn render_status(frame: &mut Frame, area: Rect, app: &AppState) {
    let status = Paragraph::new(format!(" {}", app.status_message))
        .style(Style::default().fg(Color::LightCyan));
    frame.render_widget(status, area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &AppState) {
    let start_color = if app.media_info.is_some() && !app.is_downloading {
        Color::Green
    } else {
        Color::DarkGray
    };

    let help_line = Line::from(vec![
        Span::styled("[Enter] ", Style::default().fg(start_color).add_modifier(Modifier::BOLD)),
        Span::styled("Download  ", Style::default().fg(start_color)),
        Span::styled("[Tab] ", Style::default().fg(Color::Yellow)),
        Span::raw("Mode  "),
        Span::styled("[r] ", Style::default().fg(Color::Yellow)),
        Span::raw("Res  "),
        Span::styled("[a] ", Style::default().fg(Color::Yellow)),
        Span::raw("Audio  "),
        Span::styled("[i] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw("Artwork  "),
        Span::styled("[F] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw("Streams  "),
        Span::styled("[d] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("Folder  "),
        Span::styled("[p] ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        Span::raw("Picard  "),
        Span::styled("[v] ", Style::default().fg(Color::Yellow)),
        Span::raw("Paste  "),
        Span::styled("[q] ", Style::default().fg(Color::Red)),
        Span::raw("Quit"),
    ]);

    let footer = Paragraph::new(help_line).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, area);
}

fn str_display_width(s: &str) -> usize {
    s.chars().map(|c| if c as u32 > 0x7f { 2 } else { 1 }).sum()
}

fn truncate_to_display_width(s: &str, max_width: usize) -> String {
    let mut cur_width = 0;
    let mut res = String::new();
    for c in s.chars() {
        let w = if c as u32 > 0x7f { 2 } else { 1 };
        if cur_width + w > max_width {
            break;
        }
        cur_width += w;
        res.push(c);
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_width() {
        assert_eq!(str_display_width("Milk Bar P.M.11:00"), 18);
        assert_eq!(str_display_width("さよなら"), 8);
        assert_eq!(str_display_width("午前3時の太陽"), 13);
    }

    #[test]
    fn test_truncation() {
        assert_eq!(truncate_to_display_width("Milk Bar P.M.11:00", 10), "Milk Bar P");
        assert_eq!(truncate_to_display_width("さよならアンディ", 8), "さよなら");
    }
}


