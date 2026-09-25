# 🦀 shard - yt-dlp TUI ダウンローダー 設計方針書 (Design Document)

## 1. 概要とコンセプト

`shard` は、外付けHDDやローカルストレージを活用し、動画・音声を爆速かつ快適に取得・整理するための **Rust製・高機能 TUI (Terminal User Interface) yt-dlp フロントエンド** です。

- **名前の由来**: 「かけら」「破片（Shard）」。ストリーミングメディアから高品質な欠片（データ）を安全・快適にすくい取るイメージ。
- **ターゲット環境**:
  - OS: NixOS (Xfce / Niri 共存環境)
  - 端末: Kitty (Kitty Graphics Protocol による高画質画像プレビュー対応)
  - ストレージ: 外付けHDD (1TB, LUKS暗号化バックアップ併用, APM 254) / 内蔵ストレージ
  - 外部連携: MusicBrainz Picard (メタデータ自動付与・楽曲管理)

---

## 2. コア機能要件

| No | 要件項目 | 仕様詳細 |
| :--- | :--- | :--- |
| 1 | **音声 / 動画モード切替** | ・**動画モード**: 通常の動画ダウンロード（解像度・コンテナ指定）<br>・**音声モード**: 音声のみ抽出（MP3, FLAC, OPUS, AAC 等、ビットレート指定、サムネイル埋め込み） |
| 2 | **解像度上限設定** | ・**デフォルト: 無制限 (Best Video + Best Audio)**<br>・プリセット: `無制限 (Best)`, `2160p (4K)`, `1440p (2K)`, `1080p`, `720p`, `480p`, `360p`<br>・内部セレクタ: `bv*[height<=1080]+ba/b` 等の柔軟なフィルタ生成 |
| 3 | **`-F` 詳細フォーマット選択** | ・`yt-dlp -F` (内部的には `yt-dlp -J` の JSON) からストリーム一覧を高速パース<br>・Ratatui のインタラクティブテーブルで表示（ID, 拡張子, 解像度, FPS, VCODEC, ACODEC, ビットレート, サイズ）<br>・映像と音声を個別に選択して結合ダウンロード可能 |
| 4 | **Picard 連携ボタン** | ・音声ダウンロード完了後、またはキー一発（`P` キー）で **MusicBrainz Picard** を非同期起動<br>・ダウンロード先ディレクトリまたは生成された音声ファイルパスを引数として渡し、Picard 上で即座に対象フォルダを開いてタグ編集・アルバム照会へ直結 |
| 5 | **画像取得 ＆ プレビュー** | ・**Kitty Graphics Protocol** (クレート `ratatui-image`) を使用し、ターミナル内にサムネイルを高精細インライン表示<br>・動画情報の事前フェッチ時にサムネイルを取得・キャッシュ<br>・「サムネイルのみ保存」「アートワーク抽出」もモードまたはオプションで対応 |
| 6 | **クリップボード自動取得** | ・起動時、または `p` キー押下時に Wayland (`wl-paste`) / X11 (`arboard`) から URL を自動検出 |
| 7 | **進捗表示 ＆ 通知** | ・Ratatui のゲージウィジェットによるダウンロード進捗（%、速度、ETA、総容量）のリアルタイム描画<br>・ダウンロード完了・エラー時にデスクトップ通知（`notify-rust` / `notify-send`）を発行 |
| 8 | **保存先ブックマーク登録** | ・複数のダウンロード先ディレクトリ（外付けHDD Videos / Music、ローカル Downloads 等）を事前登録・管理<br>・`d` キーでポップアップ一覧から即時切り替え、または動画/音声モード切り替え時にデフォルト先へ自動連動 |

---

## 3. UI / UX デザイン設計

### 3.1 メイン画面イメージ (通常モード)

