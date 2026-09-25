use std::path::PathBuf;
use tokio::sync::mpsc;
use crate::browser::DirBrowserState;
use crate::config::AppConfig;
use crate::preview::ImagePreviewManager;
use crate::ytdlp::download::{AudioFormat, DownloadEvent, DownloadMode, DownloadOptions, MaxResolution, spawn_download_task};
use crate::ytdlp::models::MediaInfo;
use crate::ytdlp::parser::DownloadProgress;
use crate::external::clipboard::get_clipboard_url;
use crate::external::notify::send_notification;
use crate::external::picard::open_in_picard;

/// アプリケーションの画面モード
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppScreenMode {
    Normal,
    FormatSelectModal,
    DirBrowserModal,
    ImageSelectModal,
}

/// 非同期処理からメインループへのメッセージ
pub enum AppMessage {
    MetadataFetched(Result<MediaInfo, String>),
    ThumbnailFetched(Result<image::DynamicImage, String>),
    ImageSaved(Result<PathBuf, String>),
    Download(DownloadEvent),
}

/// メインアプリケーション状態
pub struct AppState {
    pub url_input: String,
    pub is_editing_url: bool,

    pub mode: DownloadMode,
    pub max_resolution: MaxResolution,
    pub audio_format: AudioFormat,

    pub config: AppConfig,
    pub current_bookmark_index: usize,
    pub current_dest_path: PathBuf,
    pub dir_browser: DirBrowserState,

    pub screen_mode: AppScreenMode,

    // メタデータ・プレビュー情報
    pub media_info: Option<MediaInfo>,
    pub is_fetching_meta: bool,
    pub preview_manager: ImagePreviewManager,
    pub spinner_frame: usize,

    // -F フォーマットモーダルの選択状態
    pub format_list_selected: usize,
    pub custom_video_format: Option<String>,
    pub custom_audio_format: Option<String>,

    // 画像選択モーダルの状態
    pub image_list_selected: usize,

    // ダウンロード状態
    pub is_downloading: bool,
    pub progress: DownloadProgress,
    pub status_message: String,
    pub last_download_dir: Option<PathBuf>,
    pub download_task_handle: Option<tokio::task::JoinHandle<()>>,

    // 内部通信
    pub msg_tx: mpsc::UnboundedSender<AppMessage>,
    pub msg_rx: mpsc::UnboundedReceiver<AppMessage>,
}

impl AppState {
    pub fn new() -> Self {
        let config = AppConfig::load_or_create();
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let preview_manager = ImagePreviewManager::new();

        let initial_bookmark_index = 0;
        let initial_dest = config.bookmarks.get(initial_bookmark_index)
            .map(|b| b.path.clone())
            .unwrap_or_else(|| dirs::download_dir().unwrap_or_else(|| PathBuf::from(".")));

        let dir_browser = DirBrowserState::new(initial_dest.clone());

        let mut app = Self {
            url_input: String::new(),
            is_editing_url: false,

            mode: DownloadMode::Video,
            max_resolution: MaxResolution::Unlimited,
            audio_format: AudioFormat::Mp3,

            config,
            current_bookmark_index: initial_bookmark_index,
            current_dest_path: initial_dest,
            dir_browser,

            screen_mode: AppScreenMode::Normal,

            media_info: None,
            is_fetching_meta: false,
            preview_manager,
            spinner_frame: 0,

            format_list_selected: 0,
            custom_video_format: None,
            custom_audio_format: None,

            image_list_selected: 0,

            is_downloading: false,
            progress: DownloadProgress::default(),
            status_message: "Press 'v' to paste URL from clipboard. Press 'p' to open Picard.".to_string(),
            last_download_dir: None,
            download_task_handle: None,

            msg_tx,
            msg_rx,
        };

        // 初期モード連動の保存先設定
        app.sync_dest_for_current_mode();

        app
    }

    /// モードに応じたルートディレクトリと保存先の同期
    pub fn sync_dest_for_current_mode(&mut self) {
        let category = match self.mode {
            DownloadMode::Audio => "audio",
            DownloadMode::Video | DownloadMode::CustomFormats { .. } => "video",
            DownloadMode::ThumbnailOnly => "any",
        };

        if let Some(pos) = self.config.bookmarks.iter().position(|b| b.category == category) {
            self.current_bookmark_index = pos;
            let root = self.config.bookmarks[pos].path.clone();
            self.current_dest_path = root.clone();
            self.dir_browser.set_root(root);
        }
    }

