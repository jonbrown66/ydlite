# Kumo appearance refinement verification

**Current status (2026-10-13): all download UI refinements have been withdrawn at the user's request. Only light/dark/system appearance remains. The download page, queue, options, action menu, queue store and existing browser test scripts match the original tracked baseline. The speed chart, sampler and its test were removed. `retained-light.png` and `retained-dark.png` show the restored layout. Everything below is a historical record of the withdrawn implementation, not the current UI. Backups and historical screenshots are retained for recovery.**

Verified on 2026-10-13. This preview runs the real Vue app and shared controls with mocked Tauri IPC. Sample URLs, titles, paths and download events are fixtures.

## Delivered

- Light, dark and system appearance; persisted selection, live system changes and an early desktop startup theme.
- Shared semantic colors for the shell, controls, menus, dialogs and task states.
- Download settings/activity columns on wide windows and stacked content on narrow windows.
- Prepare/download/process feedback, aligned progress metrics and a shared keyboard-accessible queue action menu.

## Passed checks

- `npm run build:app`: TypeScript and desktop frontend production build.
- `npm run build`: website production build.
- `node scripts/appearance-regression.cjs`: system/manual choice, restart persistence, corrupt preferences, storage failure, cross-window synchronization and cleanup.
- Existing Node regressions: download storage, download options, download queue and persistent queue.
- `git diff --check`.

## Internal browser checks

- Light/dark switching, selection in Settings, reload persistence, system preference changes and startup shell theme.
- Prepare/download/process/completed/error states; missing-tools warning and installation confirmation dialog.
- Queue menu Arrow keys and Escape focus return; unsaved settings confirmation and dialog focus.
- Queue layouts at 720, 960 and 1180 px: no horizontal overflow. Settings at 720 × 560: no body overflow; save bar does not cover the settings sheet.
- Reduced motion preserves gentle progress feedback. No console errors were captured.
- Measured semantic text/background combinations meet 4.5:1: body, muted, subtle, primary, primary foreground, danger, danger foreground, success and warning. This is a targeted palette check, not a complete accessibility certification.

## Scope limits

The browser screenshots verify frontend behavior with mocked IPC. They do not verify native WebView2 rendering, actual yt-dlp downloads, native dialogs, or a packaged installer. The existing Chrome browser regression scripts were adapted to the new menu/progress markup but were not executed, because the requested browser for verification was the internal browser.

No download backend changes, deployment, publication or file deletions were performed. Backups of edited files are in `before/`; existing user artifacts were preserved.

## Screenshots

- `light-progress.png` and `dark-progress.png`: matching desktop layouts.
- `light-queue-720.png` and `dark-queue-720.png`: narrow queue layouts.
- `dark-progress-720.png`: narrow active progress.
- `light-error.png` and `dark-error.png`: failed download.
- `light-warning.png` and `dark-warning.png`: missing tools.
- `dark-menu.png`, `dark-dialog.png`, `dark-install-dialog.png`: floating layers.
- `light-settings-720.png` and `dark-settings-720.png`: narrow Settings.

Run `node artifacts/kumo-themes/prepare-preview.cjs` to recreate the local test page from the existing mock fixture and actual startup shell.
