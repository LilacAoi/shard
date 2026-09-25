use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use crate::ytdlp::parser::{parse_progress_line, DownloadProgress};

/// ダウンロードモード
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadMode {
    Video,
    Audio,
    ThumbnailOnly,
    CustomFormats {
        video_id: Option<String>,
        audio_id: Option<String>,
    },
}

/// 動画解像度の上限
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaxResolution {
    Unlimited,
    Res2160p,
    Res1440p,
    Res1080p,
    Res720p,
    Res480p,
}

impl MaxResolution {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Unlimited => "Unlimited (Best)",
            Self::Res2160p => "2160p (4K)",
            Self::Res1440p => "1440p (2K)",
            Self::Res1080p => "1080p",
            Self::Res720p => "720p",
            Self::Res480p => "480p",
        }
    }

    pub fn height_limit(&self) -> Option<u32> {
        match self {
            Self::Unlimited => None,
            Self::Res2160p => Some(2160),
            Self::Res1440p => Some(1440),
            Self::Res1080p => Some(1080),
            Self::Res720p => Some(720),
            Self::Res480p => Some(480),
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Unlimited => Self::Res1080p,
            Self::Res1080p => Self::Res720p,
            Self::Res720p => Self::Res480p,
            Self::Res480p => Self::Res2160p,
            Self::Res2160p => Self::Res1440p,
            Self::Res1440p => Self::Unlimited,
        }
    }
}

/// 音声フォーマット設定
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioFormat {
    Mp3,
    Flac,
    Opus,
    M4a,
}

impl AudioFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Flac => "flac",
            Self::Opus => "opus",
            Self::M4a => "m4a",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Mp3 => Self::Flac,
            Self::Flac => Self::Opus,
            Self::Opus => Self::M4a,
            Self::M4a => Self::Mp3,
        }
    }
}

/// ダウンロードジョブのパラメータ
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub url: String,
    pub dest_dir: PathBuf,
    pub mode: DownloadMode,
    pub max_resolution: MaxResolution,
    pub audio_format: AudioFormat,
}

/// ダウンロードイベント
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Started,
    Progress(DownloadProgress),
    StatusMessage(String),
    Finished { success: bool, output_path: Option<PathBuf> },
    Error(String),
}

/// yt-dlp コマンド引数の組み立て
pub fn build_ytdlp_args(opts: &DownloadOptions) -> Vec<String> {
    let mut args = Vec::new();

    // 保存先ディレクトリ
    args.push("-P".to_string());
    args.push(opts.dest_dir.to_string_lossy().to_string());

    // パース用プログレス設定
    args.push("--newline".to_string());
    args.push("--progress-template".to_string());
    args.push("%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s|%(progress._total_bytes_estimate_str)s".to_string());

    match &opts.mode {
        DownloadMode::Video => {
            // 解像度上限フィルタ
            let format_str = match opts.max_resolution.height_limit() {
                Some(h) => format!("bv*[height<={}]+ba/b", h),
                None => "bv*+ba/b".to_string(),
            };
            args.push("-f".to_string());
            args.push(format_str);
            args.push("--no-write-thumbnail".to_string());
            args.push("-o".to_string());
            args.push("%(title)s [%(id)s].%(ext)s".to_string());
        }
        DownloadMode::Audio => {
            args.push("-x".to_string());
            args.push("--audio-format".to_string());
            args.push(opts.audio_format.as_str().to_string());
            args.push("--audio-quality".to_string());
            args.push("0".to_string());
            args.push("--no-write-thumbnail".to_string());
            args.push("--add-metadata".to_string());
            args.push("-o".to_string());
            args.push("%(artist,uploader)s - %(title)s [%(id)s].%(ext)s".to_string());
        }
        DownloadMode::ThumbnailOnly => {
            args.push("--write-thumbnail".to_string());
            args.push("--skip-download".to_string());
            args.push("--convert-thumbnails".to_string());
            args.push("png".to_string());
            args.push("-o".to_string());
            args.push("%(title)s [%(id)s]_thumbnail.%(ext)s".to_string());
        }
        DownloadMode::CustomFormats { video_id, audio_id } => {
            let format_selector = match (video_id, audio_id) {
                (Some(v), Some(a)) => format!("{}+{}", v, a),
                (Some(v), None) => v.clone(),
                (None, Some(a)) => a.clone(),
                (None, None) => "best".to_string(),
            };
            args.push("-f".to_string());
            args.push(format_selector);
            args.push("-o".to_string());
            args.push("%(title)s [%(id)s].%(ext)s".to_string());
        }
    }

    // 対象URL
    args.push(opts.url.clone());
    args
}

/// 非同期ダウンロードタスクの起動
pub fn spawn_download_task(
    opts: DownloadOptions,
    tx: mpsc::UnboundedSender<DownloadEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let args = build_ytdlp_args(&opts);
        let _ = tx.send(DownloadEvent::Started);
        let _ = tx.send(DownloadEvent::StatusMessage(format!("Executing: yt-dlp {}", args.join(" "))));

        let mut cmd = tokio::process::Command::new("yt-dlp");
        cmd.args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(DownloadEvent::Error(format!("Failed to start yt-dlp: {}", e)));
                let _ = tx.send(DownloadEvent::Finished { success: false, output_path: None });
                return;
            }
        };

        let stdout = child.stdout.take().expect("Failed to open stdout");
        let stderr = child.stderr.take().expect("Failed to open stderr");

        let tx_out = tx.clone();
        let stdout_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if let Some(prog) = parse_progress_line(&line) {
                    let _ = tx_out.send(DownloadEvent::Progress(prog));
                } else if !line.trim().is_empty() {
                    let _ = tx_out.send(DownloadEvent::StatusMessage(line));
                }
            }
        });

        let tx_err = tx.clone();
        let stderr_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if !line.trim().is_empty() {
                    let _ = tx_err.send(DownloadEvent::StatusMessage(format!("yt-dlp: {}", line)));
                }
            }
        });

        let status = child.wait().await;
        let _ = stdout_handle.await;
        let _ = stderr_handle.await;

        match status {
            Ok(s) if s.success() => {
                let _ = tx.send(DownloadEvent::Finished {
                    success: true,
                    output_path: Some(opts.dest_dir),
                });
            }
            Ok(s) => {
                let _ = tx.send(DownloadEvent::Error(format!("yt-dlp exited with status {}", s)));
                let _ = tx.send(DownloadEvent::Finished { success: false, output_path: None });
            }
            Err(e) => {
                let _ = tx.send(DownloadEvent::Error(format!("Error waiting for yt-dlp: {}", e)));
                let _ = tx.send(DownloadEvent::Finished { success: false, output_path: None });
            }
        }
    })
}
