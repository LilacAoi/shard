use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};
use directories::ProjectDirs;

/// アプリケーション全体の設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub audio: AudioSettingsConfig,
    #[serde(default)]
    pub bookmarks: Vec<Bookmark>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_mode")]
    pub default_mode: String,
    #[serde(default = "default_max_resolution")]
    pub max_resolution: String,
    #[serde(default = "default_false")]
    pub auto_paste_clipboard: bool,
    #[serde(default = "default_true")]
    pub notify_on_complete: bool,
}

fn default_mode() -> String { "video".to_string() }
fn default_max_resolution() -> String { "unlimited".to_string() }
fn default_true() -> bool { true }
fn default_false() -> bool { false }

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_mode: default_mode(),
            max_resolution: default_max_resolution(),
            auto_paste_clipboard: false,
            notify_on_complete: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettingsConfig {
    #[serde(default = "default_audio_format")]
    pub format: String,
    #[serde(default = "default_audio_quality")]
    pub quality: String,
    #[serde(default = "default_false")]
    pub embed_thumbnail: bool,
    #[serde(default = "default_true")]
    pub add_metadata: bool,
}

fn default_audio_format() -> String { "mp3".to_string() }
fn default_audio_quality() -> String { "0".to_string() }

impl Default for AudioSettingsConfig {
    fn default() -> Self {
        Self {
            format: default_audio_format(),
            quality: default_audio_quality(),
            embed_thumbnail: true,
            add_metadata: true,
        }
    }
}

/// 保存先ルートディレクトリの登録情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: usize,
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub category: String, // "video", "audio", "thumbnail", "any"
}

impl Default for AppConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

        let music_dir = dirs::audio_dir().unwrap_or_else(|| home.join("Music"));
        let video_dir = dirs::video_dir().unwrap_or_else(|| home.join("Videos"));
        let download_dir = dirs::download_dir().unwrap_or_else(|| home.join("Downloads"));

        let bookmarks = vec![
            Bookmark {
                id: 1,
                name: "Music".to_string(),
                path: music_dir,
                category: "audio".to_string(),
            },
            Bookmark {
                id: 2,
                name: "Videos".to_string(),
                path: video_dir,
                category: "video".to_string(),
            },
            Bookmark {
                id: 3,
                name: "Downloads".to_string(),
                path: download_dir,
                category: "any".to_string(),
            },
        ];

        Self {
            general: GeneralConfig::default(),
            audio: AudioSettingsConfig::default(),
            bookmarks,
        }
    }
}

impl AppConfig {
    /// 設定ファイルの格納パス (`~/.config/shard/config.toml`)
    pub fn config_path() -> PathBuf {
        if let Some(proj) = ProjectDirs::from("com", "antigravity", "shard") {
            proj.config_dir().join("config.toml")
        } else {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            home.join(".config/shard/config.toml")
        }
    }

    /// キャッシュディレクトリの格納パス (`~/.cache/shard`)
    pub fn cache_dir() -> PathBuf {
        if let Some(proj) = ProjectDirs::from("com", "antigravity", "shard") {
            proj.cache_dir().to_path_buf()
        } else {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            home.join(".cache/shard")
        }
    }

    /// 設定の読み込み（ファイルが存在しない場合はデフォルトを作成・保存）
    pub fn load_or_create() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str::<AppConfig>(&content) {
                    return config;
                }
            }
        }

        // デフォルト作成と保存
        let default_config = Self::default();
        let _ = default_config.save();
        default_config
    }

    /// 設定のファイル保存
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config dir: {:?}", parent))?;
        }

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config to TOML")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {:?}", path))?;

        Ok(())
    }

    /// モードに応じた推奨保存先ブックマークの取得
    #[allow(dead_code)]
    pub fn default_bookmark_for_mode(&self, mode_category: &str) -> Option<&Bookmark> {
        self.bookmarks.iter()
            .find(|b| b.category == mode_category)
            .or_else(|| self.bookmarks.first())
    }
}
