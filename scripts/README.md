# 下载回归检查

`node scripts/appearance-regression.cjs` 验证浅色、深色和跟随系统、手动选择优先级、重启持久化、多窗口同步、存储不可用与监听清理。外观偏好使用独立存储键，不修改下载偏好或任务数据。

`node scripts/download-regression.cjs` 运行真实 Pinia 下载状态层并模拟 IPC，覆盖单任务下载、暂停后继续复用同一请求、取消、失败分类、播放列表依次下载、仅重试失败项，以及旧尝试的事件不会推进当前进度。

`node scripts/download-options-regression.cjs` 覆盖文件与音轨选项原样传给后台、覆盖前必须确认、失败分类与可操作提示。`cargo test --manifest-path src-tauri/Cargo.toml --lib real_file_policies_and_mp3_conversion -- --include-ignored` 使用 `YDLITE_TEST_MEDIA_URL` 指定本机可播放测试媒体，验证实际跳过、编号、覆盖、MP3 转换及格式失效。公开站点回归 `real_public_download_and_playback` 单独运行，需提供 `YDLITE_TEST_PUBLIC_URL`，会真实下载 best/720p/MP3；不默认访问外部站点。

`node scripts/download-storage-regression.cjs` 覆盖历史容量与损坏数据、同文件去重、删除后新增、下载偏好的持久化及旧配置兼容。

Windows 浏览器回归可运行 `scripts/run-frontend-regression.ps1`。它启动独立开发端口并在结束时关闭自己的服务。需要本机 Chrome 和 Playwright；通过 `PLAYWRIGHT_MODULE` 指定模块位置。

也可先启动 Tauri 模式的 Vite，再设置 `FRONTEND_TEST_URL` 执行 `node scripts/frontend-regression.cjs`。浏览器仅使用模拟下载数据和 Tauri IPC，不读取真实 Cookie 或账号配置。覆盖窗口控件、键盘与高对比焦点、解析、下载、暂停与继续、取消、历史、设置保存与放弃、关闭确认及四种窗口宽度。截图目录可用 `FRONTEND_ARTIFACT_DIR` 指定。

`cargo test --manifest-path src-tauri/Cargo.toml --lib` 运行后台单元测试，包括缩略图代理的 base64 与 URL 升级检查。真实下载路径可单独运行；缩略图代理的真实网络检查 `fetches_referer_protected_thumbnail` 需要访问外网图片主机：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib downloader::tests -- --include-ignored --nocapture
```

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib downloader::tests -- --include-ignored --nocapture
```

需要本机 yt-dlp，使用临时 HTTP 服务和测试媒体，覆盖解析缓存、失效回退、取消、单次完成事件，以及暂停保留 `.part` 文件、继续时从断点续传；不访问真实视频站点。

`python scripts/download-performance-regression.py` 使用本机 yt-dlp 和 FFmpeg 测量临时媒体的解析及下载准备耗时。结果不代表真实站点速度。

截图默认保存在 `test-output/frontend/`（构建产物，不入库）。浏览器模拟不替代 Windows WebView2 与真实站点验证。
