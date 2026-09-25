use std::path::PathBuf;
use std::fs;
use anyhow::{Context, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntryItem {
    pub name: String,
    pub path: PathBuf,
    pub is_parent: bool,
}

#[derive(Debug, Clone)]
pub struct DirBrowserState {
    pub root_path: PathBuf,
    pub current_path: PathBuf,
    pub entries: Vec<DirEntryItem>,
    pub selected_index: usize,

    // 新規ディレクトリ作成プロンプト用
    pub is_creating_dir: bool,
    pub new_dir_input: String,
    pub error_message: Option<String>,
}

impl DirBrowserState {
    pub fn new(root_path: PathBuf) -> Self {
        let current_path = if root_path.exists() {
            root_path.clone()
        } else {
            // パスが存在しない場合はローカルホームまたは親を作成/使用
            dirs::download_dir().unwrap_or_else(|| PathBuf::from("."))
        };

        let mut browser = Self {
            root_path,
            current_path,
            entries: Vec::new(),
            selected_index: 0,
            is_creating_dir: false,
            new_dir_input: String::new(),
            error_message: None,
        };

        browser.reload_entries();
        browser
    }

    /// ルートパスを変更してリセット
    pub fn set_root(&mut self, root: PathBuf) {
        self.root_path = root.clone();
        self.current_path = root;
        self.selected_index = 0;
        self.is_creating_dir = false;
        self.new_dir_input.clear();
        self.error_message = None;
        self.reload_entries();
    }

    /// 現在のディレクトリ内のサブフォルダ一覧を再読み込み
    pub fn reload_entries(&mut self) {
        self.entries.clear();
        self.error_message = None;

        // ルートより深い場合は ".."（親ディレクトリへ戻る）を追加
        if self.current_path != self.root_path {
            if let Some(parent) = self.current_path.parent() {
                self.entries.push(DirEntryItem {
                    name: ".. (Parent Directory)".to_string(),
                    path: parent.to_path_buf(),
                    is_parent: true,
                });
            }
        }

        // サブディレクトリの走査
        if let Ok(read_dir) = fs::read_dir(&self.current_path) {
            let mut dirs = Vec::new();
            for entry in read_dir.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        // 隠しフォルダは除外
                        if !name.starts_with('.') {
                            dirs.push(DirEntryItem {
                                name,
                                path: entry.path(),
                                is_parent: false,
                            });
                        }
                    }
                }
            }
            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            self.entries.extend(dirs);
        } else {
            self.error_message = Some(format!("Directory not accessible: {:?}", self.current_path));
        }

        if self.selected_index >= self.entries.len() && !self.entries.is_empty() {
            self.selected_index = self.entries.len() - 1;
        }
    }

    /// 選択中のサブディレクトリに入る (yaziの 'l' / Enter)
    pub fn enter_selected(&mut self) -> bool {
        if let Some(entry) = self.entries.get(self.selected_index).cloned() {
            self.current_path = entry.path;
            self.selected_index = 0;
            self.reload_entries();
            true
        } else {
            false
        }
    }

    /// 親ディレクトリへ移動 (yaziの 'h')
    pub fn go_parent(&mut self) -> bool {
        if self.current_path != self.root_path {
            if let Some(parent) = self.current_path.parent().map(|p| p.to_path_buf()) {
                self.current_path = parent;
                self.selected_index = 0;
                self.reload_entries();
                return true;
            }
        }
        false
    }

    /// 新規フォルダ作成モードに入る
    pub fn start_create_dir(&mut self) {
        self.is_creating_dir = true;
        self.new_dir_input.clear();
        self.error_message = None;
    }

    /// 新規フォルダ作成の確定
    pub fn confirm_create_dir(&mut self) -> Result<PathBuf> {
        let name = self.new_dir_input.trim();
        if name.is_empty() {
            anyhow::bail!("Folder name cannot be empty");
        }

        let new_path = self.current_path.join(name);
        fs::create_dir_all(&new_path)
            .with_context(|| format!("Failed to create directory at: {:?}", new_path))?;

        self.is_creating_dir = false;
        self.new_dir_input.clear();
        self.current_path = new_path.clone();
        self.reload_entries();

        Ok(new_path)
    }

    /// 新規フォルダ作成のキャンセル
    pub fn cancel_create_dir(&mut self) {
        self.is_creating_dir = false;
        self.new_dir_input.clear();
        self.error_message = None;
    }
}
