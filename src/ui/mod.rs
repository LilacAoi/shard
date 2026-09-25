pub mod main_view;
pub mod format_modal;
pub mod dir_browser_modal;
pub mod image_modal;

use ratatui::Frame;
use crate::app::{AppScreenMode, AppState};

pub fn render(frame: &mut Frame, app: &mut AppState) {
    let area = frame.area();

    // 常にベースのメイン画面を描画
    main_view::render_main_view(frame, area, app);

    // モーダルがアクティブならオーバーレイ描画
    match app.screen_mode {
        AppScreenMode::Normal => {}
        AppScreenMode::FormatSelectModal => {
            format_modal::render_format_modal(frame, area, app);
        }
        AppScreenMode::DirBrowserModal => {
            dir_browser_modal::render_dir_browser_modal(frame, area, app);
        }
        AppScreenMode::ImageSelectModal => {
            image_modal::render_image_modal(frame, area, app);
        }
    }
}
