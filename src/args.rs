use clap::Parser;
use std::process;

#[derive(Parser, Debug)]
#[command(name = "dl-media", about = "全能影音分析與下載器 (v0.3.0)")]
pub struct Args {
    #[arg(short, long, help = "貼上要下載的影片或播放清單網址")]
    pub url: Option<String>,
    
    #[arg(short, long, help = "指定下載類型 (音訊 → 1, 無聲影片 → 2, 有聲影片 → 3)")]
    pub media_type: Option<u8>,
    
    #[arg(short, long, help = "指定輸出格式 (音訊格式 → mp3 or m4a，影片格式 → mp4 or mkv)")]
    pub format: Option<String>,

    #[arg(short, long, help = "指定輸出資料夾路徑 (預設為系統的 Downloads)")]
    pub output: Option<String>,
    
    #[arg(short, long, help = "手動指定 Cookie 檔案路徑")]
    pub cookie: Option<String>,
    
    #[arg(long, help = "打開應用程式設定資料夾 (用來放入 Cookie)")]
    pub open_config: bool,
    
    #[arg(long = "fc", help = "強制調用 config 內已經儲存好的 Cookie")]
    pub force_cookie: bool,
}

impl Args {
    /// 早期嚴格參數防呆與匹配驗證
    pub fn validate(&self) {
        // 1. 檢查下載類型是否在允許範圍內
        if let Some(m) = self.media_type {
            if m < 1 || m > 3 {
                eprintln!("❌ 錯誤：不支援的下載類型 '{}'。", m);
                eprintln!("💡 提示：-m 參數請輸入 1, 2 或 3。");
                eprintln!("   1 → 音訊");
                eprintln!("   2 → 無聲影片");
                eprintln!("   3 → 有聲影片");
                process::exit(1);
            }
        }

        // 2. 檢查輸出格式的有效性以及與下載類型的匹配
        if let Some(ref fmt) = self.format {
            let valid_formats = ["mp4", "mkv", "mp3", "m4a"];
            let f_lower = fmt.to_lowercase();
            
            if !valid_formats.contains(&f_lower.as_str()) {
                eprintln!("❌ 錯誤：不支援的輸出格式 '{}'。", fmt);
                eprintln!("💡 提示：-f 參數請輸入正確的副檔名。");
                eprintln!("   音訊格式 → mp3, m4a");
                eprintln!("   影片格式 → mp4, mkv");
                process::exit(1);
            }

            if let Some(m) = self.media_type {
                if m == 1 && f_lower != "mp3" && f_lower != "m4a" {
                    eprintln!("❌ 錯誤：格式 '{}' 無法與類型 (音訊 -m 1) 匹配。", fmt);
                    eprintln!("💡 提示：下載音訊時，-f 只能設定為 mp3 或 m4a。");
                    process::exit(1);
                } else if (m == 2 || m == 3) && f_lower != "mp4" && f_lower != "mkv" {
                    eprintln!("❌ 錯誤：格式 '{}' 無法與類型 (影片 -m {}) 匹配。", fmt, m);
                    eprintln!("💡 提示：下載影片時，-f 只能設定為 mp4 或 mkv。");
                    process::exit(1);
                }
            }
        }
    }
}