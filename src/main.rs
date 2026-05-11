mod args;
mod config;
mod parser;
mod processor;
mod setup;
mod ui;
mod utils;

use anyhow::{Context, Result};
use args::Args;
use clap::{CommandFactory, Parser}; // 🎯 引入 CommandFactory 給補全使用
use std::path::PathBuf;
use std::process;

// 🎯 新增 Tokio 運行時
#[tokio::main]
async fn main() {
    println!("🚀 dl-media v{}", env!("CARGO_PKG_VERSION"));
    if let Err(e) = run().await {
        eprintln!("\n❌ [執行錯誤]: {}", e);
        for cause in e.chain().skip(1) {
            eprintln!("   原因: {}", cause);
        }
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args = Args::parse();

    // 🎯 攔截並生成自動補全腳本
    if let Some(generator) = args.generator {
        let mut cmd = Args::command();
        let name = cmd.get_name().to_string();
        clap_complete::generate(generator, &mut cmd, name, &mut std::io::stdout());
        return Ok(());
    }

    args.validate()?;

    let (app_config_dir, config) = setup::init_config()?;
    let config_file_path = app_config_dir.join("config.toml");

    // 🎯 參數名稱改為 config
    if args.config {
        setup::interactive_config_setup(&config_file_path, config)?;
        println!("👋 設定已完成，您可以重新執行程式來套用新設定。");
        return Ok(());
    }

    setup::check_dependencies()?;

    // 🎯 修正：直接檢查 String 是否為空
    let final_download_dir = args
        .output
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| {
            if config.download_dir.is_empty() {
                None
            } else {
                Some(PathBuf::from(&config.download_dir))
            }
        })
        .unwrap_or_else(|| dirs::download_dir().expect("找不到系統下載目錄"));

    let final_tmp_dir = app_config_dir.join(".tmp");

    // 🎯 修正：直接檢查 String 是否為空
    let resolved_cookie_dir = if config.cookie_dir.is_empty() {
        app_config_dir.clone()
    } else {
        PathBuf::from(&config.cookie_dir)
    };

    let is_silent = args.is_fully_automated();

    // 🎯 取得多組網址陣列
    let (input_urls, media_type, target_ext) = if is_silent {
        (
            args.url.clone().unwrap(),
            args.media_type.unwrap() as u8,
            args.format.clone().unwrap().to_lowercase(),
        )
    } else {
        ui::get_user_input(&args).context("無法取得使用者輸入")?
    };

    // 🎯 使用大迴圈處理每一個網址
    for input_url in input_urls {
        println!("\n▶️ 開始處理網址: {}", input_url);

        let site_target = parser::extract_site_name(&input_url);
        let (mut valid_videos, is_playlist, has_restricted) =
            parser::scan_url(&input_url, args.force_cookie, &site_target)?;

        let cookie_args = setup::handle_cookies(
            &site_target,
            has_restricted,
            &args.cookie,
            &resolved_cookie_dir,
            is_silent,
        )?;

        if !cookie_args.is_empty() && is_playlist {
            valid_videos =
                parser::rescan_with_cookies(&input_url, &cookie_args, valid_videos.len())?;
        }

        let final_target_dir =
            utils::prepare_output_dir(&final_download_dir, &input_url, &cookie_args, is_playlist);
        let dl_args = utils::build_download_args(media_type, &target_ext, &input_url, &cookie_args);

        // 🎯 呼叫非同步下載迴圈並傳入並行數
        utils::execute_download_loop(
            valid_videos,
            is_playlist,
            media_type,
            target_ext.clone(),
            dl_args,
            cookie_args,
            final_target_dir,
            final_tmp_dir.clone(),
            args.force_cookie,
            config.max_concurrent_downloads, // 傳遞設定中的並行數
        )
        .await?;
    }

    Ok(())
}
