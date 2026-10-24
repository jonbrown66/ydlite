# Changelog

## Unreleased

- Simplify the download page: one link input, shorter labels, no queue panel or global task tray.
- Remove the persistent download queue; a single task downloads directly and playlists run in order.
- Add pause and resume. Pausing keeps the partial file, so a resumed download continues where it stopped.
- Trim settings copy and keep only the options that change the result.
- Remove the clipboard link suggestion; paste is the single entry point.
- Load preview thumbnails through the backend so image hosts that reject foreign referers (Bilibili) display correctly.
- Rename the paused action to Start.
- Fix Cancel doing nothing while a download is paused.
- Use a rounded progress bar with a subtle active sheen; it dims when paused and turns green on completion.
- Add a portable build: an unzipped `YDLite.exe` runs without installation and reads `tools\` beside it.
- Redesign the landing page around the current interface: fresh screenshots, a features grid, scroll reveal, and download links for all three builds.
- Unify every button on one class contract and drop the Tailwind/CVA runtime that only styled a single component (-12% JS, -25% CSS in the desktop bundle).
- Remove the old release-validation artifacts, the promo video and the retired queue regression scripts from the repository.

## 0.2.0

- Focus YDLite on video and audio downloads; remove subtitle, transcription, translation and model-management features while retaining existing user files.
- Add durable batch queues, independent retry/cancellation, playlist expansion and restart recovery.
- Replace single/multiple link input navigation with block-style segmented tabs and keyboard navigation.
- Preserve Chinese filenames; add naming presets and skip, number or explicitly confirmed overwrite policies.
- Show distinct codecs, frame rates, audio languages and estimated format sizes. Require explicitly selected formats instead of falling back silently.
- Add compatibility/quality preferences and separate audio-track selection.
- Distinguish login, site access, rate limits, transport, format, disk-space and permission errors with recovery actions.
- Keep successful download history searchable and preserve saved options across upgrades.

