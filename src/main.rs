mod app;
mod browser;
mod config;
mod external;
mod preview;
mod ui;
mod ytdlp;

use std::io::{stdout, Result};
use std::time::Duration;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use crate::app::{AppScreenMode, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppState::new();

    let res = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Application error: {:?}", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut AppState,
) -> Result<()> {
    let tick_rate = Duration::from_millis(80);

    loop {
        app.handle_messages();
        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }

                match app.screen_mode {
                    AppScreenMode::Normal => {
                        if handle_normal_mode_key(&key, app) {
                            break;
                        }
                    }
                    AppScreenMode::FormatSelectModal => {
                        handle_format_modal_key(&key, app);
                    }
                    AppScreenMode::DirBrowserModal => {
                        handle_dir_browser_modal_key(&key, app);
                    }
                    AppScreenMode::ImageSelectModal => {
                        handle_image_modal_key(&key, app);
                    }
                }
            }
        }
    }

    Ok(())
}

fn handle_normal_mode_key(key: &event::KeyEvent, app: &mut AppState) -> bool {
    // 1. URL 入力モード時
    if app.is_editing_url {
        match key.code {
            KeyCode::Enter => {
                let url = app.url_input.clone();
                app.set_url(url);
                app.is_editing_url = false;
            }
            KeyCode::Esc => {
                app.is_editing_url = false;
            }
            KeyCode::Backspace => {
                app.url_input.pop();
            }
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.paste_clipboard();
            }
            KeyCode::Char(c) if c != '\n' && c != '\r' => {
                app.url_input.push(c);
            }
            _ => {}
        }
        return false;
    }

    // 2. コマンド・操作モード時
    match key.code {
        KeyCode::Char('q') => {
            return true;
        }
        KeyCode::Enter => {
            // 明示的なダウンロード開始（取得完了後のみ）
            app.start_download();
        }
        KeyCode::Tab => {
            app.toggle_mode();
        }
        KeyCode::Char('r') => {
            app.toggle_resolution();
        }
        KeyCode::Char('a') => {
            app.toggle_audio_format();
        }
        KeyCode::Char('d') => {
            app.dir_browser.set_root(app.current_dest_path.clone());
            app.screen_mode = AppScreenMode::DirBrowserModal;
        }
        KeyCode::Char('f') | KeyCode::Char('F') => {
            if app.media_info.is_some() {
                app.screen_mode = AppScreenMode::FormatSelectModal;
            } else if app.is_fetching_meta {
                app.status_message = "⏳ Media info still fetching. Please wait...".to_string();
            } else {
                app.status_message = "⚠️ No media loaded yet. Please paste or enter a URL first.".to_string();
            }
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            if app.media_info.is_some() {
                app.screen_mode = AppScreenMode::ImageSelectModal;
            } else if app.is_fetching_meta {
                app.status_message = "⏳ Media info still fetching. Please wait...".to_string();
            } else {
                app.status_message = "⚠️ No media loaded yet. Please paste or enter a URL first.".to_string();
            }
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            app.open_picard();
        }
        KeyCode::Char('v') => {
            app.paste_clipboard();
        }
        KeyCode::Char('u') | KeyCode::Char('/') => {
            app.is_editing_url = true;
        }
        _ => {}
    }

    false
}

fn handle_format_modal_key(key: &event::KeyEvent, app: &mut AppState) {
    let total_formats = app.media_info.as_ref().map(|m| m.formats.len()).unwrap_or(0);

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
            app.screen_mode = AppScreenMode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if total_formats > 0 && app.format_list_selected + 1 < total_formats {
                app.format_list_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.format_list_selected > 0 {
                app.format_list_selected -= 1;
            }
        }
        KeyCode::Char(' ') => {
            if let Some(ref info) = app.media_info {
                if let Some(f) = info.formats.get(app.format_list_selected) {
                    let id = f.format_id.clone();
                    if app.custom_video_format.as_deref() == Some(&id) {
                        app.custom_video_format = None;
                    } else {
                        app.custom_video_format = Some(id);
                    }
                }
            }
        }
        KeyCode::Char('a') => {
            if let Some(ref info) = app.media_info {
                if let Some(f) = info.formats.get(app.format_list_selected) {
                    let id = f.format_id.clone();
                    if app.custom_audio_format.as_deref() == Some(&id) {
                        app.custom_audio_format = None;
                    } else {
                        app.custom_audio_format = Some(id);
                    }
                }
            }
        }
        KeyCode::Char('c') => {
            app.custom_video_format = None;
            app.custom_audio_format = None;
            app.status_message = "Cleared custom stream selection. Reverted to Automatic Best.".to_string();
        }
        _ => {}
    }
}

fn handle_dir_browser_modal_key(key: &event::KeyEvent, app: &mut AppState) {
    // 1. 新規フォルダ作成プロンプト中
    if app.dir_browser.is_creating_dir {
        match key.code {
            KeyCode::Enter => {
                match app.dir_browser.confirm_create_dir() {
                    Ok(new_path) => {
                        app.current_dest_path = new_path.clone();
                        app.status_message = format!("📁 Created and selected new directory: {:?}", new_path);
                    }
                    Err(e) => {
                        app.dir_browser.error_message = Some(e.to_string());
                    }
                }
            }
            KeyCode::Esc => {
                app.dir_browser.cancel_create_dir();
            }
            KeyCode::Backspace => {
                app.dir_browser.new_dir_input.pop();
            }
            KeyCode::Char(c) => {
                app.dir_browser.new_dir_input.push(c);
            }
            _ => {}
        }
        return;
    }

    // 2. ディレクトリブラウズ中
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen_mode = AppScreenMode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let total = app.dir_browser.entries.len();
            if total > 0 && app.dir_browser.selected_index + 1 < total {
                app.dir_browser.selected_index += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.dir_browser.selected_index > 0 {
                app.dir_browser.selected_index -= 1;
            }
        }
        KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
            // サブディレクトリに入る (yaziの 'l' / Enter)
            app.dir_browser.enter_selected();
        }
        KeyCode::Char('h') | KeyCode::Left => {
            // 親ディレクトリへ移動 (yaziの 'h')
            app.dir_browser.go_parent();
        }
        KeyCode::Char(' ') => {
            // 現在のディレクトリを保存先として確定
            app.current_dest_path = app.dir_browser.current_path.clone();
            app.screen_mode = AppScreenMode::Normal;
            app.status_message = format!("📂 Destination folder selected: {:?}", app.current_dest_path);
        }
        KeyCode::Char('n') | KeyCode::Char('+') => {
            // 新規フォルダ作成モードに入る
            app.dir_browser.start_create_dir();
        }
        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
            let idx = (c as usize) - ('1' as usize);
            if idx < app.config.bookmarks.len() {
                app.switch_root_bookmark(idx);
            }
        }
        _ => {}
    }
}

fn handle_image_modal_key(key: &event::KeyEvent, app: &mut AppState) {
    let total_images = app.media_info.as_ref()
        .map(|m| m.get_unique_thumbnails().len())
        .unwrap_or(0);

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('i') | KeyCode::Char('I') => {
            app.screen_mode = AppScreenMode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if total_images > 0 && app.image_list_selected + 1 < total_images {
                app.image_list_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.image_list_selected > 0 {
                app.image_list_selected -= 1;
            }
        }
        KeyCode::Enter => {
            app.save_selected_image();
            app.screen_mode = AppScreenMode::Normal;
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            app.save_best_image();
            app.screen_mode = AppScreenMode::Normal;
        }
        _ => {}
    }
}

