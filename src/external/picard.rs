use std::path::Path;
use std::process::{Command, Stdio};
use anyhow::{Context, Result};

/// 指定ディレクトリまたはファイルを引数にして MusicBrainz Picard をバックグラウンド起動
pub fn open_in_picard(target_path: &Path) -> Result<()> {
    Command::new("picard")
        .arg(target_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("Failed to spawn picard with path: {:?}", target_path))?;

    Ok(())
}
