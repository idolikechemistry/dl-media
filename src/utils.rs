use crate::parser::{self, VideoItem};
use crate::processor;
use chrono::Local;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Command as AsyncCommand;
use tokio::sync::Semaphore;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};

/// 構建 yt-dlp 的下載指令參數
pub fn build_download_args(
    media_type: u8,
    target_ext: &str,
    _input_url: &str,
    _cookie_args: &[String],
) -> Vec<String> {
    let mut dl_args: Vec<String> = vec![
        "--newline".into(),
        "--progress".into(),
        "--no-colors".into(),
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

pub fn prepare_output_dir(
    base_dir: &PathBuf,
    input_url: &str,
    cookie_args: &[String],
    is_pl: bool,
) -> PathBuf {
    let mut dir = base_dir.clone();
    let _ = fs::create_dir_all(&dir);

    if is_pl {
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

pub async fn execute_download_loop(
    valid_videos: Vec<VideoItem>,
    is_playlist: bool,
    media_type: u8,
    target_ext: String,
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

    let semaphore = Arc::new(Semaphore::new(max_concurrent as usize));
    let mut handles = vec![];
    let multi_progress = Arc::new(MultiProgress::new());

    for (idx, video) in valid_videos.into_iter().enumerate() {
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        let target_ext = target_ext.clone();
        let mut current_dl_args = dl_args.clone();
        let cookie_args = cookie_args.clone();
        let target_dir = target_dir.clone();
        let tmp_dir = tmp_dir.clone();
        let multi_progress = multi_progress.clone();

        let handle = tokio::spawn(async move {
            let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();
            let pid = std::process::id();
            let session_id = format!("{}_pid{}_{}", ts, pid, idx);
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

            let pb = multi_progress.add(ProgressBar::new(100));
            pb.set_style(
                ProgressStyle::with_template(
                    "{prefix:.bold.dim} {spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {msg}"
                )
                .unwrap()
                .progress_chars("#>-"),
            );
            pb.set_prefix(format!("[{}/{}]", idx + 1, total));
            pb.set_message("準備下載...");

            if max_concurrent == 1 {
                let probe_result = match parser::probe_video_info(&video.url, &cookie_args) {
                    Ok(info) => info,
                    Err(e) => {
                        pb.println(format!("⚠️ 無法取得影片資訊，將使用預設參數：{}", e));
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
                        pb.println("📌 採用 MP4：將自動下載最高 1080p 相容畫質。");
                    }
                }
            } else {
                if media_type != 1 && target_ext == "mp4" {
                    pb.println("📌 採用 MP4：自動下載最高相容畫質。");
                }
            }

            let tmp_output_template =
                format!("{}/tmp_{}.%(ext)s", session_tmp_dir.to_string_lossy(), ts);
            current_dl_args.push("-o".into());
            current_dl_args.push(tmp_output_template);
            current_dl_args.push(video.url.clone());

            let mut child = AsyncCommand::new("yt-dlp")
                .current_dir(&session_tmp_dir)
                .args(&cookie_args)
                .args(&current_dl_args)
                .stdout(Stdio::piped())
                .spawn()
                .expect("執行 yt-dlp 失敗");

            if let Some(stdout) = child.stdout.take() {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if line.contains("[download]") && line.contains("%") {
                        if let Some(pct_str) = line.split_whitespace().find(|s| s.contains("%")) {
                            if let Ok(pct) = pct_str.replace('%', "").parse::<f32>() {
                                pb.set_position(pct as u64);
                            }
                        }
                        pb.set_message(line.replace("[download]", "").trim().to_string());
                    }
                }
            }

            let status = child
                .wait()
                .await
                .unwrap_or_else(|_| panic!("等待 yt-dlp 失敗"));
            let mut success = false;
            let mut downloaded_path_str = String::new();

            if status.success() {
                if let Ok(entries) = fs::read_dir(&session_tmp_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let file_name = path.file_name().unwrap_or_default().to_string_lossy();

                        if file_name.starts_with(&format!("tmp_{}", ts))
                            && !file_name.ends_with(".vtt")
                            && !file_name.ends_with(".ass")
                            && !file_name.ends_with(".srt")
                        {
                            downloaded_path_str = path.to_string_lossy().to_string();
                            break;
                        }
                    }
                }
            }

            let mut final_res_info = String::new();

            if status.success() && !downloaded_path_str.is_empty() {
                let downloaded_file = PathBuf::from(downloaded_path_str);

                processor::process_external_subtitles(
                    &session_tmp_dir,
                    &ts,
                    &final_name,
                    &target_dir,
                    media_type,
                );

                pb.set_position(0);
                pb.set_message("正在執行封裝...");

                let merged = if media_type != 1 {
                    processor::merge_subs_and_danmaku(
                        &session_tmp_dir,
                        &ts,
                        &downloaded_file,
                        &final_path,
                        pb.clone(),
                    )
                    .await
                } else {
                    false
                };

                if !merged {
                    let _ = fs::rename(&downloaded_file, &final_path);
                    pb.set_position(100);
                }

                if media_type != 1 {
                    final_res_info = processor::get_video_resolution(&final_path)
                        .map_or("".into(), |r| format!(" [畫質: {}]", r));
                }

                // 🎯 核心修正：先安全地印出成功文字，然後讓進度條徹底消失
                pb.println(format!("✅ 儲存成功：{}{}", final_name, final_res_info));
                pb.finish_and_clear();

                success = true;
            } else {
                // 🎯 核心修正：錯誤時也是先印文字，然後清除進度條
                pb.println(format!("⚠️ 下載失敗：{}", video.title));
                pb.finish_and_clear();
            }

            processor::cleanup_tmps(&session_tmp_dir);
            drop(permit);

            success
        });

        handles.push(handle);
    }

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
