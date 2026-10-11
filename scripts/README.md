# 前端回归检查

下载队列逻辑回归：`node scripts/download-queue-regression.cjs`。直接运行页面的 TypeScript 逻辑并模拟 IPC，覆盖终态事件早于调用返回、失败后继续、无事件的启动失败、取消锁定及历史归属；不需要浏览器或真实下载。

先运行 `npm run dev -- --mode tauri`，然后执行 `node scripts/frontend-regression.cjs`。
脚本需要可用的 Playwright 和本机 Chrome。可通过 `PLAYWRIGHT_MODULE` 指定 Playwright 模块路径，通过 `PLAYWRIGHT_CHANNEL` 改用 `msedge` 等已安装渠道。

Windows 可直接执行 `scripts/run-frontend-regression.ps1`。它使用独立的 1422 端口启动测试服务，并在成功或失败后自动关闭该服务；不会占用应用开发端口 1420。

脚本只在独立浏览器上下文中模拟 Tauri IPC，不使用真实配置、凭证或下载文件。覆盖关闭按钮悬停/按下颜色、下拉框键盘操作、取消状态、历史删除后的新增、任务搜索与类型筛选、下载完成后的字幕入口、收起输出设置后仍按所选内容导出 SRT 且不生成视频、设置放弃修改、弹窗焦点与关闭确认，以及四种窗口宽度下的字幕布局和保存栏不遮挡内容。截图保存在 `artifacts/frontend/`。

脚本还检查任务操作菜单的 Escape 和方向键操作、关闭后的焦点返回、字幕样式预设的方向键选择，以及展开输出设置时主操作按钮仍位于首屏。设置截图使用模拟模型和运行组件记录。

这不替代 Windows WebView2、系统缩放和真实 yt-dlp 下载的端到端验证。

焦点回归覆盖输入组的单层外框、按钮和下拉的统一 outline、菜单键盘焦点及高对比模式；减少动画模式仍保留温和等待反馈和进度过渡。

性能回归增加折叠日志突发、最近 1000 行保留、展开后的批量刷新，以及进度即时更新。后端检查完整解析的判定、120 秒缓存失效、工具替换和版本缓存失效。

`python scripts/download-performance-regression.py` 使用项目已安装的 yt-dlp 和本机 FFmpeg，通过临时 HTTP 服务比较重复解析与单次解析、URL 下载与解析信息复用。只使用临时测试媒体，不访问真实视频站点；结果是本机准备耗时，不代表网站传输速度。

真实 Rust 下载路径回归：`cargo test --manifest-path src-tauri/Cargo.toml --lib downloader::tests -- --include-ignored --nocapture`。使用应用选择的本机 yt-dlp（包括 PATH 安装），覆盖真实解析到缓存再到下载的完整路径、触发到首个数据的耗时、下载不重复请求解析页面、失效回退、取消不重试及单次完成事件。

工具缓存测量：`cargo test --manifest-path src-tauri/Cargo.toml --lib tool_probe::tests::benchmark_real_tool_version_cache -- --ignored --nocapture`。此单项需要 `src-tauri/target/release/tools/yt-dlp.exe`。PATH 查找回归验证搜索顺序、扩展名和缓存建立。
