# 下载回归检查

`node scripts/appearance-regression.cjs` 验证浅色、深色和跟随系统、手动选择优先级、重启持久化、多窗口同步、存储不可用与监听清理。外观偏好使用独立存储键，不修改下载偏好或任务数据。

`node scripts/download-options-regression.cjs` 覆盖文件与音轨选项持久化、覆盖前确认与拒绝确认、跳过已有文件终态、同分辨率不同编码/帧率及错误分类。`cargo test --manifest-path src-tauri/Cargo.toml --lib real_file_policies_and_mp3_conversion -- --include-ignored` 使用 `YDLITE_TEST_MEDIA_URL` 指定本机可播放测试媒体，验证实际跳过、编号、覆盖、MP3 转换及格式失效。公开站点回归 `real_public_download_and_playback` 单独运行，需提供 `YDLITE_TEST_PUBLIC_URL`，会真实下载 best/720p/MP3；不默认访问外部站点。

`node scripts/download-queue-regression.cjs` 运行真实页面的 TypeScript 逻辑并模拟 IPC，覆盖终态早于返回、失败后继续、启动失败、取消、失败项单独重试、格式映射、历史归属和日志批量更新。

`node scripts/download-storage-regression.cjs` 覆盖历史容量与损坏数据、同文件去重、删除后新增、下载偏好的持久化及旧配置兼容。

`node scripts/persistent-queue-regression.cjs` 运行真实 Pinia 队列状态层，覆盖 20 链接独立解析、去重、播放列表展开、目录和格式恢复、单项重试、取消命令晚于下载返回、暂停与重启、解析取消、存储失败和损坏队列保护。

设置 `PLAYWRIGHT_MODULE` 和 `FRONTEND_TEST_URL` 后运行 `node scripts/queue-browser-regression.cjs`，以独立 Chrome 验证批量表单、键盘、单项操作、目录和格式修改、筛选、重载恢复及 720–1180px 布局。使用模拟 IPC；不访问真实站点或 Cookie 文件。截图位于 `artifacts/download-queue/`。

下载页标签检查覆盖 Single link / Multiple links 的方向键、Home / End、隐藏面板焦点、单链解析结果与多链草稿保留，以及切换时队列不被中断。截图位于 `artifacts/download-tabs/`。已有截图路径被占用时，可用 `FRONTEND_ARTIFACT_DIR` 指定前端回归的独立输出目录。

Windows 浏览器回归可运行 `scripts/run-frontend-regression.ps1`。它启动独立开发端口并在结束时关闭自己的服务。需要本机 Chrome 和 Playwright；通过 `PLAYWRIGHT_MODULE` 指定模块位置。

也可先启动 Tauri 模式的 Vite，再设置 `FRONTEND_TEST_URL` 执行 `node scripts/frontend-regression.cjs`。浏览器仅使用模拟下载数据和 Tauri IPC，不读取真实 Cookie 或账号配置。覆盖窗口控件、键盘与高对比焦点、下载进度与取消、历史菜单与搜索、设置保存与放弃、关闭确认及四种窗口宽度。

`cargo test --manifest-path src-tauri/Cargo.toml --lib` 运行后台单元测试。真实下载路径可单独运行：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib downloader::tests -- --include-ignored --nocapture
```

需要本机 yt-dlp，使用临时 HTTP 服务和测试媒体，覆盖解析缓存、失效回退、取消及单次完成事件；不访问真实视频站点。

`python scripts/download-performance-regression.py` 使用本机 yt-dlp 和 FFmpeg 测量临时媒体的解析及下载准备耗时。结果不代表真实站点速度。

截图保存在 `artifacts/frontend/`。浏览器模拟不替代 Windows WebView2 与真实站点验证。
