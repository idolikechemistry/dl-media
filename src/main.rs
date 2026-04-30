use chrono::Local;
use clap::Parser;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use dirs::download_dir;
use regex::Regex;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Parser, Debug)]
#[command(name = "dl-media", about = "全能影音分析與下載器 (Rust v5.3.2 完全體)")]
struct Args {
    #[arg(short, long)]
    url: Option<String>,
    #[arg(short, long)]
    media_type: Option<u8>,
    #[arg(short, long)]
    format: Option<String>,
}

fn check_dependencies() {
    let deps = ["yt-dlp", "ffmpeg"];
    for dep in deps {
        if Command::new(dep).arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_err() {
            eprintln!("❌ 錯誤：未安裝 {}，請先安裝該工具。", dep);
            std::process::exit(1);
        }
    }
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

fn main() {
    check_dependencies();
    let args = Args::parse();

    let input_url = match args.url {
        Some(url) => url,
        None => {
            println!("📥 全能下載器 v5.3.2 (Rust 完整版)");
            Input::with_theme(&ColorfulTheme::default()).with_prompt("🔗 請貼上影片或播放清單網址").interact_text().unwrap()
        }
    };
    if input_url.trim().is_empty() { std::process::exit(1); }

    let media_type = match args.media_type {
        Some(mt) => mt,
        None => {
            let types = vec!["🎧 音訊", "🔕 無聲影片", "🎥 有聲影片"];
            (Select::with_theme(&ColorfulTheme::default()).with_prompt("🎯 下載類型").default(0).items(&types).interact().unwrap() + 1) as u8
        }
    };

    let target_ext = match args.format {
        Some(fmt) => fmt,
        None => {
            if media_type == 1 {
                let formats = vec!["M4A (原生無損)", "MP3 (320k)"];
                let sel = Select::with_theme(&ColorfulTheme::default()).with_prompt("🎵 音訊格式").default(0).items(&formats).interact().unwrap();
                if sel == 1 { "mp3".to_string() } else { "m4a".to_string() }
            } else {
                let formats = vec!["MP4 (高相容)", "MKV (最高畫質)"];
                let sel = Select::with_theme(&ColorfulTheme::default()).with_prompt("🎞️ 影片格式").default(0).items(&formats).interact().unwrap();
                if sel == 1 { "mkv".to_string() } else { "mp4".to_string() }
            }
        }
    };

    let mut cookie_args = vec!["--cookies-from-browser".to_string(), "safari".to_string()];
    let bilibili_cookie = PathBuf::from("/opt/homebrew/yt-dlp_cookie_bilibili.txt");
    if input_url.contains("bilibili.com") && bilibili_cookie.exists() {
        cookie_args = vec!["--cookies".to_string(), bilibili_cookie.to_string_lossy().to_string()];
    }

    let mut dl_args: Vec<String> = vec![
        "--ignore-errors", "--no-overwrites", "--embed-thumbnail", "--embed-metadata", 
        "--embed-chapters", "--convert-thumbnails", "jpg", "--restrict-filenames",
        "--sponsorblock-remove", "sponsor,intro,outro",
    ].into_iter().map(String::from).collect();

    println!("🔍 正在分析資訊...");
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

    // === 播放清單解析與目錄建立 ===
    let playlist_output = Command::new("yt-dlp")
        .args(&cookie_args).args(["--flat-playlist", "--print", "webpage_url", &input_url])
        .output().expect("解析清單失敗");
    let urls_str = String::from_utf8_lossy(&playlist_output.stdout);
    let mut playlist_urls: Vec<&str> = urls_str.lines().filter(|l| !l.trim().is_empty()).collect();
    
    if playlist_urls.is_empty() {
        playlist_urls.push(&input_url);
    }
    
    let total = playlist_urls.len();
    let mut target_dir = download_dir().expect("無法找到下載目錄");
    
    if total > 1 {
        let title_output = Command::new("yt-dlp")
            .args(&cookie_args).args(["--flat-playlist", "--print", "playlist_title", "--playlist-items", "1", &input_url])
            .output().expect("取得標題失敗");
        let mut title = String::from_utf8_lossy(&title_output.stdout).trim().to_string();
        if title.is_empty() { title = "Playlist".to_string(); }
        let safe_title = title.replace(&['/', '\\', ':', '*', '?', '"', '<', '>', '|'][..], "_");
        target_dir = target_dir.join(safe_title);
        fs::create_dir_all(&target_dir).expect("建立目錄失敗");
        println!("📚 建立清單資料夾: {}", target_dir.display());
    }

    let mut success_count = 0;
    let mut fail_count = 0;

    // === 核心下載迴圈 ===
    for (idx, video_url) in playlist_urls.iter().enumerate() {
        let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();
        println!("=================================================");
        println!("🎬 下載中 ({}/{})...", idx + 1, total);

        let title_output = Command::new("yt-dlp").args(["--get-title", video_url]).output().unwrap();
        let raw_title = String::from_utf8_lossy(&title_output.stdout).trim().replace(&['/', '\\', ':', '*', '?', '"', '<', '>', '|'][..], "_");
        let safe_title = if raw_title.is_empty() { "Video".to_string() } else { raw_title };
        
        let final_name = if total > 1 {
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
            // 尋找主媒體檔案
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

                // === 彈幕合併邏輯 ===
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

                // === 字幕保留與淨化邏輯 ===
                if media_type == 1 {
                    if let Ok(entries) = fs::read_dir(&target_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            let ext = path.extension().unwrap_or_default().to_string_lossy();
                            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                            
                            if file_name.starts_with(&format!("tmp_{}.", ts)) && (ext == "vtt" || ext == "srt") {
                                // 提取語言代碼 (例如 tmp_xxx.zh-Hant.vtt -> zh-Hant)
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

                success_count += 1;
                println!("✅ 儲存成功：{}", final_name);
            } else {
                fail_count += 1;
                println!("❌ 找不到已下載的主檔案。");
            }

            // 終極清理暫存檔
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

    println!("---");
    println!("🎉 任務全部完成！儲存位置：{}", target_dir.display());
    if success_count > 0 { println!("✨ success {}", success_count); }
    if fail_count > 0 { println!("⚠️ failed {}", fail_count); }
}