```text
┌── shard - yt-dlp TUI Downloader ────────────────────────────────────────────────────────┐
│ URL: [ https://www.youtube.com/watch?v=xxxxxxxxxxx                                    ] │
│                                                                                         │
│ ┌─ Preview ──────────────────────┐ ┌─ Media Information ──────────────────────────────┐ │
│ │                                │ │ Title:    Sample Video Title                     │ │
│ │                                │ │ Channel:  Official Music Channel                 │ │
│ │     [ Kitty Graphics Protocol  │ │ Duration: 03:45                                  │ │
│ │       Thumbnail Image Preview ]│ │ Uploaded: 2026-09-01                             │ │
│ │                                │ │ Best Res: 3840x2160 (4K 60fps)                   │ │
│ └────────────────────────────────┘ └──────────────────────────────────────────────────┘ │
│                                                                                         │
│ Mode:     [ ● Video ]    (   Audio )    (   Thumbnail Only )                            │
│ Max Res:  [ ● Unlimited ] ( 1080p ) ( 720p ) ( 480p ) ( Custom... )                     │
│ Audio:    [ MP3 (320k) ] ( FLAC )   ( OPUS )                                            │
│ Target:   [1] 📂 External HDD Videos (/mnt/storage/Videos/)                             │
│                                                                                         │
│ Progress: [██████████████████████████░░░░░░░░░░] 72.4% (12.4 MiB/s)  ETA: 00:18         │
│ Status:   [download] Destination: Sample Video Title [xxxx].mp4                         │
│                                                                                         │
│ [Enter] Start  [Tab] Mode  [r] Res  [F] Formats  [d] Dest  [P] Picard  [p] Paste  [q]   │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 `-F` フォーマット選択モーダル (詳細選択)

```text
┌── Select Streams (yt-dlp -F) ───────────────────────────────────────────────────────────┐
│ Filter: [ ] (Press / to search)               [x] Merge Best Audio Automatically       │
│                                                                                         │
│ ID     EXT   RESOLUTION   FPS   VCODEC          ACODEC         BITRATE     SIZE         │
│ ─────────────────────────────────────────────────────────────────────────────────────── │
│ 313    webm  3840x2160    60    vp09.00.51...   video only     18.2Mbps    ~450MiB      │
│ 271    webm  2560x1440    60    vp09.00.50...   video only     9.1Mbps     ~220MiB      │
│ 137    mp4   1920x1080    60    avc1.64002a     video only     4.3Mbps     ~110MiB   <  │
│ 248    webm  1920x1080    60    vp09.00.41...   video only     3.1Mbps     ~85MiB       │
│ 140    m4a   audio only   --    audio only      mp4a.40.2      128kbps     ~3.5MiB   *  │
│ 251    webm  audio only   --    audio only      opus           160kbps     ~4.1MiB      │
│                                                                                         │
│ [Space] Select/Deselect Video (137)   [a] Select Audio (140)   [Enter] Confirm   [Esc] Cancel│
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

### 3.3 保存先ディレクトリ選択・ブックマークモーダル (`d` キー)

```text
┌── Select Download Destination ──────────────────────────────────────────────────────────┐
│  > [1] 📂 External HDD: Videos     /mnt/storage/Videos                                  │
│    [2] 🎵 External HDD: Music      /mnt/storage/Music                                   │
│    [3] 📥 Local: Downloads         /home/user/Downloads                                 │
│    [4] 🖼️ Local: Pictures (Thumbs) /home/user/Pictures/Wallpapers                       │
│    [+] ➕ Add Current/Custom Path...                                                     │
│                                                                                         │
│  [1-9] Quick Select   [j/k] Navigate   [Enter] Set Destination   [Esc] Close            │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. ターミナル画像プレビューの技術方針

### 4.1 Kitty Graphics Protocol の活用
- ユーザー環境は **Kitty (0.48.2)** であり、ターミナル画像プロトコルの最高峰である **Kitty Graphics Protocol** に完全対応。
- Rust クレート `ratatui-image` を使用：
  - 端末ケーパビリティ（Kitty / Sixel / iTerm2 / Half-Block Unicode）を自動検出。
  - Kitty 環境では高速・ゼロ劣化のネイティブ描画（GPUテクスチャ転送）。
  - Kitty 以外の環境（Xfce Terminal や標準端末）で起動された場合でも、自動的に半角ブロック文字（Halfblocks）や Sixel にフォールバックしてクラッシュを防ぐ。

### 4.2 サムネイル取得・非同期パイプライン
1. URL が入力/ペーストされたら、非同期タスク (`tokio::spawn`) を発行。
2. `yt-dlp --dump-json --no-playlist <URL>` または `yt-dlp --print thumbnail <URL>` を実行してメタデータとサムネイルURLを取得。
3. `reqwest` でサムネイル画像バイナリを非同期取得し、メモリ上で `image` クレートでデコード。
4. メイン描画ループにチャネル（`mpsc`）経由で画像データを送信し、プレビューエリアにレンダリング。

---

## 5. yt-dlp コマンドパラメータ設計

### 5.1 動画ダウンロード
- **無制限 (Best quality / デフォルト)**:
  ```bash
  yt-dlp -f "bv*+ba/b" \
         -P "/path/to/destination" \
         -o "%(title)s [%(id)s].%(ext)s" \
         --newline \
         --progress-template "%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s|%(progress._total_bytes_estimate_str)s" \
         "<URL>"
  ```
- **解像度上限指定 (例: 1080p 以下で最高品質)**:
  ```bash
  yt-dlp -f "bv*[height<=1080]+ba/b" \
         -P "/path/to/destination" \
         -o "%(title)s [%(id)s].%(ext)s" \
         --newline \
         --progress-template "%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s|%(progress._total_bytes_estimate_str)s" \
         "<URL>"
  ```

### 5.2 音声ダウンロード (Picard 連携用)
- **MP3 / FLAC 抽出 + サムネイル埋め込み + メタデータ付与**:
  ```bash
  yt-dlp -x \
         --audio-format mp3 \
         --audio-quality 0 \
         --embed-thumbnail \
         --add-metadata \
         -P "/path/to/music/destination" \
         -o "%(artist,uploader)s - %(title)s [%(id)s].%(ext)s" \
         --newline \
         --progress-template "%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s|%(progress._total_bytes_estimate_str)s" \
         "<URL>"
  ```

### 5.3 `-F` 詳細フォーマット取得
- **JSON パース方式**:
  ```bash
  yt-dlp -J --no-playlist "<URL>"
  ```
  JSON 出力の `formats` 配列から以下のフィールドを抽出してテーブル化：
  - `format_id`: フォーマットID（例: `137`, `251`）
  - `ext`: 拡張子（`mp4`, `webm`, `m4a`）
  - `resolution`: 解像度（`1920x1080`, `audio only`）
  - `fps`: フレームレート
  - `vcodec` / `acodec`: コーデック名
  - `tbr` / `vbr` / `abr`: ビットレート
  - `filesize` / `filesize_approx`: 推定容量

---

## 6. Picard 連携メカニズム

MusicBrainz Picard はコマンドライン引数としてディレクトリパスまたはファイルパスを受け取ることが可能：
```bash
picard "/mnt/storage/Music/DownloadedFolder"
```

### Rust 実装設計:
```rust
use std::process::Command;

