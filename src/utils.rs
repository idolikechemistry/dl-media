use crate::parser::{self, VideoItem};
use crate::processor;
use chrono::Local;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

/// 構建 yt-dlp 的下載指令參數 (不含輸出路徑、網址與動態字幕)
pub fn build_download_args(
    media_type: u8,
    target_ext: &str,
    _input_url: &str,
    _cookie_args: &[String],
) -> Vec<String> {
    let mut dl_args: Vec<String> = vec![
        "--quiet".into(),
        "--progress".into(),
        "--no-warnings".into(),
        "--ignore-errors".into(),
        "--no-overwrites".into(),
        "--embed-thumbnail".into(),
        "--embed-metadata".into(),
        "--embed-chapters".into(),
        "--convert-thumbnails".into(),
        "jpg".into(),
        "--restrict-filenames".into(),
        "--sponsorblock-remove".into(),
        "sponsor,intro,outro".into(),
    ];

    if media_type == 1 {
        // 音訊模式
        dl_args.extend(vec![
            "--extract-audio".into(),
            "--audio-format".into(),
            target_ext.into(),
        ]);
        if target_ext == "mp3" {
            dl_args.extend(vec![
                "--audio-quality".into(),
                "320k".into(),
                "-f".into(),
                "bestaudio".into(),
            ]);
        } else {
            dl_args.extend(vec!["-f".into(), "bestaudio[ext=m4a]/bestaudio".into()]);
        }
    } else {
        // 影片模式
        dl_args.extend(vec!["--merge-output-format".into(), target_ext.into()]);
        if target_ext == "mkv" {
            dl_args.extend(vec!["-f".into(), "bv*+ba/best".into()]);
        } else {
            dl_args.extend(vec![
                "-f".into(),
                "bv*[vcodec^=avc]+ba[ext=m4a]/best[ext=mp4]/best".into(),
            ]);
        }
    }
    dl_args
}

