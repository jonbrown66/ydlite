# 下载页信息层级精简

本轮修改仅涉及 `src/pages/DownloadPage.vue` 和 `src/styles.css`，保留上一轮的控件尺寸、圆角、页签留白及主题支持。

- 默认下载表单优先展示格式、保存位置和下载操作。
- 具体格式、音轨、命名、文件冲突策略、格式偏好及来源诊断归入原生 details 的 More options，支持键盘展开。保留原有选择逻辑。
- 工具状态和空状态去掉单独的背景、描边；视频类型改为辅助文字。
- 预计大小放在下载操作旁，减少独立信息行。
- 高级音轨及格式操作在队列运行时同步禁用。

没有删除功能或源文件。`before/` 是本轮修改前的两个源文件，复制后已核对哈希。`before-light.png` 和 `after-light.png` 是同一 960×900 视口的修改前后截图。

## 已执行验证

- `npm run build:app`：通过。
- `node scripts/appearance-regression.cjs`：通过。
- `node scripts/download-queue-regression.cjs`：通过。
- 内部浏览器使用真实 Vue 前端和已有 Tauri mock IPC，未进行真实下载。
- More options 支持 Enter 展开/收起；已选音轨和 Author + title 命名在收起再展开后保留；可选择具体 720p 格式。
- 模拟下载时，高级格式按钮和音轨禁用；42% 进度、3.2 MiB/s 速度及 00:08 剩余时间正常显示。
- 模拟下载失败显示连接错误及队列 Retry；缺少工具的场景保留醒目的 Missing yt-dlp, ffmpeg 和 Install 入口。
- 720px 窄窗口的深浅色和展开状态未观察到横向溢出；工具状态可用键盘展开；浏览器控制台无错误。

此记录不声称验证了原生安装器或真实网络下载，也不代表全应用信息架构已重做。
