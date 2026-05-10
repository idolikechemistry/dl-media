use anyhow::{Context, Result};
use serde_json::Value;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct VideoItem {
    pub title: String,
    pub url: String,
}

// 🎯 新增：畫質格式結構
#[derive(Debug, Clone)]
pub struct VideoFormat {
    pub format_id: String,
    pub height: u32,
    pub vcodec: String,
    pub ext: String,
}

// 🎯 新增：整合後的探測結果結構
pub struct VideoInfo {
    pub langs: Vec<String>,
    pub formats: Vec<VideoFormat>,
}

pub fn extract_site_name(url: &str) -> String {
    let url_lower = url.to_lowercase();
    if url_lower.contains("youtube.com")
        || url_lower.contains("youtu.be")
        || url_lower.contains("googleusercontent.com")
    {
        return "youtube".into();
    }
    if url_lower.contains("bilibili.com") || url_lower.contains("b23.tv") {
        return "bilibili".into();
    }
    if url_lower.contains("twitter.com") || url_lower.contains("x.com") {
        return "twitter".into();
    }
    if url_lower.contains("facebook.com") || url_lower.contains("fb.watch") {
        return "facebook".into();
    }
    if url_lower.contains("instagram.com") {
        return "instagram".into();
    }

    url_lower
        .split('/')
        .nth(2)
        .and_then(|d| d.split('.').rev().nth(1))
        .unwrap_or("unknown")
        .to_string()
}

pub fn scan_url(
    input_url: &str,
    force_cookie: bool,
    site_target: &str,
) -> Result<(Vec<VideoItem>, bool, bool)> {
    println!("🔍 正在分析網址資訊...");

    let output = Command::new("yt-dlp")
        .args([
            "--flat-playlist",
            "--skip-download",
            "--print",
            "playlist:%(playlist_title)s",
            "--print",
            "item:%(title)s|%(webpage_url)s",
            "--ignore-errors",
            "--no-warnings",
            input_url,
        ])
        .output()
        .context("執行 yt-dlp 解析清單失敗")?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr).to_lowercase();

    let mut valid_videos = Vec::new();
    let mut is_playlist = false;

    let mut has_restricted = force_cookie
        || stderr_str.contains("sign in")
        || stderr_str.contains("login")
        || stderr_str.contains("cookie")
        || stderr_str.contains("登錄")
        || stderr_str.contains("private");

    for line in stdout_str.lines() {
        if let Some(pl_title) = line.strip_prefix("playlist:") {
            if pl_title != "NA" && !pl_title.is_empty() && pl_title != "null" {
                is_playlist = true;
            }
        } else if let Some(item) = line.strip_prefix("item:") {
            if item.contains("[Private video]")
                || item.contains("[Deleted video]")
                || item.contains("Private")
            {
                has_restricted = true;
            } else if let Some((title, url)) = item.rsplit_once('|') {
                valid_videos.push(VideoItem {
                    title: title.to_string(),
                    url: url.to_string(),
                });
            }
        }
    }

    if valid_videos.is_empty() {
        valid_videos.push(VideoItem {
            title: "Video".to_string(),
            url: input_url.to_string(),
        });
    }

    if site_target == "bilibili" {
        has_restricted = true;
    }

    print_analysis_report(
        site_target,
        is_playlist,
        valid_videos.len(),
        has_restricted,
        force_cookie,
    );

    Ok((valid_videos, is_playlist, has_restricted))
}

pub fn rescan_with_cookies(
    input_url: &str,
    cookie_args: &[String],
    original_total: usize,
) -> Result<Vec<VideoItem>> {
    println!("🔄 正在透過 Cookie 驗證並重新掃描清單...");

    let output = Command::new("yt-dlp")
        .args(cookie_args)
        .args([
            "--flat-playlist",
            "--skip-download",
            "--print",
            "item:%(title)s|%(webpage_url)s",
            "--ignore-errors",
            "--no-warnings",
            input_url,
        ])
        .output()
        .context("透過 Cookie 重新解析清單失敗")?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut new_videos = Vec::new();

    for line in stdout_str.lines() {
        if let Some(item) = line.strip_prefix("item:") {
            if item.contains("[Private video]") || item.contains("[Deleted video]") {
                continue;
            }
            if let Some((title, url)) = item.rsplit_once('|') {
                new_videos.push(VideoItem {
                    title: title.to_string(),
                    url: url.to_string(),
                });
            }
        }
    }

    if new_videos.is_empty() {
        new_videos.push(VideoItem {
            title: "Video".to_string(),
            url: input_url.to_string(),
        });
    }

    let new_total = new_videos.len();
    if new_total > original_total {
        println!("--------------------------------------------------");
        println!(
            "🔓 解鎖成功！透過 Cookie 發現了 {} 部隱藏/會員專屬內容。",
            new_total - original_total
        );
        println!("--------------------------------------------------");
    }

    Ok(new_videos)
}

fn print_analysis_report(site: &str, is_pl: bool, count: usize, restricted: bool, forced: bool) {
    println!("--------------------------------------------------");
    println!("📡 來源網站：{}", site);
    println!(
        "📋 內容類型：{}",
        if is_pl {
            format!("【播放清單】(包含 {} 部內容)", count)
        } else {
            "【單一內容】".into()
        }
    );

    let status = if forced {
        "⚠️ 強制調用 Cookie 模式"
    } else if restricted {
        "⚠️ 偵測到限制內容 (需 Cookie)"
    } else {
        "🔓 公開內容"
    };
    println!("🔒 權限狀態：{}", status);
    println!("--------------------------------------------------");
}

// 🎯 核心修改：合併探測字幕與畫質
pub fn probe_video_info(url: &str, cookie_args: &[String]) -> Result<VideoInfo> {
    let is_bilibili = url.contains("bilibili.com") || url.contains("b23.tv");

    let output = Command::new("yt-dlp")
        .args(cookie_args)
        .args(["--dump-json", "--no-warnings", "--skip-download", url])
        .output()
        .context("無法獲取影片 Metadata")?;

    let mut langs = Vec::new();
    let mut formats = Vec::new();

    if let Ok(json) = serde_json::from_slice::<Value>(&output.stdout) {
        // 1. 抓取字幕
        for sub_type in ["subtitles", "automatic_captions"] {
            if let Some(subs) = json.get(sub_type).and_then(|s| s.as_object()) {
                for lang in subs.keys() {
                    langs.push(lang.clone());
                }
            }
        }

        // 2. 抓取影片格式
        if let Some(fmts) = json.get("formats").and_then(|f| f.as_array()) {
            for f in fmts {
                let vcodec = f.get("vcodec").and_then(|v| v.as_str()).unwrap_or("none");
                let height = f.get("height").and_then(|h| h.as_u64());
                let ext = f.get("ext").and_then(|e| e.as_str()).unwrap_or("");

                if vcodec != "none" && height.is_some() && ext != "mhtml" {
                    formats.push(VideoFormat {
                        format_id: f.get("format_id").unwrap().as_str().unwrap().to_string(),
                        height: height.unwrap() as u32,
                        vcodec: vcodec.to_string(),
                        ext: ext.to_string(),
                    });
                }
            }
        }
    }

    if is_bilibili {
        langs.push("danmaku".into());
    }
    langs.sort();
    langs.dedup();

    Ok(VideoInfo { langs, formats })
}