pub fn launch_picard(target_path: &Path) -> std::io::Result<()> {
    Command::new("picard")
        .arg(target_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    Ok(())
}
```
- **UX**:
  - TUI 画面下部に `[P] Open Picard` ボタン・ショートカットを配置。
  - 音声ダウンロード完了時にダイアログ/トーストで「Picard で開きますか？ [Y/n]」を出すことも可能。
  - 現在のダウンロード保存先フォルダ、またはダウンロードした音声ファイルを直接指定してバックグラウンド起動するため、TUI を終了せずにシームレスにタグ付け作業へ移行できる。

---

## 7. アーキテクチャと内部状態モデル

```mermaid
flowchart TD
    subgraph UI ["TUI (Ratatui + Crossterm)"]
        EventLoop["Main Event Loop (250ms tick / Key Events)"]
        State["App State (Mode, Formats, Progress, URL, Preview)"]
        Renderer["Render Pipeline (UI Widgets + ratatui-image)"]
    end

    subgraph Backend ["Async Workers (Tokio Tasks)"]
        MetaWorker["Metadata & Thumbnail Fetcher (yt-dlp -J + reqwest)"]
        DlWorker["Download Worker (yt-dlp child process)"]
        ClipboardWorker["Clipboard Watcher / Reader"]
    end

    subgraph System ["External OS / Tools"]
        Ytdlp["yt-dlp CLI"]
        Picard["MusicBrainz Picard"]
        Notify["Desktop Notifications (notify-rust)"]
        Disk["Storage Drive (/mnt/storage/...)"]
    end

    EventLoop -->|Key / Input| State
    State --> Renderer
    Renderer --> UI

    MetaWorker -->|Parsed Formats / Decoded Image| State
    DlWorker -->|Progress Lines (%)| State
    DlWorker -->|Finish Event| Notify
    
    State -->|Launch Picard (P key)| Picard
    MetaWorker --> Ytdlp
    DlWorker --> Ytdlp
    Ytdlp --> Disk
    Picard --> Disk
```

### 状態マシン (State Machine)
- `AppMode`:
  - `Input`: URL入力・待機中
  - `FetchingMeta`: メタデータ・サムネイル・フォーマット取得中（ローディングスピナー）
  - `Ready`: 取得完了、モード（動画/音声）・画質上限選択可能
  - `FormatSelect`: `-F` フォーマット一覧選択モーダル表示中
  - `DestinationSelect`: 保存先ディレクトリブックマーク選択モーダル表示中 (`d` キー)
  - `Downloading`: ダウンロード中（プログレスバー更新、中断可能）
  - `Completed`: ダウンロード完了（Picard起動ボタン強調、ファイル一覧表示）

---

## 8. 設定ファイルとブックマーク管理仕様 (`config.toml`)

設定ファイルは標準の XDG ディレクトリ（`~/.config/shard/config.toml`）に自動保存・読み込みされます。
初回起動時にデフォルト設定が自動生成され、TUI 上からもキー操作で追加・切り替えが可能です。

```toml
# ~/.config/shard/config.toml

[general]
default_mode = "video"       # "video" | "audio" | "thumbnail"
max_resolution = "unlimited" # "unlimited" | "2160p" | "1440p" | "1080p" | "720p" | "480p"
auto_paste_clipboard = false
notify_on_complete = true

[audio]
format = "mp3"               # "mp3" | "flac" | "opus" | "m4a"
quality = "0"                # 0 = 最高音質 (VBR / 320k相当)
embed_thumbnail = false
add_metadata = true

# 保存先ディレクトリのブックマーク一覧
[[bookmarks]]
id = 1
name = "Music"
path = "/path/to/Music"
category = "audio"           # 音声モード時のデフォルト候補 (Picard連携と親和)

[[bookmarks]]
id = 2
name = "Videos"
path = "/path/to/Videos"
category = "video"           # 動画モード時のデフォルト候補

[[bookmarks]]
id = 3
name = "Downloads"
path = "/path/to/Downloads"
category = "any"
```

- **自動切り替えロジック**:
  - `Tab` キーで「動画モード」にした時は、直前またはカテゴリ `video` のブックマークがデフォルト選択。
  - 「音声モード」にした時は、カテゴリ `audio` のブックマーク（外付けHDD Music）が自動選択され、ダウンロード後に即座に Picard で開ける状態になる。
  - 任意のタイミングで `d` キーを押せば、数字キー（1〜9）または上下キーで保存先を即時変更可能。

---

## 9. 推奨クレート構成 (Cargo.toml 案)

```toml
[package]
name = "shard"
version = "0.1.0"
edition = "2021"
authors = ["LilacAoi"]
description = "A fast, ergonomic yt-dlp TUI downloader built with Rust and Ratatui"

[dependencies]
# TUI & Terminal
ratatui = "0.29"
crossterm = { version = "0.28", features = ["event-stream"] }
ratatui-image = { version = "4.4", features = ["kitty"] }

# Async runtime & Process
tokio = { version = "1.40", features = ["full"] }
tokio-stream = "0.1"

# Serialization & CLI parsing
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
clap = { version = "4.5", features = ["derive"] }

# Image processing (Thumbnail preview)
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "webp"] }
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }

