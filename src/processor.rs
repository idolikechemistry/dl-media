use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// 🎯 註記：原本的 find_main_file 已經被移除，改為由 yt-dlp 直接回傳絕對路徑！

pub fn process_external_subtitles(
    tmp_dir: &Path,
    ts: &str,
    final_name: &str,
    target_dir: &Path,
    media_type: u8,
) {
    let entries = fs::read_dir(tmp_dir).unwrap();
    let re_vtt = Regex::new(r"tmp_.*\.vtt$").unwrap();
    let re_tags = Regex::new(r"<[^>]*>").unwrap();

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap();

        if re_vtt.is_match(file_name) && file_name.contains(ts) && !file_name.contains(".clean.vtt")
        {
            if let Ok(content) = fs::read_to_string(&path) {
                let cleaned = re_tags.replace_all(&content, "");
                let clean_path = path.with_extension("clean.vtt");
                let _ = fs::write(&clean_path, cleaned.to_string());

                if media_type == 1 {
                    let lang_suffix = file_name.split('.').rev().nth(1).unwrap_or("sub");
                    let final_vtt_name = final_name.replace(
                        &Path::new(final_name).extension().unwrap().to_str().unwrap(),
                        &format!("{}.vtt", lang_suffix),
                    );
                    let _ = fs::rename(&clean_path, target_dir.join(final_vtt_name));
                }
            }
        }
    }
}

pub fn merge_subs_and_danmaku(
    tmp_dir: &Path,
    ts: &str,
    video_path: &Path,
    final_path: &Path,
) -> bool {
    let mut sub_files: Vec<(PathBuf, String, String)> = Vec::new();

    if let Ok(entries) = fs::read_dir(tmp_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&format!("tmp_{}", ts))
                && (name.ends_with(".ass") || name.ends_with(".clean.vtt"))
            {
                let parts: Vec<&str> = name.split('.').collect();
                let raw_lang = if name.ends_with(".clean.vtt") && parts.len() >= 4 {
                    parts[parts.len() - 3].to_string()
                } else if parts.len() >= 3 {
                    parts[parts.len() - 2].to_string()
                } else {
                    "und".to_string()
                };

                let (iso_lang, display_title) = match raw_lang.as_str() {
                    "zh-Hant" | "zh-TW" | "zh-HK" => ("chi", "繁體中文"),
                    "zh-Hans" | "zh-CN" | "zh" => ("zho", "簡體中文"),
                    "en" | "en-US" | "en-GB" => ("eng", "English"),
                    "ja" => ("jpn", "日本語"),
                    "ko" => ("kor", "한국어"),
                    "danmaku" => ("cmn", "中文彈幕"),
                    _ => ("und", raw_lang.as_str()),
                };

                sub_files.push((
                    entry.path(),
                    display_title.to_string(),
                    iso_lang.to_string(),
                ));
            }
        }
    }

    if sub_files.is_empty() {
        return false;
    }
    sub_files.sort_by(|a, b| a.1.cmp(&b.1));
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-loglevel").arg("error");
    cmd.arg("-hide_banner");
    cmd.arg("-i").arg(video_path);

    for (sub_path, _, _) in &sub_files {
        cmd.arg("-i").arg(sub_path);
    }

    cmd.arg("-c:v").arg("copy").arg("-c:a").arg("copy");

    if final_path.extension().and_then(|e| e.to_str()) == Some("mp4") {
        cmd.arg("-c:s").arg("mov_text");
        // 🎯 核心補強：告訴 FFmpeg 在 MP4 寫入時，盡量保留並使用元數據標籤
        cmd.arg("-movflags").arg("+use_metadata_tags");
    } else {
        cmd.arg("-c:s").arg("copy");
    }

    cmd.arg("-map").arg("0");
    for i in 1..=sub_files.len() {
        cmd.arg("-map").arg(format!("{}", i));
    }

    for (i, (_, title, iso)) in sub_files.iter().enumerate() {
        cmd.arg(format!("-metadata:s:s:{}", i))
            .arg(format!("language={}", iso));
        cmd.arg(format!("-metadata:s:s:{}", i))
            .arg(format!("title={}", title));
        cmd.arg(format!("-metadata:s:s:{}", i))
            .arg(format!("handler_name={}", title));
        // 🎯 針對 Apple 體系：設置 'name' 標籤，這是某些 MP4 播放器讀取標題的最後希望
        cmd.arg(format!("-metadata:s:s:{}", i))
            .arg(format!("name={}", title));
    }

    cmd.arg("-y").arg(final_path);
    cmd.status().map_or(false, |s| s.success())
}

pub fn get_video_resolution(path: &Path) -> Option<String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=s=x:p=0",
            path.to_str().unwrap(),
        ])
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// 終極大掃除
pub fn cleanup_tmps(session_tmp_dir: &Path) {
    if session_tmp_dir.exists() {
        // 直接移除整個時間戳資料夾及其內部所有檔案
        let _ = fs::remove_dir_all(session_tmp_dir);
    }
}
