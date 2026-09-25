use image::DynamicImage;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use anyhow::{Context, Result};

pub struct ImagePreviewManager {
    picker: Picker,
    current_image: Option<DynamicImage>,
    current_protocol: Option<StatefulProtocol>,
    last_url: Option<String>,
}

impl ImagePreviewManager {
    pub fn new() -> Self {
        let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
        Self {
            picker,
            current_image: None,
            current_protocol: None,
            last_url: None,
        }
    }

    /// URLから画像データを非同期取得してデコード
    pub async fn fetch_image_from_url(url: &str) -> Result<DynamicImage> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()?;

        let bytes = client.get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let img = image::load_from_memory(&bytes)
            .context("Failed to decode image from bytes")?;

        Ok(img)
    }

    /// 複数のURL候補から順に試行して画像を取得
    pub async fn fetch_image_with_fallbacks(candidates: &[String]) -> Result<DynamicImage> {
        let mut last_err = anyhow::anyhow!("No thumbnail candidates provided");
        for url in candidates {
            match Self::fetch_image_from_url(url).await {
                Ok(img) => return Ok(img),
                Err(e) => last_err = e,
            }
        }
        Err(last_err)
    }

    /// URL から画像を直接ダウンロードしてローカルファイルに保存
    pub async fn download_image_to_file(
        url: &str,
        dest_dir: &std::path::Path,
        suggested_name: &str,
    ) -> Result<std::path::PathBuf> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        let resp = client.get(url)
            .send()
            .await?
            .error_for_status()?;

        let ext = if url.contains(".webp") {
            "webp"
        } else if url.contains(".png") {
            "png"
        } else {
            "jpg"
        };

        let safe_name: String = suggested_name
            .chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                _ => c,
            })
            .collect();
        let safe_name = if safe_name.trim().is_empty() { "thumbnail".to_string() } else { safe_name.trim().to_string() };

        let file_name = format!("{}.{}", safe_name, ext);
        let dest_path = dest_dir.join(&file_name);

        let bytes = resp.bytes().await?;
        tokio::fs::create_dir_all(dest_dir).await?;
        tokio::fs::write(&dest_path, &bytes).await?;

        Ok(dest_path)
    }

    /// 新しい画像をセットし、StatefulProtocol を生成
    pub fn set_image(&mut self, img: DynamicImage, url: Option<String>) {
        let proto = self.picker.new_resize_protocol(img.clone());
        self.current_image = Some(img);
        self.current_protocol = Some(proto);
        self.last_url = url;
    }

    /// 現在の StatefulProtocol のミュータブル参照を取得
    pub fn protocol_mut(&mut self) -> Option<&mut StatefulProtocol> {
        self.current_protocol.as_mut()
    }

    /// プロトコルタイプ名（Kitty / Sixel / Halfblocks 等）
    pub fn protocol_name(&self) -> &'static str {
        match self.picker.protocol_type() {
            ratatui_image::picker::ProtocolType::Kitty => "Kitty Protocol (Native GPU)",
            ratatui_image::picker::ProtocolType::Sixel => "Sixel",
            ratatui_image::picker::ProtocolType::Iterm2 => "iTerm2",
            ratatui_image::picker::ProtocolType::Halfblocks => "Unicode Half-Blocks",
        }
    }

    /// プレビューのクリア
    pub fn clear(&mut self) {
        self.current_image = None;
        self.current_protocol = None;
        self.last_url = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn test_fetch_and_decode() {
        let url = "https://i.ytimg.com/vi_webp/dQw4w9WgXcQ/maxresdefault.webp";
        let res = ImagePreviewManager::fetch_image_from_url(url).await;
        match res {
            Ok(img) => {
                println!("Successfully decoded image: {}x{}", img.width(), img.height());
            }
            Err(e) => {
                panic!("Failed to decode image: {:?}", e);
            }
        }
    }
}
