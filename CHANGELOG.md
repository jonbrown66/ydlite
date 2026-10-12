# Changelog

## 0.2.0

- Focus YDLite on video and audio downloads; remove subtitle, transcription, translation and model-management features while retaining existing user files.
- Add durable batch queues, independent retry/cancellation, playlist expansion and restart recovery.
- Replace single/multiple link input navigation with block-style segmented tabs and keyboard navigation.
- Preserve Chinese filenames; add naming presets and skip, number or explicitly confirmed overwrite policies.
- Show distinct codecs, frame rates, audio languages and estimated format sizes. Require explicitly selected formats instead of falling back silently.
- Add compatibility/quality preferences and separate audio-track selection.
- Distinguish login, site access, rate limits, transport, format, disk-space and permission errors with recovery actions.
- Keep successful download history searchable and preserve saved options across upgrades.

Release validation and remaining environment-dependent checks are recorded in `artifacts/p0-p1/README.md`.
