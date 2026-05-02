use crate::parser;
use chrono::Local;
use dirs::download_dir;
use regex::Regex;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

pub fn build_download_args(media_type: u8, target_ext: &str, input_url: &str, cookie_args: &[String]) -> Vec<String> {
    let mut dl_args: Vec<String> = vec![
        "--quiet".into(), "--progress".into(), "--no-warnings".into(), "--ignore-errors".into(),
        "--no-overwrites".into(), "--embed-thumbnail".into(), "--embed-metadata".into(),
        "--embed-chapters".into(), "--convert-thumbnails".into(), "jpg".into(), "--restrict-filenames".into(),
        "--sponsorblock-remove".into(), "sponsor,intro,outro".into(),
    ];

    if parser::has_subtitles(input_url, cookie_args) {
        if media_type == 1 {
            dl_args.extend(vec!["--write-subs".into(), "--write-auto-subs".into()]);
        } else {
            dl_args.extend(vec!["--embed-subs".into(), "--write-subs".into(), "--write-auto-subs".into()]);
        }
        dl_args.extend(vec!["--sub-langs".into(), "zh-Hant,zh-TW,zh-HK,zh-Hans,zh,en,ja,danmaku".into()]);
    }

    if media_type == 1 {
        dl_args.extend(vec!["--extract-audio".into(), "--audio-format".into(), target_ext.into()]);
        if target_ext == "mp3" { dl_args.extend(vec!["--audio-quality".into(), "320k".into(), "-f".into(), "bestaudio".into()]); }
        else { dl_args.extend(vec!["-f".into(), "bestaudio[ext=m4a]/bestaudio".into()]); }
    } else {
        dl_args.extend(vec!["--merge-output-format".into(), target_ext.into()]);
        if target_ext == "mkv" { dl_args.extend(vec!["-f".into(), "bv*+ba/best".into()]); }
        else { dl_args.extend(vec!["-f".into(), "bv*[vcodec^=avc]+ba[ext=m4a]/best[ext=mp4]/best".into()]); }
    }
    dl_args
}

pub fn prepare_output_dir(output_arg: &Option<String>, input_url: &str, cookie_args: &[String], is_playlist: bool) -> PathBuf {
    let mut target_dir = output_arg.as_ref().map(PathBuf::from).unwrap_or_else(|| download_dir().expect("找不到下載目錄"));
    if !target_dir.exists() { fs::create_dir_all(&target_dir).ok(); }
    
    if is_playlist {
        let out = Command::new("yt-dlp").args(cookie_args).args(["--print", "playlist_title", "--no-warnings", "--playlist-items", "1", input_url]).output().ok();
        let title = out.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).filter(|t| !t.is_empty() && t != "NA").unwrap_or_else(|| "Playlist".into());
        target_dir = target_dir.join(title.replace(&['/', '\\', ':', '*', '?', '"', '<', '>', '|'][..], "_"));
        fs::create_dir_all(&target_dir).ok();
    }
    target_dir
}

pub fn clean_vtt_file(original_path: &PathBuf, clean_path: &PathBuf) {
    if let Ok(content) = fs::read_to_string(original_path) {
        let re = Regex::new(r"<\/?c[^>]*>").unwrap();
        let cleaned_content = re.replace_all(&content, "");
        let _ = fs::write(clean_path, cleaned_content.as_ref());
    }
}

pub fn get_video_resolution(file_path: &PathBuf) -> Option<String> {
    let output = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=s=x:p=0", file_path.to_str().unwrap()])
        .output()
        .ok()?;
    if output.status.success() {
        let res = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !res.is_empty() { return Some(res); }
    }
    None
}

pub fn execute_download_loop(
    valid_urls: Vec<String>,
    is_playlist: bool,
    media_type: u8,
    target_ext: &str,
    dl_args: Vec<String>,
    cookie_args: Vec<String>,
    target_dir: PathBuf,
    force_cookie: bool,
) {
    let mut success_count = 0;
    let mut fail_count = 0;
    let start_time = Instant::now();
    let total = valid_urls.len();

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

    crate::ui::print_summary(success_count, fail_count, &time_str, &target_dir.to_string_lossy());
    if fail_count > 0 && !force_cookie {
        println!("💡 提示：若失敗項目屬於私密、會員限定或年齡限制內容，請加上 '--fc' 參數重新執行以強制調用 config 內的 Cookie！");
    }
}