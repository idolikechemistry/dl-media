use chrono::Local;
use clap::Parser;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use dirs::{config_dir, download_dir};
use regex::Regex;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(name = "dl-media", about = "全能影音分析與下載器 (v0.1.9)")]
struct Args {
    // 🌟 v0.1.9 更新：補齊所有參數的詳細說明
    #[arg(short, long, help = "貼上要下載的影片或播放清單網址")]
    url: Option<String>,
    #[arg(short, long, help = "指定下載類型 (音訊 → 1, 無聲影片 → 2, 有聲影片 → 3)")]
    media_type: Option<u8>,
    #[arg(short, long, help = "指定輸出格式 (音訊格式 → mp3 or m4a，影片格式 → mp4 or mkv)")]
    format: Option<String>,
    #[arg(short, long, help = "手動指定 Cookie 檔案路徑")]
    cookie: Option<String>,
    #[arg(long, help = "打開應用程式設定資料夾 (用來放入 Cookie)")]
    open_config: bool,
    #[arg(long = "fc", help = "強制調用 config 內已經儲存好的 Cookie，若無檔案則必定開啟資料夾等待")]
    force_cookie: bool,
}

fn check_dependencies() {
    let deps = ["yt-dlp", "ffmpeg", "ffprobe"];
    for dep in deps {
        if Command::new(dep).arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_err() {
            eprintln!("❌ 錯誤：未安裝 {}，請先安裝該工具。", dep);
            std::process::exit(1);
        }
    }
}

fn get_app_config_dir() -> PathBuf {
    let mut path = config_dir().expect("無法取得系統設定目錄");
    path.push("dl-media");
    if !path.exists() {
        fs::create_dir_all(&path).expect("無法建立應用程式設定資料夾");
    }
    path
}

fn open_config_folder() {
    let path = get_app_config_dir();
    println!("📂 正在開啟設定資料夾：{}", path.display());
    println!("💡 請將您的 Cookie 檔案放入此資料夾中，並以 'cookie_網站名.txt' 命名。");
    println!("   範例：cookie_youtube.txt, cookie_bilibili.txt, cookie_twitter.txt");
    
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(&path).status();
    #[cfg(target_os = "windows")]
    let _ = Command::new("explorer").arg(&path).status();
    #[cfg(target_os = "linux")]
    let _ = Command::new("xdg-open").arg(&path).status();
}

fn has_subtitles(url: &str, cookie_args: &[String]) -> bool {
    let mut cmd = Command::new("yt-dlp");
    cmd.args(cookie_args).args(["--dump-json", "--no-warnings", "--playlist-items", "1", url]);
    if let Ok(output) = cmd.output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
            let has_subs = json.get("subtitles").map_or(false, |s| s.is_object() && !s.as_object().unwrap().is_empty());
            let has_auto_subs = json.get("automatic_captions").map_or(false, |s| s.is_object() && !s.as_object().unwrap().is_empty());
            return has_subs || has_auto_subs;
        }
    }
    false
}

fn clean_vtt_file(original_path: &PathBuf, clean_path: &PathBuf) {
    if let Ok(content) = fs::read_to_string(original_path) {
        let re = Regex::new(r"<\/?c[^>]*>").unwrap();
        let cleaned_content = re.replace_all(&content, "");
        let _ = fs::write(clean_path, cleaned_content.as_ref());
    }
}

fn get_video_resolution(file_path: &PathBuf) -> Option<String> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "error",                           
            "-select_streams", "v:0",                
            "-show_entries", "stream=width,height",  
            "-of", "csv=s=x:p=0",                    
            file_path.to_str().unwrap()
        ])
        .output()
        .ok()?;

    if output.status.success() {
        let res = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !res.is_empty() {
            return Some(res);
        }
    }
    None
}

fn extract_site_name(url: &str) -> String {
    let host = url.replace("https://", "").replace("http://", "").replace("www.", "");
    let domain = host.split('/').next().unwrap_or("unknown");
    
    if domain.contains("youtu.be") || domain.contains("youtube.com") { return "youtube".to_string(); }
    if domain.contains("b23.tv") || domain.contains("bilibili.com") { return "bilibili".to_string(); }
    if domain.contains("x.com") || domain.contains("twitter.com") { return "twitter".to_string(); }
    if domain.contains("fb.watch") || domain.contains("facebook.com") { return "facebook".to_string(); }
    if domain.contains("instagram.com") { return "instagram".to_string(); }
    
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() >= 2 {
        parts[parts.len() - 2].to_string()
    } else {
        domain.to_string()
    }
}

