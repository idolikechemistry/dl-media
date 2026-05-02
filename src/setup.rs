use dialoguer::{theme::ColorfulTheme, Confirm};
use dirs::config_dir;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// 檢查系統環境是否具備 yt-dlp, ffmpeg, ffprobe
pub fn check_dependencies() {
    let deps = [
        ("yt-dlp", "https://github.com/yt-dlp/yt-dlp#installation"),
        ("ffmpeg", "https://ffmpeg.org/download.html"),
        ("ffprobe", "https://ffmpeg.org/download.html"),
    ];
    
    let mut missing = Vec::new();

    for (dep, url) in deps {
        if Command::new(dep).arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_err() {
            missing.push((dep, url));
        }
    }

    if !missing.is_empty() {
        println!("❌ 偵測到缺少必要組件，請先安裝以下工具以繼續執行：\n");
        for (name, url) in missing {
            println!("  📌 [{}]", name);
            println!("     👉 手動下載：{}", url);
            #[cfg(target_os = "macos")]
            println!("     💻 Mac 安裝指令：brew install {}", name);
        }
        println!("\n完成安裝後請重啟程式！");
        std::process::exit(1);
    }
}

/// 取得並確保應用程式的專屬設定資料夾存在
pub fn get_app_config_dir() -> PathBuf {
    let mut path = config_dir().expect("無法取得系統設定目錄");
    path.push("dl-media");
    if !path.exists() {
        fs::create_dir_all(&path).expect("無法建立應用程式設定資料夾");
    }
    path
}

/// 開啟設定資料夾 (供使用者放入 Cookie)
pub fn open_config_folder() {
    let path = get_app_config_dir();
    println!("📂 正在開啟設定資料夾：{}", path.display());
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(&path).status();
    #[cfg(target_os = "windows")]
    let _ = Command::new("explorer").arg(&path).status();
    #[cfg(target_os = "linux")]
    let _ = Command::new("xdg-open").arg(&path).status();
}

/// 負責尋找 Cookie、提示使用者放入 Cookie，並產生 yt-dlp 需要的參數
pub fn handle_cookies(
    site_target: &str,
    has_restricted: bool,
    manual_cookie: &Option<String>,
    is_silent: bool,
) -> Vec<String> {
    let mut cookie_args = Vec::new();

    // 1. 優先檢查使用者是否手動指定了 Cookie 檔案 (-c 參數)
    if let Some(manual_path_str) = manual_cookie {
        let path = PathBuf::from(manual_path_str);
        if path.exists() {
            cookie_args.push("--cookies".to_string());
            cookie_args.push(path.to_string_lossy().to_string());
            println!("🍪 已載入自訂 Cookie 檔案：{}", path.display());
            return cookie_args;
        } else {
            println!("⚠️ 找不到指定的 Cookie 檔案：{}", path.display());
        }
    }

    // 2. 檢查自動設定資料夾內的專屬 Cookie
    let config_path = get_app_config_dir();
    let expected_filename = format!("cookie_{}.txt", site_target);
    let target_file = config_path.join(&expected_filename);
    
    if target_file.exists() {
        cookie_args.push("--cookies".to_string());
        cookie_args.push(target_file.to_string_lossy().to_string());
        println!("🍪 已從 config 載入 {} 專用 Cookie：{}", site_target, expected_filename);
    } else if has_restricted {
        // 3. 如果找不到，但又偵測到權限受限，觸發互動引導
        println!("⚠️ config 內未偵測到 {} 專用 Cookie ({})", site_target, expected_filename);
        
        let want_to_wait = if is_silent {
            println!("⚙️ 靜默模式執行中，跳過手動放入 Cookie 的互動等待步驟。");
            false
        } else {
            Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("此內容可能需要 Cookie 才能取得完整權限。是否要現在開啟設定檔放入？")
                .default(true)
                .interact()
                .unwrap()
        };

        if want_to_wait {
            open_config_folder();
            
            println!("⏳ 請將 {} 放入剛開啟的資料夾中。", expected_filename);
            println!("👉 放入完成後，請在終端機按下「Enter」鍵繼續...");
            
            let mut _pause = String::new();
            io::stdin().read_line(&mut _pause).unwrap();

            if target_file.exists() {
                println!("🎉 偵測到 Cookie 檔案！已成功套用。");
                cookie_args.push("--cookies".to_string());
                cookie_args.push(target_file.to_string_lossy().to_string());
            } else {
                println!("⏳ 仍未偵測到檔案。程式將以無 Cookie 的「訪客模式」繼續。");
            }
        } else if !is_silent {
            println!("👻 跳過設定，程式將以「訪客模式」繼續嘗試下載。");
        }
    }

    cookie_args
}