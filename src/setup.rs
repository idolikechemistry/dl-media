use crate::config::Config;
use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use dirs::config_dir;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// 1. 檢查系統環境是否具備必要工具
pub fn check_dependencies() -> Result<()> {
    let deps = [
        ("yt-dlp", "https://github.com/yt-dlp/yt-dlp#installation"),
        ("ffmpeg", "https://ffmpeg.org/download.html"),
        ("ffprobe", "https://ffmpeg.org/download.html"),
        ("danmaku2ass", "https://github.com/m13253/danmaku2ass"),
    ];

    let mut missing = Vec::new();

    for (dep, url) in deps {
        // 同時檢查 --version 與 -h 以確保工具存在
        if Command::new(dep).arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_err()
            && Command::new(dep).arg("-h").stdout(Stdio::null()).stderr(Stdio::null()).status().is_err()
        {
            missing.push((dep, url));
        }
    }

    if !missing.is_empty() {
        let mut error_msg = String::from("❌ 偵測到缺少必要組件，請先安裝以下工具：\n\n");
        for (name, url) in missing {
            error_msg.push_str(&format!("  📌 [{}]\n     👉 下載：{}\n", name, url));
            #[cfg(target_os = "macos")]
            if name != "danmaku2ass" {
                error_msg.push_str(&format!("     💻 Mac 指令：brew install {}\n", name));
            }
        }
        anyhow::bail!(error_msg);
    }
    Ok(())
}

/// 2. 初始化設定環境：建立資料夾與產生預設 config.toml
pub fn init_config() -> Result<(PathBuf, Config)> {
    let mut path = config_dir().context("無法取得系統設定目錄")?;
    path.push("dl-media");
    if !path.exists() {
        fs::create_dir_all(&path).context("無法建立應用程式設定資料夾")?;
    }

    let config_file = path.join("config.toml");
    if !config_file.exists() {
        fs::write(&config_file, Config::default_template()).context("生成預設設定檔失敗")?;
        println!("✨ 已在設定資料夾生成新的 config.toml。");
    }

    let config_data = Config::load(&config_file)?;
    Ok((path, config_data))
}

/// 3. 互動式設定引導 (TUI)：支援拖曳路徑輸入
pub fn interactive_config_setup(config_path: &Path, mut config: Config) -> Result<()> {
    let theme = ColorfulTheme::default();

    loop {
        let options = vec![
            format!("📂 下載目錄 [目前: {}]", config.download_dir.as_deref().unwrap_or("預設 (Downloads)")),
            format!("⏳ 暫存目錄 [目前: {}]", config.tmp_dir.as_deref().unwrap_or("預設 (與下載目錄相同)")),
            format!("🍪 Cookie 目錄 [目前: {}]", config.cookie_dir.as_deref().unwrap_or("預設 (App設定夾)")),
            "✅ 完成並退出".to_string(),
        ];

        let selection = Select::with_theme(&theme)
            .with_prompt("🛠️  dl-media 偏好設定引導 (請使用上下鍵選擇)")
            .items(&options)
            .default(0)
            .interact()?;

        if selection == 3 { break; } // 選擇退出

        println!("\n💡 操作指引：");
        println!("   1. 我現在會為您開啟資料夾視窗。");
        println!("   2. 請在視窗中找到目標資料夾，並將其「拖入」此終端機視窗中。");
        
        // 自動幫使用者開啟設定資料夾作為起點
        let _ = open_folder(&config_path.parent().unwrap().to_path_buf());

        let input_path: String = Input::with_theme(&theme)
            .with_prompt("📍 請拖入路徑並按下 Enter")
            .interact_text()?;

        // 核心邏輯：清理拖曳路徑產生的特殊字元 (例如引號或 Mac 的轉義空白)
        let cleaned_path = input_path.trim()
            .trim_matches('"')
            .trim_matches('\'')
            .replace("\\ ", " "); // 處理 Mac 終端機拖曳產生的轉義空白

        match selection {
            0 => config.download_dir = Some(cleaned_path),
            1 => config.tmp_dir = Some(cleaned_path),
            2 => config.cookie_dir = Some(cleaned_path),
            _ => {}
        }

        // 即時儲存，保證設定不遺失
        config.save(config_path).context("儲存設定失敗")?;
        println!("✨ 設定已更新！\n");
    }

    Ok(())
}

/// 4. 輔助函式：開啟系統檔案總管
pub fn open_folder(path: &PathBuf) -> Result<()> {
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(path).status();
    #[cfg(target_os = "windows")]
    let _ = Command::new("explorer").arg(path).status();
    #[cfg(target_os = "linux")]
    let _ = Command::new("xdg-open").arg(path).status();
    Ok(())
}

/// 5. 處理 Cookie 載入邏輯
pub fn handle_cookies(
    site_target: &str,
    has_restricted: bool,
    manual_cookie: &Option<String>,
    resolved_cookie_dir: &PathBuf,
    is_silent: bool,
) -> Result<Vec<String>> {
    let mut cookie_args = Vec::new();

    // 優先權 1：命令列 -c 指定
    if let Some(manual_path_str) = manual_cookie {
        let path = PathBuf::from(manual_path_str);
        if path.exists() {
            cookie_args.push("--cookies".to_string());
            cookie_args.push(path.to_string_lossy().to_string());
            println!("🍪 已套用自訂 Cookie：{}", path.display());
            return Ok(cookie_args);
        }
    }

    // 優先權 2：設定路徑下的 cookie_site.txt
    let expected_filename = format!("cookie_{}.txt", site_target);
    let target_file = resolved_cookie_dir.join(&expected_filename);

    if target_file.exists() {
        cookie_args.push("--cookies".to_string());
        cookie_args.push(target_file.to_string_lossy().to_string());
        println!("🍪 已載入 {} 專用 Cookie", site_target);
    } else if has_restricted {
        println!("⚠️ 未偵測到 {} 專用 Cookie ({})", site_target, expected_filename);

        let want_to_wait = if is_silent { false } else {
            Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("此內容需要權限。是否要現在開啟 Cookie 目錄放入？")
                .default(true)
                .interact()?
        };

        if want_to_wait {
            open_folder(resolved_cookie_dir)?;
            println!("⏳ 請將 {} 放入資料夾，完成後按下 Enter 繼續...", expected_filename);
            let mut _pause = String::new();
            io::stdin().read_line(&mut _pause)?;

            if target_file.exists() {
                println!("🎉 偵測到 Cookie！已成功套用。");
                cookie_args.push("--cookies".to_string());
                cookie_args.push(target_file.to_string_lossy().to_string());
            }
        }
    }

    Ok(cookie_args)
}