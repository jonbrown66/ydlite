# YDLite

YDLite 是一款 Windows 视频下载与字幕处理工具。支持粘贴视频链接下载，也可以导入本地视频，完成语音转录、字幕翻译和字幕视频生成。

[下载 Windows 版](https://ydlite.pages.dev) · [提交问题](https://github.com/jonbrown66/ydlite/issues)

## 主要功能

- 使用 `yt-dlp` 解析和下载视频。
- 支持视频链接和本地视频。
- 自动检查视频是否已有字幕。
- 已有外语字幕时直接翻译，保留原时间轴。
- 没有字幕时自动识别音轨语言并生成中文字幕。
- 支持本地 Whisper 和自定义 OpenAI 兼容 AI 接口。
- 导出 SRT 字幕或生成带字幕的 MP4 视频；生成视频时可选择单语或双语字幕并自定义样式。
- 显示任务进度，支持任务记录、失败重试和缓存清理。

## 字幕处理方式

| 方式 | 语音识别 | 翻译 |
| --- | --- | --- |
| 本地免费 | Whisper | 免费翻译通道 |
| 本地 + 自定义 AI | Whisper | OpenAI 兼容接口 |

本地模型按需下载，不会在安装应用时强制下载：

- Whisper Small Q5：约 181 MB
- Whisper Large V3 Turbo Q5：约 548 MB
- CPU 运行组件：约 7.6 MB
- NVIDIA CUDA 运行组件：约 647 MB

## 基本使用

### 下载视频

1. 粘贴视频链接并解析。
2. 选择格式和保存位置。
3. 开始下载。
4. 下载完成后可直接创建字幕。

### 生成字幕

1. 打开“自动字幕”。
2. 选择本地视频或下载结果。
3. 点击“开始自动字幕”。
4. 应用会自动检查字幕轨道；没有可用字幕时使用 Whisper 识别，再完成翻译、校对和字幕视频生成。
5. 处理完成后播放字幕视频、打开字幕或打开文件夹。

首次使用时，请在“设置”中选择并下载 Whisper 模型和运行组件。若使用自定义 AI，填写服务提供商的 OpenAI 兼容 API 地址、模型 ID 和 API Key 即可。

字幕视频提供经典、强对比和黄色高亮三种预设，也可以调整字体、位置、字号、颜色、描边和双语排列。双语字幕共用同一时间轴，译文默认跟随原文样式。样式设置仅作用于烧录后的 MP4，SRT 仍导出为纯文本字幕。

## 隐私

- 本地 Whisper 识别和视频处理在设备上完成。
- 云端模式只上传处理所需的音频或文本。
- API Key 使用系统凭据管理器保存，不写入字幕项目或日志。
- `.env` 方式适合本地使用；若多人共用电脑，建议改用“设置”页保存到 Windows 凭据管理器。
- 原视频不会被覆盖。

## 安装

目前提供 Windows x64 安装包：

- `YDLite_0.1.0_x64-setup.exe`
- `YDLite_0.1.0_x64_en-US.msi`

应用依赖 `yt-dlp`、FFmpeg 和 WebView2。缺失的下载工具可在应用内安装。

## 开发

需要 Node.js、Rust 和 Tauri 2 的 Windows 构建环境。

```powershell
npm install
npm run tauri dev
```

常用检查：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
npm run build:app
```

构建安装包：

```powershell
npm run tauri build
```

## 技术栈

- Tauri 2、Vue 3、TypeScript
- Rust、Tokio、Reqwest
- yt-dlp、FFmpeg、whisper.cpp
- OpenAI 兼容 AI API

## 说明

- 当前仅提供 Windows x64 桌面版。
- 平台下载能力取决于目标网站、账号权限和 `yt-dlp` 支持情况。
- 自动生成的字幕建议在正式发布前进行检查。
- 免费翻译通道和自定义 AI 服务的可用性、模型支持与限流策略由对应服务商决定。

第三方组件及许可见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

## License

[MIT License](LICENSE)
