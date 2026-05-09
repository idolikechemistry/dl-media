use crate::args::MediaType;
use dialoguer::{theme::ColorfulTheme, Select, Input};
use anyhow::Result;

/// 互動式取得使用者輸入
pub fn get_user_input(args: &crate::args::Args) -> Result<(String, u8, String)> {
    let theme = ColorfulTheme::default();

    // 1. 取得網址
    let url = match &args.url {
        Some(u) => u.clone(),
        None => {
            Input::<String>::with_theme(&theme)
                .with_prompt("🔗 請貼上影片或播放清單網址")
                .interact_text()?
        }
    };

    // 2. 取得下載類型 (修正：確保 match 兩邊都回傳 MediaType Enum)
    let media_type_enum = match args.media_type {
        Some(t) => t,
        None => {
            let types = vec!["🎧 音訊", "🔕 無聲影片", "🎥 有聲影片"];
            let selection = Select::with_theme(&theme)
                .with_prompt("🎬 請選擇下載類型")
                .items(&types)
                .default(2)
                .interact()?;
            
            match selection {
                0 => MediaType::Audio,
                1 => MediaType::VideoOnly,
                _ => MediaType::Video,
            }
        }
    };

    // 3. 取得格式
    let format = match &args.format {
        Some(f) => f.clone(),
        None => {
            let formats = match media_type_enum {
                MediaType::Audio => vec!["mp3", "m4a"],
                _ => vec!["mp4", "mkv"],
            };
            let selection = Select::with_theme(&theme)
                .with_prompt("📦 請選擇輸出格式")
                .items(&formats)
                .default(0)
                .interact()?;
            formats[selection].to_string()
        }
    };

    // 回傳時將 Enum 轉為 u8 給底層邏輯使用
    Ok((url, media_type_enum as u8, format))
}

/// 下載完成後的總結報告 (修正：補上 utils.rs 呼叫所需的函式)
pub fn print_summary(success: usize, fail: usize, duration: &str, path: &str) {
    println!("\n=================================================");
    println!("✅ 任務完成！");
    println!("⏱️  總耗時：{}", duration);
    println!("📊 統計：成功 {} / 失敗 {}", success, fail);
    println!("📂 存檔路徑：{}", path);
    println!("=================================================\n");
}

use inquire::{MultiSelect, ui::{RenderConfig, Styled}};

/// 提供使用者選擇要下載的語言 (僅限音訊模式)
pub fn select_subtitles(available_langs: &[String]) -> Vec<String> {
    let mut options = Vec::new();
    
    // 將複雜的語言代碼映射為直覺的選項
    if available_langs.iter().any(|l| l.contains("zh") || l.contains("chi")) {
        options.push("中文 (繁/簡/彈幕)");
    }
    if available_langs.iter().any(|l| l.starts_with("en")) {
        options.push("英文 (English)");
    }
    if available_langs.iter().any(|l| l.starts_with("ja") || l.starts_with("jpn")) {
        options.push("日文 (日本語)");
    }

    // 如果該影片完全沒有字幕，直接回傳空陣列跳過詢問
    if options.is_empty() {
        return vec![];
    }

    // 🎨 核心修改：自訂終端機 UI 外觀
    // 將選中的方塊改成直覺的打勾符號，未選中的保持空白方塊
    let render_config = RenderConfig::default()
        .with_selected_checkbox(Styled::new("[✓]"))
        .with_unselected_checkbox(Styled::new("[  ]"));

    let ans = MultiSelect::new("✨ 偵測到可用歌詞/字幕，請選擇要保留的語言 (Space 勾選 / Enter 確認)：", options)
        .with_render_config(render_config) // 套用我們自訂的外觀
        .prompt()
        .unwrap_or_default();

    let mut selected_langs = Vec::new();
    for a in ans {
        match a {
            "中文 (繁/簡/彈幕)" => selected_langs.extend(vec!["zh-Hant".into(), "zh-TW".into(), "zh-HK".into(), "zh-Hans".into(), "zh".into(), "chi".into(), "danmaku".into()]),
            "英文 (English)" => selected_langs.extend(vec!["en".into(), "en-US".into(), "en-GB".into()]),
            "日文 (日本語)" => selected_langs.extend(vec!["ja".into(), "jpn".into()]),
            _ => {}
        }
    }
    selected_langs
}