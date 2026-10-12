# YDLite

YDLite 是一款 Windows 视频与音频下载工具。粘贴链接，选择格式和保存目录，查看进度并打开下载结果。

[下载 Windows 版](https://ydlite.pages.dev) · [提交问题](https://github.com/jonbrown66/ydlite/issues)

## 功能

- 使用 yt-dlp 解析和下载视频、音频及播放列表。
- 选择最佳可用格式、720p 上限、MP3 音频或指定视频格式。
- 展开详细格式，区分相同分辨率的编码、帧率和音轨语言；指定格式失效时提示重新选择。
- 选择兼容优先或质量优先；视频独立音轨可单独选择，音视频合一的格式保留自带音轨。
- 保留中文文件名，提供标题、作者＋标题、上传日期＋标题预设，附加媒体与格式标识。
- 重复下载可跳过、编号或确认覆盖；不同媒体同名不会共享输出文件。
- 显示文件大小估算、下载速度、剩余时间和文件处理状态。
- 批量粘贴多行链接，去重并逐项解析；播放列表展开为独立任务。
- 保存最多 500 个队列任务，包括目录、格式和状态；重启后手动继续未完成任务。
- 顺序下载，单项取消不影响后续任务；失败可单独重试或重新解析，也可修改目录与格式。
- 使用自己的 cookies.txt 访问需要登录的内容。
- 保存最近 100 条成功下载记录，按名称、来源和路径搜索。
- 保存默认目录和格式，管理剪贴板链接提示。
- 检查并安装 yt-dlp、FFmpeg，更新 yt-dlp。

## 使用

1. 打开应用，按提示检查或安装下载工具。
2. 粘贴链接并点击 Analyze。
3. 选择格式、播放列表条目和保存文件夹。
4. 开始下载；完成后打开文件或文件夹。

下载页使用 Single link / Multiple links 标签切换输入方式，保留各自输入和解析结果。批量下载选择 Multiple links，每行粘贴一个链接，选择共用的目录与格式，再点击 Import and analyze。查看解析结果后点击 Continue 开始下载；Pause after current 会完成当前任务后暂停。

不提供字幕提取、转录、翻译或烧录功能。升级不会自动删除旧版创建的项目、模型或用户输出文件。

## 隐私与边界

下载请求由本机 yt-dlp 发往目标站点。Cookie 文件由本机读取，应用不提供账号代理服务。下载能力取决于站点支持情况、网络和账号权限。文件大小与剩余时间可能是估算值。

清空历史只删除记录，不删除下载文件。无需 AI API Key 或语音模型。

队列保存在本机，包括来源链接与 Cookie 文件路径，不保存 Cookie 内容或下载器的临时媒体地址。关闭应用后不自动开始下载。继续任务时，能否复用部分文件取决于 yt-dlp、站点和媒体格式，不能保证每个来源都支持断点续传。正在解析的任务取消后，需要等待当前解析返回，结果会被丢弃。

文件重复检测以本版附加的媒体与格式标识为依据，检查保存目录中仍然存在的文件；无法识别旧版未带标识的文件，或用户手动移除标识后的文件。覆盖策略每次开始队列都需确认。更换命名预设不会绕过本版的重复检测。质量优先合并时使用 MKV，以保留来源编码。

## 安装与开发

Windows x64 安装包：`YDLite_0.2.0_x64-setup.exe` / `YDLite_0.2.0_x64_en-US.msi`。应用依赖 yt-dlp、FFmpeg 和 WebView2。

开发需要 Node.js、Rust 和 Tauri 2 的 Windows 构建环境。

```powershell
npm install
npm run tauri dev
```

```powershell
node scripts/download-queue-regression.cjs
node scripts/persistent-queue-regression.cjs
node scripts/download-options-regression.cjs
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run build:app
npm run build
npm run tauri build
```

浏览器回归说明见 [scripts/README.md](scripts/README.md)。技术栈为 Tauri 2、Vue 3、TypeScript、Rust、yt-dlp 与 FFmpeg。

第三方声明见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。[MIT License](LICENSE)。
