# YDLite

YDLite is a compact Windows desktop downloader for links supported by
[`yt-dlp`](https://github.com/yt-dlp/yt-dlp). It is built with Tauri 2, Vue 3,
TypeScript, and Rust.

The product goal is simple: paste a link, parse it first, choose an output, and
download with clear local progress. YDLite does not use a cloud queue, account
system, or remote database.

## Features

- Fast Tauri desktop shell with a small Vue frontend.
- Manual dependency checks for `yt-dlp` and `ffmpeg` instead of slow startup checks.
- In-app guided installation for missing local tools.
- Link parsing with title, author, duration, thumbnail, source site, playlist entries, and format data.
- Compatible MP4 defaults: video downloads prefer `mp4` video with `m4a/AAC` audio before falling back.
- Optional `cookies.txt` for private or login-gated links.
- Save-folder selection, default-folder persistence, and recent download history.
- Live progress with percent, speed, ETA, total size, completion actions, and expandable logs.
- Playlist item selection with a simple serial queue.

## Download

Latest landing page:

https://43e18357.ydlite.pages.dev

Direct files are served from the landing page:

- `YDLite_0.1.0_x64-setup.exe`
- `YDLite_0.1.0_x64_en-US.msi`

## Requirements

For development:

- Windows 10/11
- Node.js 20+
- Rust stable
- Tauri 2 Windows prerequisites

At runtime, YDLite uses:

- `yt-dlp`
- `ffmpeg`

YDLite checks tools in this order:

1. `YDLITE_YTDLP` / `YDLITE_FFMPEG` environment variables
2. `<ydlite.exe directory>/tools/yt-dlp.exe` and `<ydlite.exe directory>/tools/ffmpeg/ffmpeg.exe`
3. System `PATH`

In-app setup installs tools into:

```text
<ydlite.exe directory>\tools\
```

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

Build the Tauri frontend only:

```powershell
npm run build:app
```

Build installers:

```powershell
npm run tauri build
```

Tauri bundles are generated under:

```text
src-tauri/target/release/bundle
```

## Download Strategy

The default video format selection is designed for Windows playback compatibility:

```text
bv*[ext=mp4]+ba[ext=m4a]/bv*[vcodec^=avc1]+ba[ext=m4a]/b[ext=mp4]/bv*+ba/b
```

This prioritizes MP4 video with M4A/AAC audio. If a site does not provide those
formats, YDLite falls back to the best available yt-dlp output.

Downloads also use:

```text
--merge-output-format mp4 --no-playlist --newline -N 4 --windows-filenames --restrict-filenames
```

## Project Structure

```text
src/                  Vue app and landing page
src-tauri/src/        Rust commands, downloader, progress parsing, tool setup
src-tauri/icons/      App icon
public/downloads/     Windows installer files used by the landing page
```

## Deployment

The landing page is deployed to Cloudflare Pages:

```powershell
npm run build
npx wrangler pages deploy dist --project-name ydlite --branch main --commit-dirty=true
```

Before deploying a new public build, copy fresh Tauri bundles into `public/downloads`.

## License

No license has been selected yet.
