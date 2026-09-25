use serde::{Deserialize, Serialize};
use serde_json::Value;

/// yt-dlp -J で取得できるメディア全体の基本情報
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MediaInfo {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub uploader: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub playlist_title: Option<String>,
    #[serde(default)]
    pub playlist_count: Option<usize>,
    #[serde(default)]
    pub playlist_index: Option<usize>,
    #[serde(default)]
    pub playlist_tracks: Vec<String>,
    #[serde(default)]
    pub duration: Option<f64>,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub thumbnails: Vec<ThumbnailInfo>,
    #[serde(default)]
    pub upload_date: Option<String>,
    #[serde(default)]
    pub formats: Vec<FormatInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThumbnailInfo {
    pub id: String,
    pub url: String,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

impl ThumbnailInfo {
    pub fn resolution_str(&self) -> String {
        if let (Some(w), Some(h)) = (self.width, self.height) {
            format!("{}x{}", w, h)
        } else {
            "auto".to_string()
        }
    }

    pub fn ext_str(&self) -> &str {
        if self.url.contains(".webp") {
            "webp"
        } else if self.url.contains(".png") {
            "png"
        } else {
            "jpg"
        }
    }
}

impl MediaInfo {
    /// どんなに変則的な yt-dlp JSON でも絶対に落ちずに安全に構築する寛容パーサー
    pub fn from_value(val: &Value) -> Self {
        let id = val.get("id")
            .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|n| n.to_string())))
            .unwrap_or_default();

        let title = val.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled")
            .to_string();

        let uploader = val.get("uploader")
            .or_else(|| val.get("uploader_id"))
            .or_else(|| val.get("creator"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let channel = val.get("channel")
            .or_else(|| val.get("channel_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let duration = val.get("duration")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)));

        let thumbnail = val.get("thumbnail")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut thumbnails = Vec::new();
        if let Some(thumbs_arr) = val.get("thumbnails").and_then(|v| v.as_array()) {
            for t in thumbs_arr {
                if let Some(url) = t.get("url").and_then(|u| u.as_str()) {
                    let id = t.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string();
                    let w = t.get("width").and_then(|w| w.as_u64().map(|n| n as u32));
                    let h = t.get("height").and_then(|h| h.as_u64().map(|n| n as u32));
                    thumbnails.push(ThumbnailInfo {
                        id,
                        url: url.to_string(),
                        width: w,
                        height: h,
                    });
                }
            }
        }

        let upload_date = val.get("upload_date")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut formats = Vec::new();
        if let Some(fmt_arr) = val.get("formats").and_then(|v| v.as_array()) {
            for f in fmt_arr {
                if let Some(info) = FormatInfo::from_value(f) {
                    formats.push(info);
                }
            }
        }

        let album = val.get("album")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let playlist_title = val.get("playlist_title")
            .or_else(|| val.get("playlist"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let playlist_count = val.get("playlist_count")
            .or_else(|| val.get("n_entries"))
            .and_then(|v| v.as_u64().map(|n| n as usize));

        let playlist_index = val.get("playlist_index")
            .or_else(|| val.get("playlist_autonumber"))
            .and_then(|v| v.as_u64().map(|n| n as usize));

        Self {
            id,
            title,
            uploader,
            channel,
            album,
            playlist_title,
            playlist_count,
            playlist_index,
            playlist_tracks: Vec::new(),
            duration,
            thumbnail,
            thumbnails,
            upload_date,
            formats,
        }
    }

    /// 最適（最高画質）なサムネイルURLを取得
    pub fn get_thumbnail_url(&self) -> Option<String> {
        // 1. thumbnails 配列がある場合、最大解像度（または末尾の高品質サムネイル）を優先
        if let Some(best) = self.thumbnails.iter()
            .filter(|t| !t.url.is_empty())
            .max_by_key(|t| t.width.unwrap_or(0) * t.height.unwrap_or(0))
        {
            if best.width.unwrap_or(0) > 0 {
                return Some(best.url.clone());
            }
        }

        // 2. thumbnails 配列の末尾
        if let Some(last) = self.thumbnails.last() {
            if !last.url.is_empty() {
                return Some(last.url.clone());
            }
        }

        // 3. thumbnail 単体フィールド
        if let Some(ref t) = self.thumbnail {
            if !t.is_empty() {
                return Some(t.clone());
            }
        }

        None
    }

    /// サムネイルURLの全候補リストを優先度順に取得
    pub fn get_thumbnail_candidates(&self) -> Vec<String> {
        let mut list = Vec::new();

        // 1. 最適サムネイル
        if let Some(url) = self.get_thumbnail_url() {
            list.push(url);
        }

        // 2. 単体 thumbnail フィールド
        if let Some(ref t) = self.thumbnail {
            if !list.contains(t) && !t.is_empty() {
                list.push(t.clone());
            }
        }

        // 3. thumbnails 配列（解像度降順）
        let mut sorted_thumbs = self.thumbnails.clone();
        sorted_thumbs.sort_by_key(|t| std::cmp::Reverse(t.width.unwrap_or(0) * t.height.unwrap_or(0)));
        for t in sorted_thumbs {
            if !list.contains(&t.url) && !t.url.is_empty() {
                list.push(t.url);
            }
        }

        list
    }

    /// 重複のないサムネイル候補リスト（解像度降順）を取得
    pub fn get_unique_thumbnails(&self) -> Vec<ThumbnailInfo> {
        let mut list = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let mut sorted = self.thumbnails.clone();
        sorted.sort_by_key(|t| std::cmp::Reverse(t.width.unwrap_or(0) * t.height.unwrap_or(0)));

        for t in sorted {
            if !t.url.is_empty() && seen.insert(t.url.clone()) {
                list.push(t);
            }
        }

        if let Some(ref url) = self.thumbnail {
            if !url.is_empty() && seen.insert(url.clone()) {
                list.push(ThumbnailInfo {
                    id: "default".to_string(),
                    url: url.clone(),
                    width: None,
                    height: None,
                });
            }
        }

        list
    }

    /// 表示用アップローダー名
    pub fn uploader_name(&self) -> &str {
        self.channel.as_deref()
            .or(self.uploader.as_deref())
            .unwrap_or("Unknown")
    }

    /// 表示用再生時間 (HH:MM:SS または MM:SS)
    pub fn duration_formatted(&self) -> String {
        let sec = self.duration.unwrap_or(0.0) as u64;
        let hours = sec / 3600;
        let mins = (sec % 3600) / 60;
        let secs = sec % 60;
        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, mins, secs)
        } else {
            format!("{:02}:{:02}", mins, secs)
        }
    }
}

/// 各ストリーム・フォーマット情報
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FormatInfo {
    #[serde(default)]
    pub format_id: String,
    #[serde(default)]
    pub ext: String,
    #[serde(default)]
    pub resolution: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub fps: Option<f64>,
    #[serde(default)]
    pub vcodec: Option<String>,
    #[serde(default)]
    pub acodec: Option<String>,
    #[serde(default)]
    pub tbr: Option<f64>,
    #[serde(default)]
    pub filesize: Option<f64>,
    #[serde(default)]
    pub filesize_approx: Option<f64>,
    #[serde(default)]
    pub format_note: Option<String>,
}

impl FormatInfo {
    /// Value から安全にフォーマットを抽出
    pub fn from_value(val: &Value) -> Option<Self> {
        let format_id = val.get("format_id")
            .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|n| n.to_string())))
            .unwrap_or_default();

        if format_id.is_empty() {
            return None;
        }

        let ext = val.get("ext").and_then(|v| v.as_str()).unwrap_or("mp4").to_string();
        let resolution = val.get("resolution").and_then(|v| v.as_str()).map(|s| s.to_string());
        let width = val.get("width").and_then(|v| v.as_u64().map(|n| n as u32));
        let height = val.get("height").and_then(|v| v.as_u64().map(|n| n as u32));
        let fps = val.get("fps").and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)));
        let vcodec = val.get("vcodec").and_then(|v| v.as_str()).map(|s| s.to_string());
        let acodec = val.get("acodec").and_then(|v| v.as_str()).map(|s| s.to_string());
        let tbr = val.get("tbr").and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)));
        let filesize = val.get("filesize").and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)));
        let filesize_approx = val.get("filesize_approx").and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)));
        let format_note = val.get("format_note").and_then(|v| v.as_str()).map(|s| s.to_string());

        Some(Self {
            format_id,
            ext,
            resolution,
            width,
            height,
            fps,
            vcodec,
            acodec,
            tbr,
            filesize,
            filesize_approx,
            format_note,
        })
    }

    /// 映像のみのストリームか
    #[allow(dead_code)]
    pub fn is_video_only(&self) -> bool {
        let v = self.vcodec.as_deref().unwrap_or("none");
        let a = self.acodec.as_deref().unwrap_or("none");
        v != "none" && a == "none"
    }

    /// 音声のみのストリームか
    pub fn is_audio_only(&self) -> bool {
        let v = self.vcodec.as_deref().unwrap_or("none");
        let a = self.acodec.as_deref().unwrap_or("none");
        v == "none" && a != "none"
    }

    /// 映像と音声の両方を含むか
    #[allow(dead_code)]
    pub fn is_muxed(&self) -> bool {
        let v = self.vcodec.as_deref().unwrap_or("none");
        let a = self.acodec.as_deref().unwrap_or("none");
        v != "none" && a != "none"
    }

    /// 解像度表示文字列（例: 1920x1080, audio only）
    pub fn resolution_str(&self) -> String {
        if self.is_audio_only() {
            "audio only".to_string()
        } else if let (Some(w), Some(h)) = (self.width, self.height) {
            format!("{}x{}", w, h)
        } else if let Some(ref res) = self.resolution {
            res.clone()
        } else {
            "unknown".to_string()
        }
    }

    /// 容量表示文字列（例: ~45.2MiB）
    pub fn filesize_str(&self) -> String {
        let bytes = self.filesize.or(self.filesize_approx);
        match bytes {
            Some(b) => {
                let mib = b / (1024.0 * 1024.0);
                if mib >= 1024.0 {
                    format!("{:.2}GiB", mib / 1024.0)
                } else {
                    format!("{:.1}MiB", mib)
                }
            }
            None => "--".to_string(),
        }
    }

    /// ビットレート表示（例: 2.5Mbps, 160kbps）
    pub fn bitrate_str(&self) -> String {
        match self.tbr {
            Some(rate) => {
                if rate >= 1000.0 {
                    format!("{:.1}Mbps", rate / 1000.0)
                } else {
                    format!("{:.0}kbps", rate)
                }
            }
            None => "--".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resilient_from_value() {
        let json_str = r#"{
            "id": "dQw4w9WgXcQ",
            "title": "Never Gonna Give You Up",
            "uploader": "Rick Astley",
            "duration": 213,
            "thumbnails": [
                {"id": "0", "url": "https://i.ytimg.com/test1.jpg", "width": 640, "height": 360},
                {"id": "1", "url": "https://i.ytimg.com/test2.jpg", "width": 1920, "height": 1080}
            ],
            "formats": [
                {"format_id": "18", "ext": "mp4", "width": 640, "height": 360, "vcodec": "avc1.42001E"}
            ]
        }"#;
        let val: serde_json::Value = serde_json::from_str(json_str).expect("Valid JSON");
        let info = MediaInfo::from_value(&val);
        assert_eq!(info.id, "dQw4w9WgXcQ");
        assert_eq!(info.title, "Never Gonna Give You Up");
        assert_eq!(info.uploader_name(), "Rick Astley");
        assert!(!info.formats.is_empty());
        assert_eq!(info.get_thumbnail_url(), Some("https://i.ytimg.com/test2.jpg".to_string()));
    }
}
