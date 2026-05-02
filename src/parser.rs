use std::process::Command;

/// 從網址中萃取出網站名稱，用來判斷後續對應的 Cookie 檔案
pub fn extract_site_name(url: &str) -> String {
    let host = url.replace("https://", "").replace("http://", "").replace("www.", "");
    let domain = host.split('/').next().unwrap_or("unknown");
    
    if domain.contains("youtu.be") || domain.contains("youtube.com") { return "youtube".to_string(); }
    if domain.contains("b23.tv") || domain.contains("bilibili.com") { return "bilibili".to_string(); }
    if domain.contains("x.com") || domain.contains("twitter.com") { return "twitter".to_string(); }
    if domain.contains("fb.watch") || domain.contains("facebook.com") { return "facebook".to_string(); }
    if domain.contains("instagram.com") { return "instagram".to_string(); }
    
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() >= 2 {
        parts[parts.len() - 2].to_string()
    } else {
        domain.to_string()
    }
}

/// 透過 yt-dlp 的 JSON 輸出，判斷影片是否含有一般字幕或自動產生字幕
pub fn has_subtitles(url: &str, cookie_args: &[String]) -> bool {
    let mut cmd = Command::new("yt-dlp");
    cmd.args(cookie_args).args(["--dump-json", "--no-warnings", "--playlist-items", "1", url]);
    if let Ok(output) = cmd.output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
            let has_subs = json.get("subtitles").map_or(false, |s| s.is_object() && !s.as_object().unwrap().is_empty());
            let has_auto_subs = json.get("automatic_captions").map_or(false, |s| s.is_object() && !s.as_object().unwrap().is_empty());
            return has_subs || has_auto_subs;
        }
    }
    false
}

/// 掃描目標網址：分析是否為播放清單、抓出所有影片網址，並偵測是否有權限限制
pub fn scan_url(input_url: &str, force_cookie: bool, site_target: &str) -> (Vec<String>, bool, bool) {
    println!("🔍 正在初步分析網址資訊...");
    
    // 檢查是否為播放清單
    let pl_check = Command::new("yt-dlp")
        .args(["--print", "playlist_title", "--no-warnings", "--playlist-items", "1", input_url])
        .output().expect("檢查清單屬性失敗");
    let pl_title = String::from_utf8_lossy(&pl_check.stdout).trim().to_string();
    let is_playlist = !pl_title.is_empty() && pl_title != "NA" && pl_title != "null";

    // 展開清單並獲取網址
    let scan_output = Command::new("yt-dlp")
        .args(["--flat-playlist", "--print", "%(title)s|%(webpage_url)s", "--ignore-errors", "--no-warnings", input_url])
        .output().expect("解析清單失敗");
    
    let stdout_str = String::from_utf8_lossy(&scan_output.stdout);
    let stderr_str = String::from_utf8_lossy(&scan_output.stderr).to_lowercase();
    
    let mut valid_urls: Vec<String> = Vec::new();
    let mut has_restricted = force_cookie || stderr_str.contains("sign in") || stderr_str.contains("login") 
        || stderr_str.contains("cookie") || stderr_str.contains("登錄") || stderr_str.contains("private");

    for line in stdout_str.lines() {
        if line.trim().is_empty() { continue; }
        if line.contains("[Private video]") || line.contains("[Deleted video]") || line.contains("Private") {
            has_restricted = true;
        } else if let Some((_title, url)) = line.rsplit_once('|') {
            valid_urls.push(url.to_string());
        } else {
            valid_urls.push(line.to_string());
        }
    }
    
    if valid_urls.is_empty() { valid_urls.push(input_url.to_string()); }
    let total = valid_urls.len();

    // 輸出分析結果
    println!("--------------------------------------------------");
    println!("📡 來源網站：{}", site_target);
    if is_playlist {
        println!("📋 內容類型：【播放清單】 (包含 {} 部內容)", total);
    } else {
        println!("📄 內容類型：【單一內容】");
    }

    if force_cookie {
        println!("🔒 權限狀態：⚠️ 您已啟用強制調用 config 內 Cookie 的模式 (--fc)");
    } else if has_restricted {
        println!("🔒 權限狀態：⚠️ 偵測到受限/私人內容！(需提供 Cookie 才能解鎖)");
    } else if site_target == "bilibili" {
        println!("🔓 權限狀態：公開 (💡 Bilibili 建議使用 Cookie 解鎖 1080p 高畫質)");
        has_restricted = true;
    } else {
        if site_target == "instagram" || site_target == "facebook" || site_target == "twitter" {
            println!("🔓 權限狀態：公開 (💡 若實際下載失敗，可加上 --fc 參數重試)");
        } else {
            println!("🔓 權限狀態：公開 (目前無偵測到存取限制)");
        }
    }
    println!("--------------------------------------------------");

    (valid_urls, is_playlist, has_restricted)
}

/// 如果發現有載入 Cookie，則重新掃描清單，試圖挖出隱藏或會員專屬影片
pub fn rescan_with_cookies(input_url: &str, cookie_args: &[String], original_total: usize) -> Vec<String> {
    println!("🔄 正在透過 Cookie 驗證並重新掃描清單...");
    let rescan_output = Command::new("yt-dlp")
        .args(cookie_args)
        .args(["--flat-playlist", "--print", "%(title)s|%(webpage_url)s", "--ignore-errors", "--no-warnings", input_url])
        .output().expect("重新解析清單失敗");
        
    let rescan_str = String::from_utf8_lossy(&rescan_output.stdout);
    let mut new_urls: Vec<String> = Vec::new();
    
    for line in rescan_str.lines() {
        if line.trim().is_empty() { continue; }
        // 過濾掉確定無法下載的私人或刪除影片
        if line.contains("[Private video]") || line.contains("[Deleted video]") { continue; }
        if let Some((_title, url)) = line.rsplit_once('|') {
            new_urls.push(url.to_string());
        } else {
            new_urls.push(line.to_string());
        }
    }
        
    if new_urls.is_empty() { new_urls.push(input_url.to_string()); }
    let new_total = new_urls.len();
    
    if new_total > original_total {
        println!("--------------------------------------------------");
        println!("🔓 解鎖成功！透過 Cookie 發現了隱藏/會員專屬內容。");
        println!("📋 更新解析結果：共包含 {} 部有效內容！", new_total);
        println!("--------------------------------------------------");
    }
    
    new_urls
}