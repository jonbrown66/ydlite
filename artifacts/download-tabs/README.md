# 下载输入方式 Tab 切换

Single link / Multiple links 采用顶部标签切换，替换原批量输入折叠入口。两种输入和单链接解析结果保留在内存中；共用下载工具状态与持久化队列，切换不会启动、取消或重置下载。多链模式下拖放文本会追加到批量输入框。

标签实现 tablist / tab / tabpanel 关联、单一 Tab 停靠点和左右方向键、Home / End 切换。隐藏面板不参与键盘焦点。共用队列在两种模式下均可操作。

已运行 DownloadPage 逻辑回归、持久化队列回归、独立 Chrome 的队列与下载/历史/设置回归、应用类型检查与构建，以及 Windows 安装包构建。Chrome 采用模拟 Tauri IPC，不代表原生 WebView2 或真实站点下载验证。

图片：single.png、multiple.png、multiple-720.png。已有前端截图文件一度无法写入，保留该文件后将同一套回归输出到 frontend/，完整检查通过。
