use std::process::Command;
use arboard::Clipboard;

/// クリップボードから URL を取得
pub fn get_clipboard_url() -> Option<String> {
    // 1. arboard による標準取得
    if let Ok(mut clipboard) = Clipboard::new() {
        if let Ok(text) = clipboard.get_text() {
            let trimmed = text.trim();
            if is_valid_url(trimmed) {
                return Some(trimmed.to_string());
            }
        }
    }

    // 2. Wayland 環境の wl-paste フォールバック
    if let Ok(output) = Command::new("wl-paste").arg("--no-newline").output() {
        if output.status.success() {
            if let Ok(text) = String::from_utf8(output.stdout) {
                let trimmed = text.trim();
                if is_valid_url(trimmed) {
                    return Some(trimmed.to_string());
                }
            }
        }
    }

    // 3. X11 環境の xclip フォールバック
    if let Ok(output) = Command::new("xclip").args(["-selection", "clipboard", "-o"]).output() {
        if output.status.success() {
            if let Ok(text) = String::from_utf8(output.stdout) {
                let trimmed = text.trim();
                if is_valid_url(trimmed) {
                    return Some(trimmed.to_string());
                }
            }
        }
    }

    None
}

/// URL かどうかの簡易検証
pub fn is_valid_url(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}
