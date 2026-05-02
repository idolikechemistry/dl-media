mod args;
mod parser;
mod setup;
mod utils;
mod ui; // 加上這行

use args::Args;
use clap::Parser;
use std::process;

fn main() {
    let args = Args::parse();
    args.validate();

    if args.open_config {
        setup::open_config_folder();
        process::exit(0);
    }

    setup::check_dependencies();

    // 修改處：從 ui 模組獲取輸入
    let (input_url, media_type, target_ext) = ui::get_user_input(&args);
    let site_target = parser::extract_site_name(&input_url);
    let is_silent = args.url.is_some() && args.media_type.is_some() && args.format.is_some();

    let (mut valid_urls, is_playlist, has_restricted) = parser::scan_url(&input_url, args.force_cookie, &site_target);
    let cookie_args = setup::handle_cookies(&site_target, has_restricted, &args.cookie, is_silent);

    if !cookie_args.is_empty() && is_playlist {
        valid_urls = parser::rescan_with_cookies(&input_url, &cookie_args, valid_urls.len());
    }

    let dl_args = utils::build_download_args(media_type, &target_ext, &input_url, &cookie_args);
    let target_dir = utils::prepare_output_dir(&args.output, &input_url, &cookie_args, is_playlist);

    utils::execute_download_loop(
        valid_urls, is_playlist, media_type, &target_ext, 
        dl_args, cookie_args, target_dir, args.force_cookie
    );
}