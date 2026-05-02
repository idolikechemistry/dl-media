# dl-media (v0.2.1) 

![Rust](https://img.shields.io/badge/Language-Rust-orange.svg)
![Version](https://img.shields.io/badge/Version-0.2.1-blue.svg)
![CI](https://img.shields.io/github/actions/workflow/status/idolikechemistry/dl-media/release.yml)

**dl-media** 是一個專為終端機使用者設計的全能影音下載工具。它簡化了 `yt-dlp` 的複雜參數，提供直覺的選單，並自動處理影片後續的彈幕合併與字幕淨化工作。

## 🌟 核心特色
- **互動式與自動化雙模**：提供直覺選單，亦支援完全自動化的指令參數執行。
- **支援多種格式**：嚴格匹配音訊 (M4A/MP3) 或影片 (MP4/MKV) 格式，防止無效下載。
- **Bilibili 自動優化**：自動處理 Cookie 並將 XML 彈幕轉為 ASS 格式封裝進影片。
- **純淨字幕與日誌**：內建過濾器移除 VTT 標籤，並將下載資訊精簡為純淨進度條。
- **沙盒 Cookie 管理**：智慧識別網站（Twitter/X, YouTube, Bilibili 等），自動管理專用 Cookie 檔案。
- **自訂輸出目錄**：支援透過參數指定下載位置，並自動分類播放清單項目。

---

## 🛠️ 執行前準備
無論您使用哪種安裝方式，請確保您的電腦已安裝以下核心工具：

1. **yt-dlp**: 影音下載核心。
2. **FFmpeg**: 影音轉碼與封裝工具。
3. **ffprobe**: 影音資訊解析工具（通常隨 FFmpeg 一併安裝）。

**macOS 安裝方式：**
```bash
brew install yt-dlp ffmpeg
```

---

## 🚀 快速開始

1. **執行程式**：
   直接執行即可進入互動選單：
   ```bash
   ./dl-media
   ```

2. **靜默自動化下載**：
   若同時指定網址、類型與格式，程式將跳過所有選單自動下載：
   ```bash
   ./dl-media -u "網址" -m 3 -f mp4
   ```

---

## 📖 指令參數說明 (Options)

| 參數 | 說明 | 範例 |
| :--- | :--- | :--- |
| `-u, --url` | 貼上要下載的影片或播放清單網址 | `-u "https://..."` |
| `-m, --media-type` | 指定下載類型 (1:音訊, 2:無聲影片, 3:有聲影片) | `-m 3` |
| `-f, --format` | 指定輸出格式 (音訊:mp3/m4a, 影片:mp4/mkv) | `-f mp4` |
| `-o, --output` | 指定輸出資料夾路徑 (預設為系統 Downloads) | `-o "./my_videos"` |
| `-c, --cookie` | 手動指定特定 Cookie 檔案路徑 | `-c "./cookie.txt"` |
| `--fc` | 強制調用 config 內已儲存的 Cookie | `--fc` |
| `--open-config` | 打開儲存 Cookie 的專屬設定資料夾 | `--open-config` |
| `-h, --help` | 顯示所有指令說明 | `-h` |

---

## 🍪 Cookie 管理機制
程式會根據網址自動對應 Cookie 檔案。輸入 `./dl-media --open-config` 即可開啟設定資料夾，請根據網站存入對應檔案：
- **YouTube**: `cookie_youtube.txt`
- **Bilibili**: `cookie_bilibili.txt`
- **Twitter/X**: `cookie_twitter.txt`
- **Instagram**: `cookie_instagram.txt`

> **提示**：若偵測到需要權限或畫質受限，程式會主動提醒。使用 `--fc` 參數可強制啟用 Cookie 驗證。

---

## 👨‍💻 自行編譯
如果您希望從原始碼構建專案，請先安裝 [Rust 開發環境](https://rustup.rs/)：
```bash
# 複製專案
git clone [https://github.com/idolikechemistry/dl-media.git](https://github.com/idolikechemistry/dl-media.git)
cd dl-media

# 編譯 Release 版本
cargo build --release

# 執行檔位置
./target/release/dl-media
```

---
*Developed by [idolikechemistry](https://github.com/idolikechemistry)*