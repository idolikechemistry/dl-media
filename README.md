# dl-media (v0.2.1) 

![Rust](https://img.shields.io/badge/Language-Rust-orange.svg)
![Version](https://img.shields.io/badge/Version-0.2.1-blue.svg)
![CI](https://img.shields.io/github/actions/workflow/status/idolikechemistry/dl-media/release.yml)

**dl-media** 是一個為終端機使用者設計的影音下載工具。它簡化了 `yt-dlp` 的複雜參數，提供直覺的選單，並自動處理影片後續的彈幕合併與字幕淨化工作。

## 🌟 核心特色
- **互動式與自動化雙模**：提供直覺選單，亦支援完全自動化的指令參數執行。
- **支援多種格式**：嚴格匹配音訊 (M4A/MP3) 或影片 (MP4/MKV) 格式，防止無效下載。
- **Bilibili 自動優化**：自動處理 Cookie 並將 XML 彈幕轉為 ASS 格式封裝進影片。
- **純淨字幕與日誌**：內建過濾器移除 VTT 標籤，並將下載資訊精簡為純淨進度條。
- **沙盒 Cookie 管理**：智慧識別網站（Twitter/X, YouTube, Bilibili 等），自動管理專用 Cookie 檔案。
- **自訂輸出目錄**：支援透過參數指定下載位置，並自動分類播放清單項目。

---

無論您使用哪種安裝方式，請確保您的系統已安裝以下核心工具：
- **[yt-dlp](https://github.com/yt-dlp/yt-dlp)** : 影音下載核心。
- **[ffmpeg](https://www.ffmpeg.org/download.html)** : 影音轉碼與封裝工具。
- **[ffprobe](https://ffmpeg.org/ffprobe.html)** : 影音資訊解析工具。


---

## 🚀 安裝方式

### macOS / Linux 使用者
請打開終端機並貼上以下指令。此指令會自動下載執行檔、賦予權限，並將其移動至系統路徑以便全域調用（此範例為 Mac Apple Silicon 版本）：

```bash
curl -L [https://github.com/idolikechemistry/dl-media/releases/download/v0.2.1/dl-media-mac-arm64](https://github.com/idolikechemistry/dl-media/releases/download/v0.2.1/dl-media-mac-arm64) -o dl-media && \
chmod +x dl-media && \
sudo mv dl-media /usr/local/bin/
```
*(注意：Linux 使用者請將網址中的 `dl-media-mac-arm64` 更改為 `dl-media-linux-x64` 即可)*

### Windows 使用者
請前往 [Releases](https://github.com/idolikechemistry/dl-media/releases) 頁面下載最新的 `dl-media-windows-x64.exe` 並手動放置於您的資料夾中。

---
## 使用方式

### MacOS / Linux 使用者

開啟終端機，輸入
```bash
dl-media
```

> [!NOTE] 
> 第一次建議使用 `dl-media -h` 查看說明文件

### Windows 使用者

直接打開執行檔就好

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
程式會根據網址自動對應 Cookie 檔案。輸入 `dl-media --open-config` 即可開啟設定資料夾，請根據網站存入對應檔案：
- **YouTube**: `cookie_youtube.txt`
- **Bilibili**: `cookie_bilibili.txt`
- **Twitter/X**: `cookie_twitter.txt`
- **Instagram**: `cookie_instagram.txt`

可自行搜尋瀏覽器插件來匯出cookie檔案

---

## 👨‍💻 自行編譯
如果您希望從原始碼構建專案：

```bash
git clone [https://github.com/idolikechemistry/dl-media.git](https://github.com/idolikechemistry/dl-media.git)
cd dl-media
cargo build --release
./target/release/dl-media
```

---