    /// URL をセットし、メタデータとサムネイルの非同期フェッチを開始（ダウンロードは開始しない！）
    pub fn set_url(&mut self, url: String) {
        let trimmed = url.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        self.url_input = trimmed.clone();
        self.is_editing_url = false;
        self.is_fetching_meta = true;
        self.status_message = format!("⏳ Fetching media info for: {}", trimmed);
        self.media_info = None;
        self.preview_manager.clear();
        self.custom_video_format = None;
        self.custom_audio_format = None;

        let tx_meta = self.msg_tx.clone();
        let tx_thumb = self.msg_tx.clone();
        let url_clone = trimmed.clone();

        tokio::spawn(async move {
            match crate::ytdlp::fetch::fetch_media_info(&url_clone).await {
                Ok(info) => {
                    let candidates = info.get_thumbnail_candidates();
                    if !candidates.is_empty() {
                        tokio::spawn(async move {
                            match ImagePreviewManager::fetch_image_with_fallbacks(&candidates).await {
                                Ok(img) => {
                                    let _ = tx_thumb.send(AppMessage::ThumbnailFetched(Ok(img)));
                                }
                                Err(e) => {
                                    let _ = tx_thumb.send(AppMessage::ThumbnailFetched(Err(e.to_string())));
                                }
                            }
                        });
                    }
                    let _ = tx_meta.send(AppMessage::MetadataFetched(Ok(info)));
                }
                Err(e) => {
                    let _ = tx_meta.send(AppMessage::MetadataFetched(Err(e.to_string())));
                }
            }
        });
    }

    /// クリップボードの URL を貼り付け
    pub fn paste_clipboard(&mut self) {
        if let Some(url) = get_clipboard_url() {
            self.set_url(url);
        } else {
            self.status_message = "No valid URL (http:// or https://) found in clipboard".to_string();
        }
    }

