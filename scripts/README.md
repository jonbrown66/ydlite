# 前端回归检查

下载队列逻辑回归：`node scripts/download-queue-regression.cjs`。直接运行页面的 TypeScript 逻辑并模拟 IPC，覆盖终态事件早于调用返回、失败后继续、无事件的启动失败、取消锁定及历史归属；不需要浏览器或真实下载。

先运行 `npm run dev -- --mode tauri`，然后执行 `node scripts/frontend-regression.cjs`。
脚本需要可用的 Playwright 和本机 Chrome。可通过 `PLAYWRIGHT_MODULE` 指定 Playwright 模块路径，通过 `PLAYWRIGHT_CHANNEL` 改用 `msedge` 等已安装渠道。

脚本只在独立浏览器上下文中模拟 Tauri IPC，不使用真实配置、凭证或下载文件。覆盖关闭按钮悬停/按下颜色、下拉框键盘操作、取消状态、历史删除后的新增、设置放弃修改、弹窗焦点与关闭确认，以及四种窗口宽度下的布局。截图保存在 `artifacts/frontend/`。

这不替代 Windows WebView2、系统缩放和真实 yt-dlp 下载的端到端验证。
