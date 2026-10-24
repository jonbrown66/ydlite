# YDLite

Windows 视频与音频下载工具。粘贴链接，选择格式和保存目录，下载完成后直接打开文件。

[下载 Windows 版](https://ydlite.pages.dev) · [提交问题](https://github.com/jonbrown66/ydlite/issues)

## 功能

- 用 yt-dlp 解析并下载视频、音频和播放列表条目。
- 格式可选最佳可用、720p 上限、MP3 音频，或指定具体格式。指定格式会区分相同分辨率的编码、帧率和音轨语言，失效时提示重新选择。
- 视频与音轨可分开选择：兼容优先合并为 MP4，质量优先保留来源编码并合并为 MKV。
- 进度条显示完成度，同时给出已下载大小、总大小和实时速率；开始前显示估算大小。
- 支持暂停。暂停会保留已下载的部分文件，继续时从断点续传（取决于站点是否支持）。
- 播放列表先勾选条目，再依次下载；单个失败不影响其余条目，可只重试失败项。
- 保留中文文件名。命名支持标题、作者＋标题、上传日期＋标题，并附加媒体与格式标识避免同名冲突。
- 重复文件可跳过、另存编号副本，或确认后覆盖。
- 保存最近 100 条下载记录，可按名称、来源和路径搜索。
- 可指定自己的 cookies.txt，访问需要登录的内容。
- 检查并安装 yt-dlp、FFmpeg，也可单独更新 yt-dlp。

## 使用

1. 首次打开时按提示检查或安装下载工具。
2. 粘贴链接，点击 **Go** 解析。
3. 选择格式和保存文件夹；播放列表可勾选要下载的条目。
4. 点击 **Download**。下载中可 **Pause** 或 **Cancel**，暂停后点 **Start** 继续。

不提供字幕提取、转录、翻译或烧录功能。

## 隐私与边界

下载请求由本机 yt-dlp 发往目标站点，应用不代理账号，也不上传下载内容。Cookie 文件只在本机读取。

预览缩略图由应用在本地抓取后传给界面，不经过第三方服务。设置与历史保存在本机，清空历史只删除记录，不删除已下载的文件。

下载能力取决于站点支持、网络状况和账号权限；文件大小与剩余时间是估算值。关闭应用会中断正在进行的下载。能否复用部分文件取决于 yt-dlp、站点和媒体格式，并非所有来源都支持断点续传。

重复检测依据本版写入的媒体与格式标识，只识别带标识的文件；旧版文件或手动改名后的文件不会被识别为新版本已有。覆盖操作每次都需要确认。

## 安装与开发

Windows x64 提供三种分发：

| 文件 | 说明 |
| --- | --- |
| `YDLite_0.2.0_x64-setup.exe` | NSIS 安装包，推荐 |
| `YDLite_0.2.0_x64_en-US.msi` | MSI，适合集中部署 |
| `YDLite_0.2.0_x64-portable.zip` | 解压后直接运行 `YDLite.exe`，无需安装 |

便携版把下载工具放在 exe 同级的 `tools\` 目录，或在设置页安装。设置和历史保存在当前 Windows 用户的 WebView2 配置中，不随 exe 移动。

运行依赖 yt-dlp、FFmpeg 和 WebView2。WebView2 随 Windows 10/11 提供，yt-dlp 与 FFmpeg 可由应用安装。

开发需要 Node.js、Rust 和 Tauri 2 的 Windows 构建环境。包管理器使用 pnpm。

```powershell
pnpm install
pnpm tauri dev        # 启动桌面应用（需要 Rust）
pnpm dev:web          # 仅在浏览器中预览落地页
```

构建与检查：

```powershell
pnpm build:app        # 类型检查 + 桌面前端构建
pnpm build            # 落地页构建
pnpm tauri build      # 生成安装包与便携构建

node scripts/download-regression.cjs
node scripts/download-options-regression.cjs
node scripts/download-storage-regression.cjs
node scripts/appearance-regression.cjs
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

浏览器回归说明见 [scripts/README.md](scripts/README.md)。技术栈为 Tauri 2、Vue 3、TypeScript、Rust、yt-dlp 与 FFmpeg。

第三方声明见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。[MIT License](LICENSE)。