    /// モード切り替え (Video -> Audio -> ThumbnailOnly -> Video)
    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            DownloadMode::Video => DownloadMode::Audio,
            DownloadMode::Audio => DownloadMode::ThumbnailOnly,
            DownloadMode::ThumbnailOnly | DownloadMode::CustomFormats { .. } => DownloadMode::Video,
        };
        self.sync_dest_for_current_mode();
    }

    /// 解像度上限切り替え
    pub fn toggle_resolution(&mut self) {
        self.max_resolution = self.max_resolution.next();
    }

    /// 音声フォーマット切り替え
    pub fn toggle_audio_format(&mut self) {
        self.audio_format = self.audio_format.next();
    }

    /// ルートディレクトリの切り替え (1〜9)
    pub fn switch_root_bookmark(&mut self, index: usize) {
        if let Some(bm) = self.config.bookmarks.get(index) {
            self.current_bookmark_index = index;
            let root = bm.path.clone();
            self.current_dest_path = root.clone();
            self.dir_browser.set_root(root);
            self.status_message = format!("Root directory switched to: {}", bm.name);
        }
    }

    /// Picard で開く
    pub fn open_picard(&mut self) {
        let dest = self.last_download_dir.clone().unwrap_or_else(|| self.current_dest_path.clone());
        match open_in_picard(&dest) {
            Ok(_) => {
                self.status_message = format!("Opened MusicBrainz Picard at: {:?}", dest);
            }
            Err(e) => {
                self.status_message = format!("Failed to launch Picard: {}", e);
            }
        }
    }

    /// 選択中のサムネイル画像を保存
    pub fn save_selected_image(&mut self) {
        let Some(ref info) = self.media_info else {
            self.status_message = "⚠️ No media info loaded.".to_string();
            return;
        };

        let thumbs = info.get_unique_thumbnails();
        if thumbs.is_empty() {
            self.status_message = "⚠️ No image candidates available.".to_string();
            return;
        }

        let selected_idx = self.image_list_selected.min(thumbs.len().saturating_sub(1));
        let thumb = &thumbs[selected_idx];
        let url = thumb.url.clone();
        let dest_dir = self.current_dest_path.clone();

        let base_name = if self.mode == DownloadMode::Audio || dest_dir.to_string_lossy().contains("music") {
            "cover".to_string()
        } else {
            format!("{}_thumb", info.title)
        };

        self.status_message = format!("⏳ Downloading image ({}) ...", thumb.resolution_str());
        let tx = self.msg_tx.clone();

        tokio::spawn(async move {
            match ImagePreviewManager::download_image_to_file(&url, &dest_dir, &base_name).await {
                Ok(path) => {
                    let _ = tx.send(AppMessage::ImageSaved(Ok(path)));
                }
                Err(e) => {
                    let _ = tx.send(AppMessage::ImageSaved(Err(e.to_string())));
                }
            }
        });
    }

    /// 最高画質（おまかせ）サムネイル画像を保存
    pub fn save_best_image(&mut self) {
        self.image_list_selected = 0;
        self.save_selected_image();
    }

    /// 明示的なダウンロード開始（メタデータ取得完了後のみ許可！）
    pub fn start_download(&mut self) {
        if self.url_input.trim().is_empty() {
            self.status_message = "⚠️ Please paste or enter a URL first (press 'v').".to_string();
            return;
        }

        if self.is_fetching_meta {
            self.status_message = "⏳ Media information is still loading. Please wait...".to_string();
            return;
        }

        if self.media_info.is_none() {
            self.status_message = "⚠️ Media information not loaded. Please re-enter URL.".to_string();
            return;
        }

        // もし ThumbnailOnly モードなら、yt-dlp の複数回DL・削除ループを回避し最高画質を即座に1枚保存！
        if self.mode == DownloadMode::ThumbnailOnly {
            self.save_best_image();
            return;
        }

        if self.is_downloading {
            self.status_message = "Download already in progress.".to_string();
            return;
        }

        let dest = self.current_dest_path.clone();
        self.last_download_dir = Some(dest.clone());
        self.is_downloading = true;
        self.progress = DownloadProgress::default();
        self.status_message = "🚀 Starting download...".to_string();

        let opts = DownloadOptions {
            url: self.url_input.trim().to_string(),
            dest_dir: dest,
            mode: match &self.mode {
                DownloadMode::CustomFormats { .. } => self.mode.clone(),
                _ if self.custom_video_format.is_some() || self.custom_audio_format.is_some() => {
                    DownloadMode::CustomFormats {
                        video_id: self.custom_video_format.clone(),
                        audio_id: self.custom_audio_format.clone(),
                    }
                }
                _ => self.mode.clone(),
            },
            max_resolution: self.max_resolution,
            audio_format: self.audio_format.clone(),
        };

        let (dl_tx, mut dl_rx) = mpsc::unbounded_channel();
        let app_tx = self.msg_tx.clone();

        tokio::spawn(async move {
            while let Some(event) = dl_rx.recv().await {
                let _ = app_tx.send(AppMessage::Download(event));
            }
        });

        let handle = spawn_download_task(opts, dl_tx);
        self.download_task_handle = Some(handle);
    }

    /// 非同期メッセージのポーリング処理
    pub fn handle_messages(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % 10;

        while let Ok(msg) = self.msg_rx.try_recv() {
            match msg {
                AppMessage::MetadataFetched(Ok(info)) => {
                    self.is_fetching_meta = false;
                    self.status_message = format!("✅ Media Ready: {} | Press [Enter] to Download, [i] for Artwork, [F] for Streams", info.title);
                    self.media_info = Some(info);
                }
                AppMessage::MetadataFetched(Err(err)) => {
                    self.is_fetching_meta = false;
                    self.status_message = format!("❌ Metadata error: {}", err);
                }
                AppMessage::ThumbnailFetched(Ok(img)) => {
                    self.preview_manager.set_image(img, Some(self.url_input.clone()));
                }
                AppMessage::ThumbnailFetched(Err(err)) => {
                    self.status_message = format!("Thumbnail warning: {}", err);
                }
                AppMessage::ImageSaved(Ok(path)) => {
                    self.status_message = format!("🎉 Image saved successfully to: {}", path.display());
                    if self.config.general.notify_on_complete {
                        send_notification("shard: Image Saved", &format!("Successfully saved image to: {}", path.display()));
                    }
                }
                AppMessage::ImageSaved(Err(err)) => {
                    self.status_message = format!("❌ Failed to save image: {}", err);
                }
                AppMessage::Download(DownloadEvent::Started) => {
                    self.is_downloading = true;
                    self.status_message = "⬇️ Downloading...".to_string();
                }
                AppMessage::Download(DownloadEvent::Progress(p)) => {
                    self.progress = p;
                }
                AppMessage::Download(DownloadEvent::StatusMessage(s)) => {
                    self.status_message = s;
                }
                AppMessage::Download(DownloadEvent::Finished { success, output_path }) => {
                    self.is_downloading = false;
                    self.download_task_handle = None;
                    if success {
                        self.progress.percent = 100.0;
                        let path_str = output_path.map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                        self.status_message = format!("🎉 Completed! Saved to: {} (Press 'P' for Picard)", path_str);

                        if self.config.general.notify_on_complete {
                            let title = self.media_info.as_ref().map(|m| m.title.as_str()).unwrap_or("Media");
                            send_notification("shard: Download Complete", &format!("Successfully saved: {}", title));
                        }
                    } else {
                        self.status_message = "❌ Download failed or cancelled.".to_string();
                    }
                }
                AppMessage::Download(DownloadEvent::Error(err)) => {
                    self.status_message = format!("Error: {}", err);
                }
            }
        }
    }
}
