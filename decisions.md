# Decisions

## 2026-08-27 — Product foundation

- Work on `feat/privacy-aperture-foundation`; never push or merge directly to `main`.
- Ship Milestone 1 as first runnable vertical slice: Tauri 2 shell, versioned local config, tested matching/message core, and production-shaped vanilla TypeScript UI.
- Keep OS overlays, foreground adapters, native-host registration, signing, and installers out of this slice. Fake platform behavior would create false acceptance evidence.
- Use native HTML controls and zero UI frameworks. Vite only builds TypeScript and assets.
- Bundle Manrope and IBM Plex Mono through Fontsource packages; runtime makes no font or network requests.
- Keep privacy state local. Logs record engineering actions only—never foreground apps, hostnames, page titles, or URLs.
- Persist one versioned JSON document atomically in OS app-config directory. Reject unknown config versions and invalid visibility/hostname data.
- Treat browser context as ephemeral memory. Reject malformed, oversized, unsupported-version, and non-monotonic native messages.

## 2026-08-27 — Hardware brightness and privacy modes

- Correct earlier product assumption: user requires real display brightness control in addition to black overlays.
- Model Galaxy S26 Ultra behavior honestly. Its side-view restriction depends on Flex Magic Pixel panel hardware; ordinary laptop software cannot reproduce that optical effect.
- Reuse applicable interaction semantics: per-app activation, global quick toggle, and Maximum privacy. On ordinary laptops, Maximum privacy combines lower hardware brightness with overlay opacity; it never claims viewing-angle restriction.
- Prefer public native interfaces: macOS IOKit/CoreGraphics brightness parameters, Windows high-level monitor configuration API, and Linux kernel backlight sysfs ABI.
- Current macOS 26.6 hardware returned zero IOKit-controllable displays. Use `DisplayServices` fallback for built-in Mac panels because direct installers—not Mac App Store—are product target. Keep IOKit first for compatible displays and expose failures honestly.
- Detect each display independently and show unsupported/error states. External monitor DDC/MCCS support varies; never claim control without successful read/write.
- Brightness preview captures current values, applies target for three seconds, then restores. Immediate restore remains available.
- Expand config from v1 to v2 with hardware fields. Read v1 safely and migrate in memory; reject unknown future versions.

## 2026-08-28 — macOS automatic hardware brightness

- Stack `feat/macos-automatic-brightness` on foundation PR #1; keep one reviewable feature per branch and never merge or push directly to `main`.
- Use native `NSWorkspace` foreground/running-application APIs through Objective-C runtime FFI. Avoid shell polling, Accessibility permission, and new Rust dependencies.
- Poll foreground bundle identity every 150 ms. This stays below 250 ms activation target while avoiding app-title, window-title, URL, or content access.
- Treat hardware control as same physical brightness level changed by macOS display keys. Capture original level once when automatic protection begins, update target without recapturing, and restore on focus loss, pause, error, or normal exit.
- Manual apply and three-second preview temporarily own brightness session. Automatic rules resume after temporary session ends; user can pause protection for persistent emergency restore.
- Maximum privacy chooses 10% hardware brightness. Normal automatic mode uses user-selected 10–100% hardware level; rule visibility remains overlay percentage for later overlay branch.
