use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// 全域編譯一次正則表達式，提升效能
static VTT_REGEX: OnceLock<Regex> = OnceLock::new();

/// 在目標目錄中尋找下載好的暫存主檔案 (排除字幕與彈幕檔)
pub fn find_main_file(target_dir: &Path, ts: &str) -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir(target_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name()?.to_string_lossy();
            if file_name.starts_with(&format!("tmp_{}.", ts))
                && !file_name.ends_with(".vtt")
                && !file_name.ends_with(".srt")
                && !file_name.ends_with(".xml")
                && !file_name.ends_with(".ass")
            {
                return Some(path);
            }
        }
    }
    None
}

/// 處理 Bilibili 彈幕轉換與影片封裝
pub fn handle_danmaku_merge(tmp_dir: &std::path::Path, ts: &str, main_file: &std::path::Path, final_path: &std::path::Path) -> bool {
    let xml_path = match find_specific_file(tmp_dir, ts, "danmaku", ".xml") {
        Some(p) => p,
        None => return false, 
    };

    println!("🎨 偵測到彈幕檔案，正在進行後製轉換...");

    // 動態獲取影片解析度
    let resolution = get_video_resolution(main_file).unwrap_or_else(|| "1920x1080".to_string());
    println!("📐 偵測到影片解析度：{}，已套用至彈幕畫布", resolution);

    let ass_path = tmp_dir.join(format!("tmp_{}.ass", ts));
    
    // 執行彈幕轉換
    let danmaku_status = std::process::Command::new("danmaku2ass")
        .args([
            xml_path.to_str().unwrap_or_default(), 
            "-o", 
            ass_path.to_str().unwrap_or_default(),
            "-s", &resolution,
            "-fs", "25",
            "-dm", "15",
        ])
        .status();

    if danmaku_status.map_or(false, |s| s.success()) && ass_path.exists() {
        println!("🎬 正在將彈幕封裝至影片中...");
        
        // 🎯 核心修正：動態判斷字幕編碼
        let ext = final_path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
        let sub_codec = if ext == "mp4" {
            "mov_text" // MP4 只能用 mov_text
        } else {
            "ass"      // MKV 原生支援 ass，能完美保留彈幕顏色與軌跡！
        };

        let ffmpeg_status = std::process::Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y"])
            .args(["-i", main_file.to_str().unwrap_or_default()])
            .args(["-i", ass_path.to_str().unwrap_or_default()])
            .args(["-map", "0:v", "-map", "0:a", "-map", "1:s", "-c", "copy", "-c:s", sub_codec])
            .args(["-disposition:s:0", "default"]) // 強制預設開啟字幕軌
            .arg(final_path.to_str().unwrap_or_default())
            .status();

        // 🌟 關鍵新增：將 .ass 檔案複製到最終目錄，並將檔名改為與影片完全一致
        let final_ass_path = final_path.with_extension("ass");
        let _ = std::fs::copy(&ass_path, &final_ass_path);
        println!("📝 已生成外部彈幕字幕檔：{}", final_ass_path.file_name().unwrap_or_default().to_string_lossy());
        return ffmpeg_status.map_or(false, |s| s.success());
    }
    false
}

/// 輔助函式：在目錄中尋找符合特定條件的檔案
fn find_specific_file(dir: &Path, ts: &str, contains: &str, ends_with: &str) -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(&format!("tmp_{}", ts)) 
               && name.contains(contains) 
               && name.ends_with(ends_with) 
            {
                return Some(entry.path());
            }
        }
    }
    None
}

/// 針對音訊模式下載後的字幕進行重新命名與清理 (修正：接收 4 個參數以對接 utils.rs)
pub fn process_external_subtitles(
    tmp_dir: &std::path::Path, 
    ts: &str, 
    final_name: &str, 
    target_dir: &std::path::Path
) {
    if let Ok(entries) = std::fs::read_dir(tmp_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path.extension().unwrap_or_default().to_string_lossy();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();

            if file_name.starts_with(&format!("tmp_{}.", ts)) && (ext == "vtt" || ext == "srt") {
                let parts: Vec<&str> = file_name.split('.').collect();
                let lang = if parts.len() >= 3 { parts[parts.len() - 2] } else { "" };
                
                let base_name = final_name.rsplit_once('.').map(|(b, _)| b).unwrap_or(final_name);
                
                // 修正：將字幕從暫存區搬移到最終目標區
                let original_sub = target_dir.join(format!("{}.{}.{}", base_name, lang, ext));
                let clean_sub = target_dir.join(format!("{}_clean.{}.{}", base_name, lang, ext));

                if std::fs::rename(&path, &original_sub).is_ok() {
                    clean_vtt_file(&original_sub, &clean_sub);
                }
            }
        }
    }
}

/// 清洗 VTT 字幕中的 HTML 標記
fn clean_vtt_file(original_path: &Path, clean_path: &Path) {
    if let Ok(content) = fs::read_to_string(original_path) {
        let re = VTT_REGEX.get_or_init(|| Regex::new(r"<\/?c[^>]*>").unwrap());
        let cleaned_content = re.replace_all(&content, "");
        let _ = fs::write(clean_path, cleaned_content.as_ref());
    }
}

/// 透過 ffprobe 獲取影片解析度
pub fn get_video_resolution(file_path: &Path) -> Option<String> {
    let output = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=s=x:p=0", file_path.to_str().unwrap_or_default()])
        .output()
        .ok()?;

    if output.status.success() {
        let res = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !res.is_empty() { return Some(res); }
    }
    None
}

/// 清理所有剩餘的 tmp_ 暫存檔案
pub fn cleanup_tmps(target_dir: &Path, ts: &str) {
    if let Ok(entries) = fs::read_dir(target_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.file_name().map_or(false, |n| n.to_string_lossy().starts_with(&format!("tmp_{}", ts))) {
                let _ = fs::remove_file(path);
            }
        }
    }
}