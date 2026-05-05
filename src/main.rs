// 1. 宣告所有模組 (這告訴編譯器去尋找對應的 .rs 檔案)
mod args;
mod config;
mod parser;
mod processor;
mod setup;
mod ui;
mod utils;

// 2. 引入必要的工具與型別
use anyhow::{Context, Result}; // 使用 anyhow 的 Result，它只需要一個參數 <()>
use args::Args;               // 引入你定義的參數結構
use clap::Parser;             // 引入 clap 的解析功能
use std::path::PathBuf;       // 引入路徑處理工具
use std::process;             // 引入系統程序控制

// 3. 程式進入點 (大腦)
fn main() {
    // 執行 run 函式，如果出錯則印出錯誤訊息並優雅退出
    println!("🚀 dl-media v{}", env!("CARGO_PKG_VERSION"));
    if let Err(e) = run() {
        eprintln!("\n❌ [執行錯誤]: {}", e);
        for cause in e.chain().skip(1) {
            eprintln!("   原因: {}", cause);
        }
        process::exit(1);
    }
}

// 4. 核心邏輯流程
fn run() -> Result<()> {
    // 解析命令列參數
    let args = Args::parse();
    args.validate()?;

    // 初始化環境與讀取設定檔
    let (app_config_dir, config) = setup::init_config()?;
    let config_file_path = app_config_dir.join("config.toml");

    // 如果使用者下了 --open-config，進入互動設定介面
    if args.open_config {
        setup::interactive_config_setup(&config_file_path, config)?;
        println!("👋 設定已完成，您可以重新執行程式來套用新設定。");
        return Ok(());
    }

    // 檢查系統依賴工具 (yt-dlp, ffmpeg 等)
    setup::check_dependencies()?;

    // --- 三層優先級路徑解析 ---
    
    // 下載路徑優先級：CLI (-o) > Config.toml > 系統 Downloads
    let final_download_dir = args.output.as_ref()
        .map(PathBuf::from)
        .or_else(|| config.download_dir.as_ref().map(PathBuf::from))
        .unwrap_or_else(|| dirs::download_dir().expect("找不到系統下載目錄"));

    // 暫存路徑優先級：Config.toml (tmp_dir) > final_download_dir
    let final_tmp_dir = config.tmp_dir.as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| final_download_dir.clone());

    // Cookie 目錄優先級：Config.toml (cookie_dir) > App 設定資料夾
    let resolved_cookie_dir = config.cookie_dir.as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| app_config_dir.clone());

    // 處理 UI 互動 (如果沒給參數)
    let is_silent = args.is_fully_automated();
    let (input_url, media_type, target_ext) = if is_silent {
        (
            args.url.clone().unwrap(), 
            args.media_type.unwrap() as u8, 
            args.format.clone().unwrap().to_lowercase()
        )
    } else {
        ui::get_user_input(&args).context("無法取得使用者輸入")?
    };

    // 網址情報分析
    let site_target = parser::extract_site_name(&input_url);
    let (mut valid_videos, is_playlist, has_restricted) = parser::scan_url(&input_url, args.force_cookie, &site_target)?;

    // 處理 Cookie 套用
    let cookie_args = setup::handle_cookies(
        &site_target, 
        has_restricted, 
        &args.cookie, 
        &resolved_cookie_dir, 
        is_silent
    )?;

    // 如果是播放清單且有 Cookie，嘗試重新掃描是否有解鎖內容
    if !cookie_args.is_empty() && is_playlist {
        valid_videos = parser::rescan_with_cookies(&input_url, &cookie_args, valid_videos.len())?;
    }

    // 準備最終存檔資料夾與下載參數
    let final_target_dir = utils::prepare_output_dir(&final_download_dir, &input_url, &cookie_args, is_playlist);
    let dl_args = utils::build_download_args(media_type, &target_ext, &input_url, &cookie_args);

    // 啟動下載迴圈 (傳入雙路徑：存檔 vs 暫存)
    utils::execute_download_loop(
        valid_videos,
        is_playlist,
        media_type,
        &target_ext,
        dl_args,
        cookie_args,
        final_target_dir,
        final_tmp_dir,
        args.force_cookie,
    )?;

    Ok(())
}