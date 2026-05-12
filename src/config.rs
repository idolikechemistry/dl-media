use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

// 🎯 確保版本號與 Cargo.toml 同步
fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// 🎯 為所有欄位提供明確的預設字串，防止被 serde 隱藏
fn default_empty_string() -> String {
    "".into()
}
fn default_concurrency() -> u32 {
    3
}
fn default_video_fmt() -> String {
    "mp4".into()
}
fn default_audio_fmt() -> String {
    "m4a".into()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "current_version")]
    pub version: String,

    #[serde(default = "default_empty_string")]
    pub download_dir: String,

    #[serde(default = "default_empty_string")]
    pub cookie_dir: String,

    #[serde(default = "default_video_fmt")]
    pub default_video_format: String,

    #[serde(default = "default_audio_fmt")]
    pub default_audio_format: String,

    #[serde(default = "default_concurrency")]
    pub max_concurrent_downloads: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: current_version(),
            download_dir: "".into(),
            cookie_dir: "".into(),
            default_video_format: "mp4".into(),
            default_audio_format: "m4a".into(),
            max_concurrent_downloads: 3,
        }
    }
}

impl Config {
    /// 從指定路徑載入設定檔，並自動處理版本結構同步
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            // 🎯 採納你的建議：直接實例化 Default，並呼叫 save() 統一寫入邏輯
            let default_config = Config::default();
            default_config.save(path)?;
            println!("✨ 初次執行：已為您生成帶有註解的設定檔 (config.toml)。");
            return Ok(default_config);
        }

        let content = fs::read_to_string(path)?;
        let mut config: Config = toml::from_str(&content)
            .context("解析設定檔失敗，若格式毀損請刪除設定檔讓程式重新生成")?;

        let app_ver = current_version();
        if config.version != app_ver {
            println!(
                "🔄 偵測到版本更新 ({} -> {})，正在同步設定檔結構...",
                config.version, app_ver
            );
            config.version = app_ver;

            // 升級時一樣呼叫統一的 save()，烙印手冊並寫入新欄位
            config.save(path)?;
            println!("✨ 設定檔結構已自動補齊，並保留您的自訂內容。");
        }

        Ok(config)
    }

    /// 🎯 統一的 save 方法：負責將詳細的使用手冊「烙印」在設定檔頂部，並寫入硬碟
    pub fn save(&self, path: &Path) -> Result<()> {
        let data = toml::to_string_pretty(self).context("序列化設定資料失敗")?;

        // 📝 統一的手動註解區
        let manual = r#"# ======================================================
# dl-media 使用者設定檔 (自動同步更新版)
# ======================================================
# 💡 提示：本檔案在版本更新時會自動重構結構。
#
# 📍 download_dir:
#    預設下載目錄。留空則使用系統「下載」資料夾。
#   - Mac:     ~/Library/Application Support/dl-media/
#   - Linux:   ~/.config/dl-media/
#   - Windows: %APPDATA%\dl-media\
#    範例: "/Users/username/Movies"
#
# 🍪 cookie_dir:
#    存放 cookie_youtube.txt 等檔案的目錄。
#    留空則使用本程式的設定資料夾。
#
# 🎬 default_video_format / default_audio_format:
#    預設封裝格式。影片可選: mp4, mkv / 音訊可選: mp3, m4a
#
# ⚡ max_concurrent_downloads:
#    最大並行下載數。建議範圍 1-5，設太高可能導致 IP 被封鎖。
#
# ⚠️ version: 版本追蹤標籤，請勿手動修改。
# ======================================================

"#;

        // 將手冊與機器生成的資料拼接
        let final_content = format!("{}{}", manual, data);

        fs::write(path, final_content).with_context(|| format!("無法寫入設定檔至: {:?}", path))?;
        Ok(())
    }
}
