# dl-media (v0.0.1) 🎥

![Rust](https://img.shields.io/badge/Language-Rust-orange.svg)
![Version](https://img.shields.io/badge/Version-0.0.1-blue.svg)
![CI](https://img.shields.io/github/actions/workflow/status/idolikechemistry/dl-media/release.yml)

**dl-media** 是一個專為終端機使用者設計的全能影音下載工具。它簡化了 `yt-dlp` 的複雜參數，提供直覺的選單，並自動處理影片後續的彈幕合併與字幕淨化工作。

## 🌟 核心特色
- **互動式選單**：不需要記住複雜指令，執行後跟著提示選擇即可。
- **支援多種格式**：可選擇音訊 (M4A/MP3)、無聲影片或標準有聲影片 (MP4/MKV)。
- **Bilibili 自動優化**：自動處理 Cookie，並將 XML 彈幕轉為 ASS 格式封裝進影片。
- **純淨字幕**：內建過濾器，自動移除 VTT 字幕中的 HTML 標籤。
- **智慧命名與分類**：支援播放清單下載，並自動建立資料夾與加上時間戳記。

---

## 🛠️ 執行前準備
無論您使用哪種安裝方式，請確保您的電腦已安裝以下核心工具：

1. **yt-dlp**: 影音下載核心。
2. **FFmpeg**: 影音轉碼與封裝工具。

**macOS 安裝方式：**
```bash
brew install yt-dlp ffmpeg
```

---

## 🚀 一般使用者：快速開始 (直接下載)

1. **下載程式**：前往 [Releases](https://github.com/idolikechemistry/dl-media/releases) 下載適合您系統的檔案：
   - `dl-media-mac-arm64` (Apple Silicon Mac)
   - `dl-media-windows-x64.exe` (Windows)
   - `dl-media-linux-x64` (Linux)

2. **賦予權限 (macOS/Linux)**：
   下載後，請打開終端機，對檔案執行權限賦予：
   ```bash
   chmod +x dl-media-mac-arm64
   ```

3. **執行程式**：
   ```bash
   ./dl-media-mac-arm64
   ```

---

## 📖 使用說明
啟動程式後，依照提示操作：
1. **貼上網址**：支援單一影片或播放清單網址。
2. **選擇類型**：音訊、無聲影片或有聲影片。
3. **選擇格式**：如 MP3 (320k) 或 MP4 (高相容)。
4. **存檔位置**：下載完成後，檔案會自動儲存於您的「下載 (Downloads)」資料夾。

> **提示**：若下載 Bilibili 影片，請確保 Cookie 檔案位於 `/opt/homebrew/yt-dlp_cookie_bilibili.txt` 以獲取最高畫質權限。

---

## 👨‍💻 進階使用者：自行編譯

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