fn main() {
    let args = Args::parse();

    // 🌟 v0.1.9 新增：早期防呆驗證，確保格式字串正確
    if let Some(ref fmt) = args.format {
        let valid_formats = ["mp4", "mkv", "mp3", "m4a"];
        if !valid_formats.contains(&fmt.to_lowercase().as_str()) {
            eprintln!("❌ 錯誤：不支援的格式設定 '{}'。", fmt);
            eprintln!("💡 提示：-f 參數請輸入正確的副檔名，不要輸入數字。");
            eprintln!("   - 音訊常用格式：mp3, m4a");
            eprintln!("   - 影片常用格式：mp4, mkv");
            std::process::exit(1);
        }
    }

    if args.open_config {
        open_config_folder();
        std::process::exit(0);
    }

    check_dependencies();

    let mut active_params = Vec::new();
    if args.url.is_some() { active_params.push("已指定網址 (-u)".to_string()); }
    if let Some(m) = args.media_type {
        let type_name = match m {
            1 => "音訊",
            2 => "無聲影片",
            3 => "有聲影片",
            _ => "未知類型",
        };
        active_params.push(format!("類型:{} (-m)", type_name));
    }
    if let Some(ref f) = args.format { active_params.push(format!("格式:{} (-f)", f)); }
    if let Some(ref c) = args.cookie { active_params.push(format!("自訂Cookie路徑: {} (-c)", c)); }
    if args.force_cookie { active_params.push("強制調用Cookie (--fc)".to_string()); }

    // ==========================================
    // 步驟 1：輸入網址與顯示歡迎介面
    // ==========================================
    let input_url = match args.url {
        Some(url) => {
            println!("dl-media v0.1.9 🚀");
            if !active_params.is_empty() {
                println!("⚙️ 當前執行指令：{}", active_params.join(", "));
            }
            url
        },
        None => {
            println!("dl-media v0.1.9 🚀");
            println!("(提示：輸入 './dl-media --help' 查看所有可用參數，例如 --fc 強制調用 Cookie)");
            
            if !active_params.is_empty() {
                println!("⚙️ 當前執行指令：{}", active_params.join(", "));
            }
            
            Input::with_theme(&ColorfulTheme::default()).with_prompt("🔗 請貼上影片或播放清單網址").interact_text().unwrap()
        }
    };
    if input_url.trim().is_empty() { std::process::exit(1); }

    let site_target = extract_site_name(&input_url);

    // ==========================================
    // 步驟 2：初次精確解析
    // ==========================================
    println!("🔍 正在初步分析網址資訊...");
    
    let pl_check = Command::new("yt-dlp")
        .args(["--print", "playlist_title", "--no-warnings", "--playlist-items", "1", &input_url])
        .output().expect("檢查清單屬性失敗");
    let pl_title = String::from_utf8_lossy(&pl_check.stdout).trim().to_string();
    let is_playlist = !pl_title.is_empty() && pl_title != "NA" && pl_title != "null";

    let scan_output = Command::new("yt-dlp")
        .args(["--flat-playlist", "--print", "%(title)s|%(webpage_url)s", "--ignore-errors", "--no-warnings", &input_url])
        .output().expect("解析清單失敗");
    
    let stdout_str = String::from_utf8_lossy(&scan_output.stdout);
    let stderr_str = String::from_utf8_lossy(&scan_output.stderr).to_lowercase();
    
    let mut valid_urls: Vec<String> = Vec::new();
    let mut has_restricted = args.force_cookie || stderr_str.contains("sign in") || stderr_str.contains("login") 
        || stderr_str.contains("cookie") || stderr_str.contains("登錄") || stderr_str.contains("private");

    for line in stdout_str.lines() {
        if line.trim().is_empty() { continue; }
        if line.contains("[Private video]") || line.contains("[Deleted video]") || line.contains("Private") {
            has_restricted = true;
        } else if let Some((_title, url)) = line.rsplit_once('|') {
            valid_urls.push(url.to_string());
        } else {
            valid_urls.push(line.to_string());
        }
    }
    
    if valid_urls.is_empty() {
        valid_urls.push(input_url.clone());
    }
    let mut total = valid_urls.len();

    println!("--------------------------------------------------");
    println!("📡 來源網站：{}", site_target);
    if is_playlist {
        println!("📋 內容類型：【播放清單】 (包含 {} 部內容)", total);
    } else {
        println!("📄 內容類型：【單一內容】");
    }

    if args.force_cookie {
        println!("🔒 權限狀態：⚠️ 您已啟用強制調用 config 內 Cookie 的模式 (--fc)");
    } else if has_restricted {
        println!("🔒 權限狀態：⚠️ 偵測到受限/私人內容！(需提供 Cookie 才能解鎖)");
    } else if site_target == "bilibili" {
        println!("🔓 權限狀態：公開 (💡 Bilibili 建議使用 Cookie 解鎖 1080p 高畫質)");
        has_restricted = true;
    } else {
        if site_target == "instagram" || site_target == "facebook" || site_target == "twitter" {
            println!("🔓 權限狀態：公開 (💡 若實際下載失敗，可加上 --fc 參數重試)");
        } else {
            println!("🔓 權限狀態：公開 (目前無偵測到存取限制)");
        }
    }
    println!("--------------------------------------------------");

    // ==========================================
    // 步驟 3 & 4：判斷與處理 Cookie
    // ==========================================
    let mut cookie_args = Vec::new();
    let mut has_loaded_cookie = false;

    if let Some(ref manual_cookie) = args.cookie {
        let path = PathBuf::from(manual_cookie);
        if path.exists() {
            cookie_args.push("--cookies".to_string());
            cookie_args.push(path.to_string_lossy().to_string());
            println!("🍪 已載入自訂 Cookie 檔案：{}", path.display());
            has_loaded_cookie = true;
        } else {
            println!("⚠️ 找不到指定的 Cookie 檔案：{}", path.display());
        }
    } else {
        let config_path = get_app_config_dir();
        let expected_filename = format!("cookie_{}.txt", site_target);
        let target_file = config_path.join(&expected_filename);
        
        if target_file.exists() {
            cookie_args.push("--cookies".to_string());
            cookie_args.push(target_file.to_string_lossy().to_string());
            println!("🍪 已從 config 載入 {} 專用 Cookie：{}", site_target, expected_filename);
            has_loaded_cookie = true;
        } else if has_restricted {
            println!("⚠️ config 內未偵測到 {} 專用 Cookie ({})", site_target, expected_filename);
            
            let want_to_wait = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("此內容可能需要 Cookie 才能取得完整權限。是否要現在開啟設定檔放入？")
                .default(true)
                .interact()
                .unwrap();

            if want_to_wait {
                println!("📂 正在為您開啟 config 目錄：{}", config_path.display());
                #[cfg(target_os = "macos")]
                let _ = Command::new("open").arg(&config_path).status();
                #[cfg(target_os = "windows")]
                let _ = Command::new("explorer").arg(&config_path).status();
                #[cfg(target_os = "linux")]
                let _ = Command::new("xdg-open").arg(&config_path).status();

                println!("⏳ 請將 {} 放入剛開啟的資料夾中。", expected_filename);
                println!("👉 放入完成後，請在終端機按下「Enter」鍵繼續...");
                
                let mut _pause = String::new();
                io::stdin().read_line(&mut _pause).unwrap();

                if target_file.exists() {
                    println!("🎉 偵測到 Cookie 檔案！已成功套用。");
                    cookie_args.push("--cookies".to_string());
                    cookie_args.push(target_file.to_string_lossy().to_string());
                    has_loaded_cookie = true;
                } else {
                    println!("⏳ 仍未偵測到檔案。程式將以無 Cookie 的「訪客模式」繼續。");
                }
            } else {
                println!("👻 跳過設定，程式將以「訪客模式」繼續嘗試下載。");
            }
        }
    }

    // ==========================================
    // 步驟 5：重新掃描
    // ==========================================
    if has_loaded_cookie && is_playlist {
        println!("🔄 正在透過 Cookie 驗證並重新掃描清單...");
        let rescan_output = Command::new("yt-dlp")
            .args(&cookie_args)
            .args(["--flat-playlist", "--print", "%(title)s|%(webpage_url)s", "--ignore-errors", "--no-warnings", &input_url])
            .output().expect("重新解析清單失敗");
            
        let rescan_str = String::from_utf8_lossy(&rescan_output.stdout);
        let mut new_urls: Vec<String> = Vec::new();
        
        for line in rescan_str.lines() {
            if line.trim().is_empty() { continue; }
            if line.contains("[Private video]") || line.contains("[Deleted video]") { continue; }
            if let Some((_title, url)) = line.rsplit_once('|') {
                new_urls.push(url.to_string());
            } else {
                new_urls.push(line.to_string());
            }
        }
            
        if new_urls.is_empty() {
            new_urls.push(input_url.clone());
        }
        
        let new_total = new_urls.len();
        
        if new_total > total {
            println!("--------------------------------------------------");
            println!("🔓 解鎖成功！透過 Cookie 發現了隱藏/會員專屬內容。");
            println!("📋 更新解析結果：共包含 {} 部有效內容！", new_total);
            println!("--------------------------------------------------");
        }
        
        valid_urls = new_urls;
        total = new_total;
    }

    // ==========================================
    // 步驟 6 & 7：選擇類型與格式
    // ==========================================
    println!("--------------------------------------------------");
    let media_type = match args.media_type {
        Some(mt) => mt,
        None => {
            let types = vec!["音訊", "無聲影片", "有聲影片"];
            (Select::with_theme(&ColorfulTheme::default()).with_prompt("下載類型").default(0).items(&types).interact().unwrap() + 1) as u8
        }
    };

    let target_ext = match args.format {
        Some(fmt) => fmt,
        None => {
            if media_type == 1 {
                let formats = vec!["M4A (原生無損)", "MP3 (320k)"];
                let sel = Select::with_theme(&ColorfulTheme::default()).with_prompt("音訊格式").default(0).items(&formats).interact().unwrap();
                if sel == 1 { "mp3".to_string() } else { "m4a".to_string() }
            } else {
                let formats = vec!["MP4 (高相容)", "MKV (最高畫質)"];
                let sel = Select::with_theme(&ColorfulTheme::default()).with_prompt("影片格式").default(0).items(&formats).interact().unwrap();
                if sel == 1 { "mkv".to_string() } else { "mp4".to_string() }
            }
        }
    };

    let mut dl_args: Vec<String> = vec![
        "--ignore-errors", "--no-overwrites", "--embed-thumbnail", "--embed-metadata", 
        "--embed-chapters", "--convert-thumbnails", "jpg", "--restrict-filenames",
        "--sponsorblock-remove", "sponsor,intro,outro",
    ].into_iter().map(String::from).collect();

    if has_subtitles(&input_url, &cookie_args) {
        if media_type == 1 {
            dl_args.extend(vec!["--write-subs".into(), "--write-auto-subs".into()]);
        } else {
            dl_args.extend(vec!["--embed-subs".into(), "--write-subs".into(), "--write-auto-subs".into()]);
        }
        dl_args.extend(vec!["--sub-langs".into(), "zh-Hant,zh-TW,zh-HK,zh-Hans,zh,en,ja,danmaku".into()]);
    }

    if media_type == 1 {
        dl_args.extend(vec!["--extract-audio".into(), "--audio-format".into(), target_ext.clone()]);
        if target_ext == "mp3" {
            dl_args.extend(vec!["--audio-quality".into(), "320k".into(), "-f".into(), "bestaudio".into()]);
        } else {
            dl_args.extend(vec!["-f".into(), "bestaudio[ext=m4a]/bestaudio".into()]);
        }
    } else {
        dl_args.extend(vec!["--merge-output-format".into(), target_ext.clone()]);
        if target_ext == "mkv" {
            dl_args.extend(vec!["-f".into(), "bv*+ba/best".into()]);
        } else {
            dl_args.extend(vec!["-f".into(), "bv*[vcodec^=avc]+ba[ext=m4a]/best[ext=mp4]/best".into()]);
        }
    }

    let mut target_dir = download_dir().expect("無法找到下載目錄");
    
    if is_playlist {
        let title_output = Command::new("yt-dlp")
            .args(&cookie_args).args(["--print", "playlist_title", "--no-warnings", "--playlist-items", "1", &input_url])
            .output().expect("取得標題失敗");
        let mut title = String::from_utf8_lossy(&title_output.stdout).trim().to_string();
        if title.is_empty() || title == "NA" { title = "Playlist".to_string(); }
        let safe_title = title.replace(&['/', '\\', ':', '*', '?', '"', '<', '>', '|'][..], "_");
        target_dir = target_dir.join(safe_title);
        fs::create_dir_all(&target_dir).expect("建立目錄失敗");
        println!("📚 已建立清單專屬資料夾: {}", target_dir.display());
    }

    let mut success_count = 0;
    let mut fail_count = 0;
    let start_time = Instant::now();

    // ==========================================
    // 步驟 8：核心下載迴圈
    // ==========================================
    for (idx, video_url) in valid_urls.iter().enumerate() {
        let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();
        println!("=================================================");
        println!("🎬 準備下載 ({}/{})...", idx + 1, total);

        let title_output = Command::new("yt-dlp").args(["--get-title", video_url]).output().unwrap();
        let raw_title = String::from_utf8_lossy(&title_output.stdout).trim().replace(&['/', '\\', ':', '*', '?', '"', '<', '>', '|'][..], "_");
        let safe_title = if raw_title.is_empty() { "Video".to_string() } else { raw_title };
        
        let final_name = if is_playlist {
            format!("{:02}-{}_{}.{}", idx + 1, safe_title, ts, target_ext)
        } else {
            format!("{}_{}.{}", safe_title, ts, target_ext)
        };

        let tmp_base = target_dir.join(format!("tmp_{}", ts));
        let tmp_output_template = format!("{}.%(ext)s", tmp_base.to_string_lossy());

        let mut current_dl_args = dl_args.clone();
        current_dl_args.push("-o".into());
        current_dl_args.push(tmp_output_template);
        current_dl_args.push(video_url.to_string());

        let status = Command::new("yt-dlp").args(&cookie_args).args(&current_dl_args).status().expect("執行 yt-dlp 失敗");

        if status.success() {
            let mut main_file: Option<PathBuf> = None;
            if let Ok(entries) = fs::read_dir(&target_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                    if file_name.starts_with(&format!("tmp_{}.", ts)) 
                        && !file_name.ends_with(".vtt") && !file_name.ends_with(".srt") 
                        && !file_name.ends_with(".xml") && !file_name.ends_with(".ass") 
                    {
                        main_file = Some(path);
                        break;
                    }
                }
            }

            if let Some(downloaded_file) = main_file {
                let final_path = target_dir.join(&final_name);

                let xml_path = target_dir.join(format!("tmp_{}.danmaku.xml", ts));
                if xml_path.exists() && media_type != 1 {
                    let ass_path = target_dir.join(format!("tmp_{}.ass", ts));
                    let _ = Command::new("danmaku2ass").args([xml_path.to_str().unwrap(), "-o", ass_path.to_str().unwrap()]).status();
                    
                    if ass_path.exists() {
                        let _ = Command::new("ffmpeg")
                            .args(["-hide_banner", "-loglevel", "error", "-y"])
                            .args(["-i", downloaded_file.to_str().unwrap()])
                            .args(["-i", ass_path.to_str().unwrap()])
                            .args(["-map", "0", "-map", "1", "-c", "copy", "-c:s", "mov_text"])
                            .arg(final_path.to_str().unwrap())
                            .status();
                        let _ = fs::remove_file(downloaded_file);
                    } else {
                        let _ = fs::rename(downloaded_file, &final_path);
                    }
                } else {
                    let _ = fs::rename(downloaded_file, &final_path);
                }

                if media_type == 1 {
                    if let Ok(entries) = fs::read_dir(&target_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            let ext = path.extension().unwrap_or_default().to_string_lossy();
                            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                            
                            if file_name.starts_with(&format!("tmp_{}.", ts)) && (ext == "vtt" || ext == "srt") {
                                let parts: Vec<&str> = file_name.split('.').collect();
                                let lang = if parts.len() >= 3 { parts[parts.len() - 2] } else { "" };
                                
                                let base_name = final_name.rsplit_once('.').map(|(b, _)| b).unwrap_or(&final_name);
                                let original_sub = target_dir.join(format!("{}.{}.{}", base_name, lang, ext));
                                let clean_sub = target_dir.join(format!("{}_clean.{}.{}", base_name, lang, ext));
                                
                                let _ = fs::rename(&path, &original_sub);
                                clean_vtt_file(&original_sub, &clean_sub);
                            }
                        }
                    }
                }

                let resolution_info = if media_type != 1 {
                    match get_video_resolution(&final_path) {
                        Some(res) => format!(" [畫質: {}]", res),
                        None => "".to_string(),
                    }
                } else {
                    "".to_string()
                };

                success_count += 1;
                println!("✅ 儲存成功：{}{}", final_name, resolution_info);
            } else {
                fail_count += 1;
                println!("❌ 找不到已下載的主檔案。");
            }

            if let Ok(entries) = fs::read_dir(&target_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                    if file_name.starts_with(&format!("tmp_{}", ts)) {
                        let _ = fs::remove_file(path);
                    }
                }
            }

        } else {
            fail_count += 1;
            println!("⚠️ 下載失敗。");
        }
    }

    let duration = start_time.elapsed();
    let secs = duration.as_secs();
    let time_str = if secs >= 60 {
        format!("{} 分 {} 秒", secs / 60, secs % 60)
    } else {
        format!("{} 秒", secs)
    };

    println!("---");
    println!("🎉 任務全部完成！儲存位置：{}", target_dir.display());
    println!("⏱️ 總耗時：{}", time_str);
    
    if success_count > 0 { 
        println!("✨ {} successful", success_count); 
    }
    
    if fail_count > 0 { 
        println!("⚠️ {} failed", fail_count);
        if !args.force_cookie {
            println!("💡 提示：若失敗項目屬於私密、會員限定或年齡限制內容，請加上 '--fc' 參數重新執行以強制調用 config 內的 Cookie！");
        }
    }
}