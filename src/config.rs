use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::{Result, Context};

/// dl-media 的核心設定結構體
/// 透過 Serialize 與 Deserialize 達成設定檔的讀寫
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    /// 預設下載目錄 (例如: /Users/jay/Downloads)
    pub download_dir: Option<String>,
    
    /// 暫存檔存放目錄 (建議設定在讀寫快速或空間充足的硬碟)
    pub tmp_dir: Option<String>,
    
    /// Cookie 檔案的統一存放路徑
    pub cookie_dir: Option<String>,
    
    /// 預設影片格式 (mp4, mkv)
    pub default_video_format: Option<String>,
    
    /// 預設音訊格式 (mp3, m4a)
    pub default_audio_format: Option<String>,
}

impl Config {
    /// 產生預設的設定檔範本字串 (包含詳盡的繁體中文註解)
    pub fn default_template() -> &'static str {
        r#"# ======================================================
# dl-media 使用者設定檔 (TOML 格式)
# ======================================================

# 📍 預設下載目錄
# 如果留空，程式會自動使用系統預設的「下載」資料夾。
# 範例 (Mac): download_dir = "/Users/您的用戶名/Movies/YouTube"
download_dir = ""

# ⏳ 暫存檔存放目錄
# 程式在合併影片、轉檔彈幕時會使用此目錄。
# 如果下載大型清單，建議設定在空間充足的硬碟。
# 如果留空，則會直接在下載目錄進行處理。
tmp_dir = ""

# 🍪 Cookie 檔案存放目錄
# 程式會在此目錄尋找 cookie_youtube.txt, cookie_bilibili.txt 等檔案。
# 如果留空，預設會使用程式設定資料夾 (~/.config/dl-media/)。
cookie_dir = ""

# 🎬 預設影片格式 (可選: mp4, mkv)
default_video_format = "mkv"

# 🎵 預設音訊格式 (可選: mp3, m4a)
default_audio_format = "m4a"
"#
    }

    /// 從指定路徑載入設定檔
    pub fn load(path: &Path) -> Result<Self> {
        // 如果檔案不存在，回傳一個帶有基本預設值的物件
        if !path.exists() {
            return Ok(Config {
                default_video_format: Some("mkv".into()),
                default_audio_format: Some("m4a".into()),
                ..Default::default()
            });
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("無法讀取設定檔: {:?}", path))?;
        
        let config: Config = toml::from_str(&content)
            .context("解析設定檔失敗，請檢查 Config.toml 的語法格式是否正確")?;
            
        Ok(config)
    }

    /// 將目前的記憶體設定寫回檔案 (實現 TUI 自動儲存)
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("序列化設定資料失敗")?;
            
        fs::write(path, content)
            .with_context(|| format!("無法寫入設定檔至: {:?}", path))?;
            
        Ok(())
    }
}