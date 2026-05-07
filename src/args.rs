use clap::{Parser, ValueEnum};

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum MediaType {
    /// 🎧 純音訊下載
    #[value(alias = "1")]
    Audio = 1,
    /// 🔕 無聲影片下載
    #[value(alias = "2")]
    VideoOnly = 2,
    /// 🎥 有聲影片下載
    #[value(alias = "3")]
    Video = 3,
}

#[derive(Parser, Debug)]
// 🎯 核心修改：移除硬編碼，改為自動讀取 Cargo.toml 的 version 和 description
#[command(version, about, long_about = None)]
pub struct Args {
    /// 貼上要下載的影片或播放清單網址
    #[arg(short, long)]
    pub url: Option<String>,

    /// 指定下載類型 (支援 1, 2, 3 或對應名稱)
    #[arg(short, long, value_enum)]
    pub media_type: Option<MediaType>,

    /// 指定輸出格式 (mp3, m4a, mp4, mkv)
    #[arg(short, long)]
    pub format: Option<String>,

    /// 指定輸出路徑 (預設為系統 Downloads)
    #[arg(short, long)]
    pub output: Option<String>,

    /// 手動指定 Cookie 檔案路徑
    #[arg(short, long)]
    pub cookie: Option<String>,

    /// 打開應用程式設定資料夾
    #[arg(long)]
    pub open_config: bool,

    /// 強制調用儲存好的 Cookie
    #[arg(long = "fc", alias = "force-cookie")] 
    pub force_cookie: bool,
}

impl Args {
    /// 檢查是否有提供網址、類型與格式 (用於判斷是否進入自動化模式)
    pub fn is_fully_automated(&self) -> bool {
        self.url.is_some() && self.media_type.is_some() && self.format.is_some()
    }

    /// 驗證參數邏輯是否合法
    pub fn validate(&self) -> anyhow::Result<()> {
        if let (Some(mt), Some(fmt)) = (self.media_type, &self.format) {
            let fmt = fmt.to_lowercase();
            match mt {
                MediaType::Audio => {
                    if fmt != "mp3" && fmt != "m4a" {
                        anyhow::bail!("❌ 格式 '{}' 與音訊類型不匹配。請使用 mp3 或 m4a。", fmt);
                    }
                }
                _ => {
                    if fmt != "mp4" && fmt != "mkv" {
                        anyhow::bail!("❌ 格式 '{}' 與影片類型不匹配。請使用 mp4 或 mkv。", fmt);
                    }
                }
            }
        }
        Ok(())
    }
}