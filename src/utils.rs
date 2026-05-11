use crate::parser::{self, VideoItem};
use crate::processor;
use chrono::Local;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Command as AsyncCommand; // 🎯 引入非同步 Command
use tokio::sync::Semaphore; // 🎯 引入信號量控制並行數

/// 構建 yt-dlp 的下載指令參數 (不含輸出路徑、網址與動態字幕)
pub fn build_download_args(
    media_type: u8,
    target_ext: &str,
    _input_url: &str,
    _cookie_args: &[String],
) -> Vec<String> {
    let mut dl_args: Vec<String> = vec![
        "--quiet".into(),
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
        // 🎯 核心修改：要求 yt-dlp 成功後只打印出最終移動後的絕對路徑
        "--print".into(),
        "after_move:filepath".into(),
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
        // 取得播放清單標題 (此處維持同步即可，因為只執行一次)
        let title = std::process::Command::new("yt-dlp")
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

/// 🎯 核心修改：非同步下載執行核心迴圈
pub async fn execute_download_loop(
    valid_videos: Vec<VideoItem>,
    is_playlist: bool,
    media_type: u8,
    target_ext: String, // 改為 String 以利跨執行緒轉移
    dl_args: Vec<String>,
    cookie_args: Vec<String>,
    target_dir: PathBuf,
    tmp_dir: PathBuf,
    force_cookie: bool,
    max_concurrent: u32,
) -> anyhow::Result<()> {
    let start_time = Instant::now();
    let total = valid_videos.len();

    let _ = fs::create_dir_all(&tmp_dir);

    // 🎯 建立信號量以限制最大並行數
    let semaphore = Arc::new(Semaphore::new(max_concurrent as usize));
    let mut handles = vec![];

    for (idx, video) in valid_videos.into_iter().enumerate() {
        // 獲取信號量許可證，若超過並行數則會在此非同步等待
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        // 複製必要的變數給非同步任務使用
        let target_ext = target_ext.clone();
        let mut current_dl_args = dl_args.clone();
        let cookie_args = cookie_args.clone();
        let target_dir = target_dir.clone();
        let tmp_dir = tmp_dir.clone();

        // 🎯 派生非同步任務
        let handle = tokio::spawn(async move {
            let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();
            let pid = std::process::id();
            let session_id = format!("{}_pid{}_{}", ts, pid, idx); // 加上 idx 確保完全唯一
            let session_tmp_dir = tmp_dir.join(&session_id);
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

            println!("🎬 開始處理 ({}/{}): {}", idx + 1, total, video.title);

            // 🎯 互動防護機制：只有在單任務執行時，才觸發 UI 選單
            if max_concurrent == 1 {
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

                let chosen_langs = crate::ui::select_subtitles(&probe_result.langs);
                if !chosen_langs.is_empty() {
                    current_dl_args.push("--write-subs".into());
                    current_dl_args.push("--write-auto-subs".into());
                    current_dl_args.push("--sub-langs".into());
                    current_dl_args.push(chosen_langs.join(","));
                }

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
            } else {
                // 並行模式下，若為影片預設印出提示即可
                if media_type != 1 && target_ext == "mp4" {
                    println!("📌 採用 MP4 安全模式：自動下載最高相容畫質。");
                }
            }

            let tmp_output_template =
                format!("{}/tmp_{}.%(ext)s", session_tmp_dir.to_string_lossy(), ts);
            current_dl_args.push("-o".into());
            current_dl_args.push(tmp_output_template);
            current_dl_args.push(video.url.clone());

            // 🎯 非同步執行 yt-dlp
            let output = AsyncCommand::new("yt-dlp")
                .current_dir(&session_tmp_dir)
                .args(&cookie_args)
                .args(&current_dl_args)
                .output()
                .await
                .expect("執行 yt-dlp 失敗");

            let mut success = false;
            let mut final_res_info = String::new();

            if output.status.success() {
                let stdout_str = String::from_utf8_lossy(&output.stdout);

                // 🎯 擷取絕對路徑：從 stdout 抓取最後一行非空的字串
                if let Some(downloaded_path_str) =
                    stdout_str.lines().filter(|l| !l.trim().is_empty()).last()
                {
                    let downloaded_file = PathBuf::from(downloaded_path_str.trim());

                    if downloaded_file.exists() {
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

                        if media_type != 1 {
                            final_res_info = processor::get_video_resolution(&final_path)
                                .map_or("".into(), |r| format!(" [畫質: {}]", r));
                        }

                        println!("✅ 儲存成功：{}{}", final_name, final_res_info);
                        success = true;
                    } else {
                        println!(
                            "❌ 錯誤：雖然 yt-dlp 回報成功，但找不到目標檔案 ({:?})",
                            downloaded_file
                        );
                    }
                } else {
                    println!("❌ 錯誤：無法從 yt-dlp 輸出中解析絕對路徑。");
                }
            } else {
                println!("⚠️ 下載失敗：{}", video.title);
            }

            // 4. 清理暫存區並釋放信號量
            processor::cleanup_tmps(&session_tmp_dir);
            drop(permit);

            success
        });

        handles.push(handle);
    }

    // 等待所有非同步任務完成並統計結果
    let mut success_count = 0;
    let mut fail_count = 0;

    for handle in handles {
        match handle.await {
            Ok(true) => success_count += 1,
            Ok(false) => fail_count += 1,
            Err(_) => fail_count += 1,
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
