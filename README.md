# YDLite

YDLite 是一款面向 Windows 的本地视频下载与字幕处理工具。粘贴视频链接即可下载，也可以导入本地视频，自动检查已有字幕、识别语音、翻译成中文，并按需生成带字幕的视频。

[下载 Windows 安装包](https://ydlite.pages.dev) · [提交问题](https://github.com/jonbrown66/ydlite/issues)

## 能做什么

### 视频下载

- 使用 `yt-dlp` 解析和下载视频。
- 下载前查看标题、封面、时长、格式和播放列表。
- 默认优先选择 MP4 视频与 M4A/AAC 音频，提高 Windows 播放兼容性。
- 显示下载百分比、速度、剩余时间、输出位置和详细日志。
- 支持播放列表选择、`cookies.txt` 和下载记录。
- 默认启用 8 路媒体分片；检测到 aria2 时为普通 HTTP 下载启用多连接加速。

### 转录翻译

导入视频后，YDLite 会先检查字幕轨道：

1. 已有中文字幕：直接提取，不重复识别。
2. 已有其他语言文本字幕：保留原时间轴，只翻译文本。
3. 没有文本字幕：从音轨识别语言并生成字幕。

目前提供三种处理方式：

| 方式 | 语音识别 | 中文翻译 | 适合场景 |
| --- | --- | --- | --- |
| 本地免费 | Whisper 本地运行 | 必应翻译兼容通道 | 不想配置 API Key |
| 本地识别 + 自定义 AI | Whisper 本地运行 | OpenAI 兼容接口 | 已有自己的 AI 接口 |
| Gemini 云端 | Gemini | Gemini | 更少本地模型占用 |

本地 Whisper 支持自动语言识别，包括中文、英语、日语、韩语和多语言内容。模型与运行组件都不会强制下载：

- Whisper Small Q5：约 181 MB。
- Whisper Large V3 Turbo Q5：约 548 MB。
- CPU 运行组件：约 7.6 MB。
- NVIDIA CUDA 运行组件：约 647 MB。

### 字幕与视频输出

- 自动生成中文字幕 SRT。
- 可选择生成带字幕的 H.264 MP4。
- 自动检测 NVIDIA NVENC、Intel QSV、AMD AMF；硬件编码失败时回退 CPU。
- 使用临时文件生成，只有 ffprobe 验证可读取且时长正确后才显示完成。
- 不覆盖原视频。

### 任务与存储

- 下载完成后可直接进入“转录翻译”。
- 任务记录可播放输出视频、打开字幕、打开文件夹、删除单条或清空记录。
- 保存字幕阶段耗时、重试次数、上传量和视频编码方式。
- 应用异常关闭后，未完成任务会变为“可继续”，Gemini 分块任务从已完成部分续跑。
- 设置页可查看和清理临时音频、WebView 缓存及清理记录。
- 清理缓存不会删除模型、字幕项目或输出视频。

## 快速开始

### 下载视频

1. 粘贴视频链接。
2. 点击“解析链接”。
3. 选择格式和保存目录。
4. 点击“开始下载”。
5. 下载完成后打开文件，或直接创建字幕。

### 生成中文字幕

1. 打开“转录翻译”。
2. 选择本地视频，或从下载结果进入。
3. YDLite 自动检查字幕、音轨和语言。
4. 确认是否生成字幕视频。
5. 点击生成，完成后播放视频或打开字幕文件。

首次使用本地模式，需要在“设置”中主动下载一个 Whisper 模型和运行组件。使用 Gemini 或自定义 AI 时，需要先保存相应 API Key。

## 隐私与密钥

- 下载、Whisper 识别、字幕项目和视频烧录都在本机执行。
- Gemini 模式只上传提取后的压缩音频，不上传视频画面。
- Gemini 文件在请求完成、失败或取消后删除。
- Gemini Key 和自定义 AI Key 保存在 Windows 凭据管理器，不写入项目 JSON、前端存储或日志。
- 使用必应翻译兼容通道时，字幕文本会发送到微软翻译服务。该方式无需用户 API Key，但不是承诺长期稳定的正式开发者 API。

## 安装与依赖

用户环境：

- Windows 10 或 Windows 11（x64）。
- WebView2 Runtime（Windows 11 通常已包含）。
- `yt-dlp` 与 `ffmpeg`。

YDLite 可在应用内安装缺失工具，默认位置：

```text
<ydlite.exe 所在目录>\tools\
```

工具查找顺序：

1. `YDLITE_YTDLP`、`YDLITE_FFMPEG`、`YDLITE_FFPROBE` 环境变量。
2. `<ydlite.exe 所在目录>/tools/`。
3. 系统 `PATH`。

Windows 安装包：

- `YDLite_0.1.0_x64-setup.exe`
- `YDLite_0.1.0_x64_en-US.msi`

## 下载策略

默认视频格式：

```text
bv*[ext=mp4]+ba[ext=m4a]/bv*[vcodec^=avc1]+ba[ext=m4a]/b[ext=mp4]/bv*+ba/b
```

核心参数：

```text
--merge-output-format mp4 --no-playlist --newline -N 8 --windows-filenames --restrict-filenames
```

aria2 仅在系统中已安装时使用，并只接管普通 HTTP 下载；DASH/HLS 继续使用 yt-dlp 原生下载器，以保留稳定的分片处理和进度。

## 开发

需要：

- Node.js 20+
- Rust stable
- Tauri 2 的 Windows 构建环境

安装依赖：

```powershell
npm install
```

启动桌面应用：

```powershell
npm run tauri dev
```

启动 Landing Page：

```powershell
npm run dev:web
```

运行检查：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
npm run build:app
```

生成 Windows 安装包：

```powershell
npm run tauri build
```

产物位于：

```text
src-tauri/target/release/bundle/
```

## 项目结构

```text
src/                         Vue 桌面界面与 Landing Page
src/pages/                   下载、任务、设置页面
src/SubtitleWorkspace.vue    自动字幕主流程
src-tauri/src/               Rust 命令与业务逻辑
public/downloads/            Landing Page 提供的安装包
public/landing/              产品截图
```

## 技术栈

- Tauri 2
- Vue 3、TypeScript、Pinia、Vue Router
- Rust、Tokio、Reqwest
- yt-dlp、FFmpeg / ffprobe
- whisper.cpp
- Gemini Interactions API
- Vite、Cloudflare Pages

## 当前范围与限制

- 当前为 Windows x64 桌面版。
- 不提供实时麦克风转录、说话人识别或专业逐字级时间轴对齐。
- 图片字幕不会 OCR，会改为从音频重新识别。
- Gemini 生成式时间戳可能需要人工抽查；对于同步要求严格的正式发布视频，请先检查字幕。
- 平台下载能力取决于 `yt-dlp`、目标网站和账号权限。
- 必应翻译兼容通道可能随微软服务变化而失效，可切换到 Gemini 或自定义 AI。

## 第三方组件

第三方组件及许可见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

## License

YDLite 使用 [MIT License](LICENSE)。
