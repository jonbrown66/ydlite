# YDLite 标识更新

将名称中的 Y 融入向下箭头，表达视频下载；下方两段短线形成下载落点。保留 UI 的 `#255ad6` 蓝色，以白色圆头线条和留白保证小尺寸识别。

## 资产与使用位置

- `src/assets/brand-mark.svg`：64×64 矢量源文件，圆角 16px；应用、官网和启动标题栏共同使用。正式构建的 HTML 图标引用由 Vite 转为有效的资产路径。
- `public/app-icon.svg`：与源文件相同的公开 SVG。
- `public/brand/ydlite-logo.png`：512×512 PNG，透明外角。
- `src-tauri/icons/icon.ico`：Tauri CLI 从同一 SVG 生成的多尺寸 Windows 图标，用于原生应用和安装包。
- `public/landing/ydlite-app.png`：替换为带新版 Logo 的应用预览。

## 生成与验证

图标生成命令：`node node_modules/@tauri-apps/cli/tauri.js icon src/assets/brand-mark.svg --output artifacts/logo/generated`。将生成的 `icon.ico` 和 `icon.png` 同步到上面的实际使用路径，避免从截图反复缩放。

`verify-logo.cjs` 已在独立 Chrome 中验证图片加载、网页图标请求、启动界面、标题栏拖动命中区域、SVG 一致性及 Windows ICO 的 16/32/48/256px 图层。人工查看 `logo-review.png` 中的 16–64px 尺寸、浅色和深色背景；未出现页面运行错误。`npm run build` 已通过。

实际 EXE 图标检查发现两个构建细节：Rust 增量构建没有监听 ICO 变化，NSIS 安装程序使用独立默认图标。已在 `src-tauri/build.rs` 加入 ICO 依赖监听，并在 Tauri 配置中设置安装/卸载图标路径；旧配置和构建脚本均已备份并核对哈希。

修正后的 `npm run tauri build` 与最终 `tauri bundle` 均通过。`verify-native-icon.ps1` 将实际 EXE 复制到按哈希命名的独立路径，排除 Windows 图标缓存影响，再读取 PE 内嵌图标；应用和安装程序均通过，并人工核对 [应用图标](app-native-icon.png) 与 [安装图标](setup-native-icon.png)。生成的 NSIS 脚本同时引用新版安装和卸载图标。

最终 EXE/MSI 已同步到 `public/downloads`，与构建产物的 SHA256 一致。未执行实际安装或卸载。

旧网页图标、Windows 图标与官网预览保存在本目录的 `.before` 文件中；此次未删除业务内容。
