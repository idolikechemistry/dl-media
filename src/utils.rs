use crate::parser::{self, VideoItem};
use crate::processor;
use chrono::Local;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

/// 構建 yt-dlp 的下載指令參數 (不含輸出路徑與網址)
pub fn build_download_args(media_type: u8, target_ext: &str, input_url: &str, cookie_args: &[String]) -> Vec<String> {
    let mut dl_args: Vec<String> = vec![
        "--quiet".into(), "--progress".into(), "--no-warnings".into(), "--ignore-errors".into(),
        "--no-overwrites".into(), "--embed-thumbnail".into(), "--embed-metadata".into(),
        "--embed-chapters".into(), "--convert-thumbnails".into(), "jpg".into(), "--restrict-filenames".into(),
        "--sponsorblock-remove".into(), "sponsor,intro,outro".into(),
    ];

    // 🎯 影片模式 (2 或 3)：預設全自動抓取所有字幕與彈幕並準備嵌入
    if media_type != 1 {
        let is_bilibili = input_url.contains("bilibili.com") || input_url.contains("b23.tv");
        
        if is_bilibili || parser::has_subtitles(input_url, cookie_args) {
            let mut sub_args = vec!["--write-subs".into(), "--write-auto-subs".into()];
            
            // 🛑 若不是 B 站，讓 yt-dlp 自己 embed；B 站則交給我們的 danmaku2ass 處理
            if !is_bilibili {
                sub_args.push("--embed-subs".into());
            }
            
            dl_args.extend(sub_args);
            dl_args.extend(vec!["--sub-langs".into(), "zh-Hant,zh-TW,zh-HK,zh-Hans,zh,en,ja,danmaku".into()]);
        }
    }

    if media_type == 1 {
        // 音訊模式
        dl_args.extend(vec!["--extract-audio".into(), "--audio-format".into(), target_ext.into()]);
        if target_ext == "mp3" { 
            dl_args.extend(vec!["--audio-quality".into(), "320k".into(), "-f".into(), "bestaudio".into()]); 
        } else { 
            dl_args.extend(vec!["-f".into(), "bestaudio[ext=m4a]/bestaudio".into()]); 
        }
    } else {
        // 影片模式
        dl_args.extend(vec!["--merge-output-format".into(), target_ext.into()]);
        if target_ext == "mkv" { 
            dl_args.extend(vec!["-f".into(), "bv*+ba/best".into()]); 
        } else { 
            dl_args.extend(vec!["-f".into(), "bv*[vcodec^=avc]+ba[ext=m4a]/best[ext=mp4]/best".into()]); 
        }
    }
    dl_args
}

/// 準備最終存檔資料夾 (處理播放清單子目錄邏輯)
pub fn prepare_output_dir(base_dir: &PathBuf, input_url: &str, cookie_args: &[String], is_pl: bool) -> PathBuf {
    let mut dir = base_dir.clone();
    let _ = fs::create_dir_all(&dir);
    
    if is_pl {
        // 如果是播放清單，嘗試抓取清單名稱作為子資料夾
        let title = Command::new("yt-dlp")
            .args(cookie_args)
            .args(["--print", "playlist_title", "--no-warnings", "--playlist-items", "1", input_url])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|t| !t.is_empty() && t != "NA" && t != "null")
            .unwrap_or_else(|| "Playlist".into());
            
        // 清理非法字元
        dir = dir.join(title.replace(&['/', '\\', ':', '*', '?', '"', '<', '>', '|'][..], "_"));
        let _ = fs::create_dir_all(&dir);
    }
    dir
}