# System integrations
arboard = "3.4"         # Clipboard (Wayland / X11)
notify-rust = "4.11"     # Desktop notifications
directories = "5.0"     # Standard XDG paths

# Utility
regex = "1.10"
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
```

---

## 10. 開発ロードマップ

1. **フェーズ 1: コア実行基盤と Nix 環境の構築**
   - `flake.nix` + `.envrc` による `cargo` / `rustc` / `rust-analyzer` / `pkg-config` / `openssl` の開発シェル整備。
   - `yt-dlp` サブプロセス実行とプログレス行の正規表現パース関数の実装・単体テスト。
2. **フェーズ 2: TUI 基本レイアウトとモード切り替え**
   - Ratatui によるメイン画面の描画（URL 入力、動画/音声モード選択、解像度上限セレクタ、進捗バー）。
   - 保存先ディレクトリのブックマーク切り替え（`d` キーモーダル）と設定ファイル読み書き。
3. **フェーズ 3: `-F` 詳細フォーマット選択と JSON 解析**
   - `yt-dlp -J` からフォーマット情報を取得し、テーブルウィジェットで対話的ストリーム選択を実装。
4. **フェーズ 4: サムネイル取得と Kitty 画像プレビュー**
   - `ratatui-image` を統合し、メタデータ取得時にサムネイル画像をインラインプレビュー。
5. **フェーズ 5: Picard 連携 & デスクトップ通知**
   - ダウンロード完了後の通知発行と、`P` キーによる Picard のターゲットフォルダ指定起動。
   - 外付けHDDへの自動保存パス（`Videos`, `Music`）のプリセット永続化。

---

## 11. NixOS 開発環境・ビルド依存ノート (arboard 等)

`arboard` (クリップボード連携) は Linux (X11 / Wayland) において以下の C ライブラリまたはシステムパッケージを必要とします：

- **ビルド・リンク時 (`flake.nix` の `buildInputs` または `road2nixos`)**:
  - `xorg.libX11`
  - `xorg.libXcursor`
  - `xorg.libXrandr`
  - `xorg.libXi`
  - `wayland`
- **ランタイムツール**:
  - `wl-clipboard` (`wl-paste` / `wl-copy`) ※Wayland (Niri) 環境でのフォールバック・確実な動作に有用
  - `xclip` または `xsel` (X11 環境)

本プロジェクト専用の `flake.nix` にこれらの C ライブラリ依存をあらかじめ含めておくことで、システム側全体に余計な開発用ライブラリを入れずとも `nix develop` (direnv) 内でクリーンにビルド・動作させることが可能です。
