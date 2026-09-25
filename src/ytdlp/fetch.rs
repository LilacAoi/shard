use std::process::Stdio;
use tokio::process::Command;
use anyhow::{Context, Result};
use serde_json::Value;
use crate::ytdlp::models::MediaInfo;

/// 指定 URL の詳細メタデータ（フォーマット一覧含む）を非同期取得
pub async fn fetch_media_info(url: &str) -> Result<MediaInfo> {
    let output = Command::new("yt-dlp")
        .args(["--dump-json", "--no-playlist", url])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to execute yt-dlp to fetch media info")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("yt-dlp returned error: {}", err.trim());
    }

    let stdout = String::from_utf8(output.stdout)
        .context("Failed to parse yt-dlp output as UTF-8")?;

    // URL から動画IDパラメータ (v=...) を抽出（プレイリストURL時に該当曲を特定するため）
    let target_video_id = extract_video_id_from_url(url);

    // 1. まず単一の JSON としてパースを試みる
    if let Ok(val) = serde_json::from_str::<Value>(&stdout) {
        let info = MediaInfo::from_value(&val);
        if !info.id.is_empty() || info.title != "Untitled" {
            return Ok(info);
        }
    }

    // 2. 複数行（JSON Lines / プレイリストやアルバム）のパース
    // 各行が1つのJSONオブジェクトになっている
    let mut parsed_entries = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
            let info = MediaInfo::from_value(&val);
            if !info.id.is_empty() || info.title != "Untitled" {
                parsed_entries.push(info);
            }
        }
    }

    // 全トラックのタイトル一覧を作成
    let all_track_titles: Vec<String> = parsed_entries.iter().map(|e| e.title.clone()).collect();
    let total_tracks = parsed_entries.len();

    // 目的の曲（または先頭曲）を特定
    let mut selected_info = None;
    if let Some(ref tid) = target_video_id {
        if let Some(found) = parsed_entries.iter().find(|e| &e.id == tid) {
            selected_info = Some(found.clone());
        }
    }

    if selected_info.is_none() {
        selected_info = parsed_entries.into_iter().next();
    }

    if let Some(mut info) = selected_info {
        if total_tracks > 1 {
            info.playlist_count = Some(total_tracks);
            info.playlist_tracks = all_track_titles;
        }
        return Ok(info);
    }

    // エラー時はキャッシュディレクトリにデバッグログを保存
    let cache_dir = crate::config::AppConfig::cache_dir();
    let _ = std::fs::create_dir_all(&cache_dir);
    let log_path = cache_dir.join("last_error.log");
    let _ = std::fs::write(&log_path, &stdout);
    anyhow::bail!("Failed to parse media info from yt-dlp output (debug log: {})", log_path.display());
}

/// URL から "v=..." パラメータを抽出
fn extract_video_id_from_url(url: &str) -> Option<String> {
    if let Some(pos) = url.find("v=") {
        let after = &url[pos + 2..];
        let end = after.find('&').unwrap_or(after.len());
        let id = &after[..end];
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_video_id() {
        assert_eq!(
            extract_video_id_from_url("https://music.youtube.com/watch?v=0ctonkWiAqY&list=OLAK5uy_..."),
            Some("0ctonkWiAqY".to_string())
        );
        assert_eq!(
            extract_video_id_from_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(extract_video_id_from_url("https://youtu.be/xxx"), None);
    }

    #[test]
    fn test_parse_multi_line_json() {
        let sample_ndjson = r#"{"id": "track1", "title": "Track One", "uploader": "Artist A"}
{"id": "track2", "title": "Track Two", "uploader": "Artist A"}
{"id": "track3", "title": "Track Three", "uploader": "Artist A"}"#;

        let mut entries = Vec::new();
        for line in sample_ndjson.lines() {
            if let Ok(val) = serde_json::from_str::<Value>(line.trim()) {
                let info = MediaInfo::from_value(&val);
                if !info.id.is_empty() {
                    entries.push(info);
                }
            }
        }
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].title, "Track One");
        assert_eq!(entries[1].title, "Track Two");
        assert_eq!(entries[2].title, "Track Three");
    }
}
