# YDLite

**YDLite is a fast Windows video downloader desktop app powered by `yt-dlp` and `ffmpeg`.**

Paste a video URL, preview metadata first, choose a format or playlist items, and download locally with clear progress. YDLite is built for people who want a simple YouTube downloader style workflow without a cloud queue, account login, or heavy browser extension.

[Download YDLite](https://ydlite.pages.dev) · [Report an issue](https://github.com/jonbrown66/ydlite/issues)

## Why YDLite

Many video downloader tools are either command-line only, browser-based, or unclear about what is happening during a download. YDLite wraps the reliability of [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) in a lightweight Windows desktop interface.

YDLite focuses on:

- Fast startup with no automatic `yt-dlp` or `ffmpeg` scan on launch.
- Local-first downloads: links, cookies, history, and files stay on your machine.
- Preview before download: title, thumbnail, duration, source, playlist entries, and available formats.
- Windows-friendly MP4 output with M4A/AAC audio defaults to avoid common Opus playback issues.
- Visible progress with percent, speed, ETA, total size, output path, and logs.
- Optional `cookies.txt` support for private, login-gated, or region-limited videos.

## Features

| Feature | What it does |
| --- | --- |
| Link parsing | Reads video metadata before downloading, including playlist information when available. |
| MP4 defaults | Prefers `mp4` video plus `m4a/AAC` audio for better Windows player compatibility. |
| Format selection | Choose available video formats or audio-only output when supported by the source. |
| Playlist control | Select playlist entries and process them through a simple serial queue. |
| Manual tool check | Check `yt-dlp` and `ffmpeg` only when needed, keeping startup immediate. |
| Guided setup | Install or update local tools from inside the app. |
| Cookie support | Use `cookies.txt` when a video requires authentication or stricter access. |
| Download history | Keep recent completed files with quick access to the output folder. |

## Download

Get the latest Windows build from the landing page:

https://ydlite.pages.dev

Available installer files:

- `YDLite_0.1.0_x64-setup.exe`
- `YDLite_0.1.0_x64_en-US.msi`

## How It Works

1. Paste a video URL.
2. Click **Parse** to preview metadata and formats.
3. Choose a save folder and output mode.
4. Download with live progress.
5. Open the finished file or output folder from history.

YDLite delegates extraction and downloading to `yt-dlp`, then uses `ffmpeg` when merging or converting media is required.

## Download Strategy

The default video format selection is designed for Windows playback compatibility:

```text
bv*[ext=mp4]+ba[ext=m4a]/bv*[vcodec^=avc1]+ba[ext=m4a]/b[ext=mp4]/bv*+ba/b
```

This prioritizes MP4 video with M4A/AAC audio. If a site does not provide those formats, YDLite falls back to the best available `yt-dlp` output.

Downloads also use:

```text
--merge-output-format mp4 --no-playlist --newline -N 4 --windows-filenames --restrict-filenames
```

## Requirements

For users:

- Windows 10 or Windows 11
- `yt-dlp`
- `ffmpeg`

YDLite can install tools into:

```text
<ydlite.exe directory>\tools\
```

Tool lookup order:

1. `YDLITE_YTDLP` and `YDLITE_FFMPEG` environment variables
2. `<ydlite.exe directory>/tools/yt-dlp.exe`
3. `<ydlite.exe directory>/tools/ffmpeg/ffmpeg.exe`
4. System `PATH`

For developers:

- Node.js 20+
- Rust stable
- Tauri 2 Windows prerequisites

## Tech Stack

- Tauri 2
- Vue 3
- TypeScript
- Rust
- Vite
- GSAP for landing page motion
- Cloudflare Pages for the landing page

## Development

Install dependencies:

```powershell
npm install
```

Run the Tauri app:

```powershell
npm run tauri dev
```

Build the landing page:

```powershell
npm run build
```

Build the Tauri frontend:

```powershell
npm run build:app
```

Build Windows installers:

```powershell
npm run tauri build
```

Tauri bundles are generated under:

```text
src-tauri/target/release/bundle
```

## Project Structure

```text
src/                  Vue desktop app and landing page
src-tauri/src/        Rust commands, downloader, progress parser, tool setup
src-tauri/icons/      App icon
public/downloads/     Windows installer files served by the landing page
public/landing/       Landing page product screenshots
```

## FAQ

### Is YDLite a YouTube downloader?

YDLite supports links that `yt-dlp` supports, including many video platforms. Platform availability depends on `yt-dlp`, the source site, and whether the video requires login cookies.

### Does YDLite upload videos or URLs to a server?

No. YDLite is a local Windows desktop app. Downloads run on your machine through `yt-dlp` and `ffmpeg`.

### Why does YDLite prefer MP4 and AAC?

Some Windows players do not handle Opus audio inside downloaded files well. YDLite prefers MP4 video with M4A/AAC audio when available for better out-of-the-box playback.

### Why are tool checks manual?

Startup should feel instant. YDLite does not scan for `yt-dlp` and `ffmpeg` automatically on launch; use the **Check** button when you need to verify or install tools.

### Can YDLite download private videos?

If `yt-dlp` can access the video with cookies, YDLite can use a `cookies.txt` file. Access depends on the source platform and your account permissions.

## Search and AI Summary

YDLite is a Windows video downloader and `yt-dlp` GUI built with Tauri, Vue, TypeScript, and Rust. It is useful for local video downloads, MP4 downloads, playlist downloads, `ffmpeg`-based merging, `cookies.txt` access, and users who want a lightweight desktop alternative to command-line `yt-dlp`.

## Deployment

The landing page is deployed to Cloudflare Pages:

```powershell
npm run build
npx wrangler pages deploy dist --project-name ydlite --branch main --commit-dirty=true
```

Before deploying a public build, copy fresh Tauri bundles into `public/downloads`.

## License

YDLite is licensed under the [MIT License](LICENSE).