/// 下載執行核心迴圈
pub fn execute_download_loop(
    valid_videos: Vec<VideoItem>,
    is_playlist: bool,
    media_type: u8,
    target_ext: &str,
    dl_args: Vec<String>,
    cookie_args: Vec<String>,
    target_dir: PathBuf, // 最終存檔目錄
    tmp_dir: PathBuf,    // 暫存加工目錄
    force_cookie: bool,
) -> anyhow::Result<()> {
    let mut success_count = 0;
    let mut fail_count = 0;
    let start_time = Instant::now();
    let total = valid_videos.len();

    // 確保暫存目錄存在
    let _ = fs::create_dir_all(&tmp_dir);

    for (idx, video) in valid_videos.iter().enumerate() {
        let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let safe_title = video.title.replace(&['/', '\\', ':', '*', '?', '"', '<', '>', '|'][..], "_");
        
        let final_name = if is_playlist {
            format!("{:02}-{}_{}.{}", idx + 1, safe_title, ts, target_ext)
        } else {
            format!("{}_{}.{}", safe_title, ts, target_ext)
        };
        let final_path = target_dir.join(&final_name);

        println!("=================================================");
        println!("🎬 準備下載 ({}/{}): {}", idx + 1, total, video.title);

        let mut current_dl_args = dl_args.clone();

        // 🎯 核心修改：純音訊模式在此啟動 TUI 歌詞選單
        if media_type == 1 {
            println!("🔍 正在掃描可用歌詞/字幕...");
            let avai_subs = parser::get_available_subtitles(&video.url, &cookie_args);
            let chosen_langs = crate::ui::select_subtitles(&avai_subs);
            
            if !chosen_langs.is_empty() {
                current_dl_args.push("--write-subs".into());
                current_dl_args.push("--write-auto-subs".into());
                current_dl_args.push("--sub-langs".into());
                current_dl_args.push(chosen_langs.join(","));
            } else {
                println!("📌 未選擇任何歌詞，將進行純音訊下載。");
            }
        }

        // 將下載輸出模板導向 tmp_dir
        let tmp_output_template = format!("{}/tmp_{}.%(ext)s", tmp_dir.to_string_lossy(), ts);
        current_dl_args.push("-o".into());
        current_dl_args.push(tmp_output_template);
        current_dl_args.push(video.url.clone());

        let status = Command::new("yt-dlp")
            .args(&cookie_args)
            .args(&current_dl_args)
            .status();

        if status.map_or(false, |s| s.success()) {
            // 1. 在暫存目錄中尋找下載好的主檔案
            if let Some(downloaded_file) = processor::find_main_file(&tmp_dir, &ts) {
                
                // 2. 進行後製加工 (若為音訊模式，則跳過合併邏輯)
                let merged = if media_type != 1 {
                    processor::handle_danmaku_merge(&tmp_dir, &ts, &downloaded_file, &final_path)
                } else {
                    false
                };

                // 3. 如果沒有觸發封裝 (或是純音訊)，則直接將主檔案搬移到最終存檔目錄
                if !merged {
                    let _ = fs::rename(&downloaded_file, &final_path);
                }

                // 4. 處理外部字幕 (重新命名、清洗並移至存檔目錄)
                processor::process_external_subtitles(&tmp_dir, &ts, &final_name, &target_dir);

                // 5. 獲取畫質資訊 (純音訊不顯示畫質)
                let res_info = if media_type != 1 {
                    processor::get_video_resolution(&final_path).map_or("".into(), |r| format!(" [畫質: {}]", r))
                } else {
                    "".into()
                };

                success_count += 1;
                println!("✅ 儲存成功：{}{}", final_name, res_info);
            } else {
                fail_count += 1;
                println!("❌ 錯誤：在暫存區找不到下載完成的檔案。");
            }

            // 6. 清理該次下載產生的所有暫存檔案
            processor::cleanup_tmps(&tmp_dir, &ts);

        } else {
            fail_count += 1;
            println!("⚠️ yt-dlp 下載失敗。");
        }
    }

    let duration = start_time.elapsed();
    let time_str = format!("{} 分 {} 秒", duration.as_secs() / 60, duration.as_secs() % 60);

    crate::ui::print_summary(success_count, fail_count, &time_str, &target_dir.to_string_lossy());
    
    if fail_count > 0 && !force_cookie {
        println!("💡 提示：若下載失敗，請嘗試使用 --fc 參數來套用您的 Cookie 設定！");
    }

    Ok(())
}