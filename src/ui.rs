use crate::args::MediaType;
use crate::parser::VideoFormat;
use anyhow::Result;
use dialoguer::{Input, Select, theme::ColorfulTheme};
use inquire::{
    MultiSelect, Select as InquireSelect,
    ui::{RenderConfig, Styled},
};

/// 互動式取得使用者輸入
/// 🎯 核心修改：回傳型別改為 Vec<String> 以支援多連結批量輸入
pub fn get_user_input(args: &crate::args::Args) -> Result<(Vec<String>, u8, String)> {
    let theme = ColorfulTheme::default();

    // 1. 取得網址 (支援多個，以空格隔開)
    let urls = match &args.url {
        Some(u) => u.clone(),
        None => {
            let input = Input::<String>::with_theme(&theme)
                .with_prompt("🔗 請貼上影片或播放清單網址 (多個網址請用空格隔開)")
                .interact_text()?;
            // 🎯 清洗並切割字串成陣列
            input.split_whitespace().map(|s| s.to_string()).collect()
        }
    };

    // 2. 取得下載類型
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
                MediaType::Audio => vec!["m4a", "mp3"],
                _ => vec!["mp4 (最高 1080p，相容性佳)", "mkv (解鎖 4K/8K 畫質)"],
            };
            let selection = Select::with_theme(&theme)
                .with_prompt("📦 請選擇輸出格式")
                .items(&formats)
                .default(0)
                .interact()?;

            // 🎯 清洗字串：只取第一個空格前的部分 (例如 "mp4" 或 "mkv")
            formats[selection]
                .split_whitespace()
                .next()
                .unwrap()
                .to_string()
        }
    };

    // 回傳時將 Enum 轉為 u8 給底層邏輯使用
    Ok((urls, media_type_enum as u8, format))
}

/// 下載完成後的總結報告
pub fn print_summary(success: usize, fail: usize, duration: &str, path: &str) {
    println!("=================================================");
    println!("✅ 任務完成！");
    println!("⏱️  總耗時：{}", duration);
    println!("📊 統計：成功 {} / 失敗 {}", success, fail);
    println!("📂 存檔路徑：{}", path);
    println!("=================================================");
}

// 🎯 畫質選擇選單
pub fn select_resolution(formats: &[VideoFormat]) -> Option<String> {
    let mut options_raw: Vec<&VideoFormat> = formats.iter().filter(|f| f.height > 1080).collect();

    if options_raw.is_empty() {
        return None;
    }

    if let Some(fhd) = formats
        .iter()
        .filter(|f| f.height <= 1080)
        .max_by_key(|f| f.height)
    {
        options_raw.push(fhd);
    }

    options_raw.sort_by(|a, b| b.height.cmp(&a.height));
    options_raw.dedup_by(|a, b| a.height == b.height);

    let display_options: Vec<String> = options_raw
        .iter()
        .map(|f| format!("{}p (編碼: {}), 來源：{})", f.height, f.vcodec, f.ext))
        .collect();

    let ans = InquireSelect::new(
        "✨ 偵測到高畫質選項，請選擇下載解析度：",
        display_options.clone(),
    )
    .prompt()
    .ok()?;

    let idx = display_options.iter().position(|x| x == &ans)?;
    Some(options_raw[idx].format_id.clone())
}

/// 提供使用者選擇要下載的語言 (僅限音訊模式)
pub fn select_subtitles(available_langs: &[String]) -> Vec<String> {
    let mut options = Vec::new();

    if available_langs
        .iter()
        .any(|l| l.contains("zh") || l.contains("chi"))
    {
        options.push("中文 (繁/簡/彈幕)");
    }
    if available_langs.iter().any(|l| l.starts_with("en")) {
        options.push("英文 (English)");
    }
    if available_langs
        .iter()
        .any(|l| l.starts_with("ja") || l.starts_with("jpn"))
    {
        options.push("日文 (日本語)");
    }

    if options.is_empty() {
        return vec![];
    }

    let render_config = RenderConfig::default()
        .with_selected_checkbox(Styled::new("[✓]"))
        .with_unselected_checkbox(Styled::new("[  ]"));

    let ans = MultiSelect::new(
        "✨ 偵測到可用歌詞/字幕，請選擇要保留的語言 (Space 勾選 / Enter 確認)：",
        options,
    )
    .with_render_config(render_config)
    .prompt()
    .unwrap_or_default();

    let mut selected_langs = Vec::new();
    for a in ans {
        match a {
            "中文 (繁/簡/彈幕)" => selected_langs.extend(vec![
                "zh-Hant".into(),
                "zh-TW".into(),
                "zh-HK".into(),
                "zh-Hans".into(),
                "zh".into(),
                "chi".into(),
                "danmaku".into(),
            ]),
            "英文 (English)" => {
                selected_langs.extend(vec!["en".into(), "en-US".into(), "en-GB".into()])
            }
            "日文 (日本語)" => selected_langs.extend(vec!["ja".into(), "jpn".into()]),
            _ => {}
        }
    }
    selected_langs
}