/// 準備最終存檔資料夾
pub fn prepare_output_dir(
    base_dir: &PathBuf,
    input_url: &str,
    cookie_args: &[String],
    is_pl: bool,
) -> PathBuf {
    let mut dir = base_dir.clone();
    let _ = fs::create_dir_all(&dir);

    if is_pl {
        let title = Command::new("yt-dlp")
            .args(cookie_args)
            .args([
                "--print",
                "playlist_title",
                "--no-warnings",
                "--playlist-items",
                "1",
                "--skip-download",
                input_url,
            ])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|t| !t.is_empty() && t != "NA" && t != "null")
            .unwrap_or_else(|| "Playlist".into());

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
    target_dir: PathBuf,
    tmp_dir: PathBuf,
    force_cookie: bool,
) -> anyhow::Result<()> {
    let mut success_count = 0;
    let mut fail_count = 0;
    let start_time = Instant::now();
    let total = valid_videos.len();

    let _ = fs::create_dir_all(&tmp_dir);

    for (idx, video) in valid_videos.iter().enumerate() {
        let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let pid = std::process::id(); // 取得 OS 分配的唯一 ID
        let session_id = format!("{}_pid{}", ts, pid);
        let session_tmp_dir = tmp_dir.join(&session_id); // 建立專屬沙盒路徑
        let _ = fs::create_dir_all(&session_tmp_dir);

        let safe_title = video
            .title
            .replace(&['/', '\\', ':', '*', '?', '"', '<', '>', '|'][..], "_");

        let final_name = if is_playlist {
            format!("{:02}-{}_{}.{}", idx + 1, safe_title, ts, target_ext)
        } else {
            format!("{}_{}.{}", safe_title, ts, target_ext)
        };
        let final_path = target_dir.join(&final_name);

        println!("=================================================");
        println!("🎬 準備下載 ({}/{}): {}", idx + 1, total, video.title);

        let mut current_dl_args = dl_args.clone();

        // 🎯 統一探測：同時掃描可用字幕與畫質
        println!("🔍 正在掃描可用資源 (畫質/字幕/彈幕)...");

        // 使用 match 避免單一影片探測失敗導致整個清單中斷
        let probe_result = match parser::probe_video_info(&video.url, &cookie_args) {
            Ok(info) => info,
            Err(e) => {
                println!("⚠️ 無法取得影片資訊，將使用預設參數：{}", e);
                crate::parser::VideoInfo {
                    langs: vec![],
                    formats: vec![],
                }
            }
        };

        // 1. 字幕選擇邏輯
        let chosen_langs = crate::ui::select_subtitles(&probe_result.langs);

        if !chosen_langs.is_empty() {
            current_dl_args.push("--write-subs".into());
            current_dl_args.push("--write-auto-subs".into());
            current_dl_args.push("--sub-langs".into());
            current_dl_args.push(chosen_langs.join(","));
        } else {
            println!("📌 未選擇任何額外字幕軌道。");
        }

        // 2. 🎯 雙軌畫質分流邏輯
        if media_type != 1 {
            if target_ext == "mkv" {
                if let Some(vid_id) = crate::ui::select_resolution(&probe_result.formats) {
                    if let Some(f_idx) = current_dl_args.iter().position(|x| x == "-f") {
                        current_dl_args[f_idx + 1] = format!("{}+bestaudio/best", vid_id);
                    }
                }
            } else if target_ext == "mp4" {
                println!("📌 採用 MP4 安全模式：將自動下載最高 1080p 相容畫質。");
            }
        }

        let tmp_output_template =
            format!("{}/tmp_{}.%(ext)s", session_tmp_dir.to_string_lossy(), ts);
        current_dl_args.push("-o".into());
        current_dl_args.push(tmp_output_template);
        current_dl_args.push(video.url.clone());

        let status = Command::new("yt-dlp")
            .current_dir(&session_tmp_dir) // 🔒 將工作目錄鎖定在沙盒內
            .args(&cookie_args)
            .args(&current_dl_args)
            .status();

        if status.map_or(false, |s| s.success()) {
            if let Some(downloaded_file) = processor::find_main_file(&session_tmp_dir, &ts) {
                // 1. 處理字幕清洗
                processor::process_external_subtitles(
                    &session_tmp_dir,
                    &ts,
                    &final_name,
                    &target_dir,
                    media_type,
                );

                // 2. 處理影片封裝
                let merged = if media_type != 1 {
                    processor::merge_subs_and_danmaku(
                        &session_tmp_dir,
                        &ts,
                        &downloaded_file,
                        &final_path,
                    )
                } else {
                    false
                };

                // 3. 若沒有任何字幕/彈幕需要封裝，直接搬運主檔案
                if !merged {
                    let _ = fs::rename(&downloaded_file, &final_path);
                }

                let res_info = if media_type != 1 {
                    processor::get_video_resolution(&final_path)
                        .map_or("".into(), |r| format!(" [畫質: {}]", r))
                } else {
                    "".into()
                };

                success_count += 1;
                println!("✅ 儲存成功：{}{}", final_name, res_info);
            } else {
                fail_count += 1;
                println!("❌ 錯誤：在暫存區找不到下載完成的檔案。");
            }

            // 4. 清理暫存區
            processor::cleanup_tmps(&session_tmp_dir);
        } else {
            fail_count += 1;
            println!("⚠️ yt-dlp 下載失敗。");
        }
    }

    let duration = start_time.elapsed();
    let time_str = format!(
        "{} 分 {} 秒",
        duration.as_secs() / 60,
        duration.as_secs() % 60
    );

    crate::ui::print_summary(
        success_count,
        fail_count,
        &time_str,
        &target_dir.to_string_lossy(),
    );

    if fail_count > 0 && !force_cookie {
        println!("💡 提示：若下載失敗，請嘗試使用 --fc 參數來套用您的 Cookie 設定！");
    }

    Ok(())
}
