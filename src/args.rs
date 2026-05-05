use clap::{Parser, ValueEnum};

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum MediaType {
    /// 🎧 純音訊下載
    Audio = 1,
    /// 🔕 無聲影片下載
    VideoOnly = 2,
    /// 🎥 有聲影片下載
    Video = 3,
}

// 實作自定義解析器，讓 -m 支援 1, 2, 3 或文字
fn parse_media_type(s: &str) -> Result<MediaType, String> {
    match s {
        "1" | "audio" => Ok(MediaType::Audio),
        "2" | "video-only" => Ok(MediaType::VideoOnly),
        "3" | "video" => Ok(MediaType::Video),
        _ => Err(format!("無效的下載類型 '{}'。請使用 1, 2, 3 或對應名稱。", s)),
    }
}

#[derive(Parser, Debug)]
#[command(name = "dl-media", version = "0.2.2", about = "影音下載器")]
pub struct Args {
    /// 貼上要下載的影片或播放清單網址
    #[arg(short, long)]
    pub url: Option<String>,

    /// 指定下載類型 (1: 音訊, 2: 無聲影片, 3: 有聲影片)
    #[arg(short, long, value_parser = parse_media_type)]
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
    // 顯式指定為 "fc"，這樣它在終端機就會變回 --fc
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