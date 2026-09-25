use regex::Regex;
use std::sync::LazyLock;

/// ダウンロード進捗状況
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DownloadProgress {
    pub percent: f64,
    pub speed: String,
    pub eta: String,
    pub total_size: String,
    pub raw_status: String,
}

static PROGRESS_PIPE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // テンプレート出力: " 45.2%| 12.3MiB/s|00:15|120.5MiB"
    Regex::new(r"^\s*([\d.]+)%\s*\|\s*([^|]*)\|\s*([^|]*)\|\s*(.*)$").unwrap()
});

static FALLBACK_PERCENT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // 標準出力フォールバック: "[download]  45.2% of ~120.5MiB at 12.3MiB/s ETA 00:15"
    Regex::new(r"\[download\]\s+([\d.]+)%").unwrap()
});

static FALLBACK_SPEED_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"at\s+([~0-9.]+[kKMGT]?i?B/s)").unwrap()
});

static FALLBACK_ETA_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"ETA\s+([\d:]+)").unwrap()
});

/// yt-dlp の標準出力行を解析して進捗を更新
pub fn parse_progress_line(line: &str) -> Option<DownloadProgress> {
    let trimmed = line.trim();

    // 1. パイプ区切りのカスタムテンプレートを優先解析
    if let Some(caps) = PROGRESS_PIPE_REGEX.captures(trimmed) {
        let percent = caps.get(1).and_then(|m| m.as_str().parse::<f64>().ok()).unwrap_or(0.0);
        let speed = caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
        let eta = caps.get(3).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
        let total = caps.get(4).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

        return Some(DownloadProgress {
            percent,
            speed: if speed.is_empty() || speed == "NA" { "--".to_string() } else { speed },
            eta: if eta.is_empty() || eta == "NA" { "--".to_string() } else { eta },
            total_size: if total.is_empty() || total == "NA" { "--".to_string() } else { total },
            raw_status: trimmed.to_string(),
        });
    }

    // 2. 標準 [download] 行のフォールバック解析
    if let Some(caps) = FALLBACK_PERCENT_REGEX.captures(trimmed) {
        let percent = caps.get(1).and_then(|m| m.as_str().parse::<f64>().ok()).unwrap_or(0.0);
        let speed = FALLBACK_SPEED_REGEX.captures(trimmed)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "--".to_string());
        let eta = FALLBACK_ETA_REGEX.captures(trimmed)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "--".to_string());

        return Some(DownloadProgress {
            percent,
            speed,
            eta,
            total_size: "--".to_string(),
            raw_status: trimmed.to_string(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_template_pipe() {
        let line = " 75.4%| 8.4MiB/s|00:12|140.2MiB";
        let res = parse_progress_line(line).expect("Should parse pipe progress");
        assert!((res.percent - 75.4).abs() < f64::EPSILON);
        assert_eq!(res.speed, "8.4MiB/s");
        assert_eq!(res.eta, "00:12");
        assert_eq!(res.total_size, "140.2MiB");
    }

    #[test]
    fn test_parse_standard_ytdlp_line() {
        let line = "[download]  35.0% of ~50.0MiB at 10.5MiB/s ETA 00:03";
        let res = parse_progress_line(line).expect("Should parse standard progress");
        assert!((res.percent - 35.0).abs() < f64::EPSILON);
        assert_eq!(res.speed, "10.5MiB/s");
        assert_eq!(res.eta, "00:03");
    }
}
