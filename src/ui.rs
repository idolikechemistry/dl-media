use crate::args::Args;
use dialoguer::{theme::ColorfulTheme, Input, Select};

pub fn get_user_input(args: &Args) -> (String, u8, String) {
    let input_url = args.url.clone().unwrap_or_else(|| {
        // 自動讀取 Cargo.toml 的版本號
        println!("dl-media v{} 🚀", env!("CARGO_PKG_VERSION")); 
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("🔗 請貼上影片或播放清單網址")
            .interact_text().unwrap()
    });

    let media_type = args.media_type.unwrap_or_else(|| {
        let types = vec!["🎧 音訊", "🔕 無聲影片", "🎥 有聲影片"];
        (Select::with_theme(&ColorfulTheme::default()).with_prompt("🎯 下載類型").default(0).items(&types).interact().unwrap() + 1) as u8
    });

    let target_ext = args.format.clone().unwrap_or_else(|| {
        if media_type == 1 {
            let formats = vec!["M4A (原生無損)", "MP3 (320k)"];
            if Select::with_theme(&ColorfulTheme::default()).with_prompt("🎵 音訊格式").items(&formats).interact().unwrap() == 1 { "mp3".into() } else { "m4a".into() }
        } else {
            let formats = vec!["MP4 (高相容)", "MKV (最高畫質)"];
            if Select::with_theme(&ColorfulTheme::default()).with_prompt("🎞️ 影片格式").items(&formats).interact().unwrap() == 1 { "mkv".into() } else { "mp4".into() }
        }
    });

    (input_url, media_type, target_ext.to_lowercase())
}

pub fn print_summary(success: usize, fail: usize, time_str: &str, dir: &str) {
    println!("---");
    println!("🎉 任務全部完成！儲存位置：{}", dir);
    println!("⏱️ 總耗時：{}", time_str);
    
    if success > 0 { 
        println!("✨ {} successful", success); 
    }
    if fail > 0 { 
        println!("⚠️ {} failed", fail); 
    }
}