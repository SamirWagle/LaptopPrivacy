# Privacy Aperture

Local-first laptop privacy dimmer. Sensitive apps and websites can combine real hardware brightness reduction with click-through black overlays.

Current stacked branch adds automatic macOS foreground-app protection to repository foundation. Matching app rules now change real panel brightness and restore captured level when focus leaves. Native overlays, browser integration, and Windows/Linux foreground automation remain separate milestones.

## Hardware behavior

- macOS: public IOKit first, then direct-distribution `DisplayServices` fallback for built-in Apple panels. This changes same hardware level as display brightness keys; automatic rules poll native foreground bundle ID every 150 ms.
- Windows: Monitor Configuration API/MCCS. Monitor support varies and needs physical QA.
- Linux: kernel `/sys/class/backlight` interface. Write permissions depend on distribution policy.
- External displays: detected per panel; unsupported DDC/MCCS hardware remains unchanged.

Galaxy S26 Ultra Privacy display narrows viewing angles using Flex Magic Pixel panel hardware. Normal laptop panels cannot reproduce that optical effect in software. Privacy Aperture mirrors useful controls—app conditions, quick restore, and Maximum privacy—while Maximum privacy means hardware dimming plus overlay, not side-view blocking.

## Development

Requirements: Node.js 20+, Rust stable, and [Tauri 2 platform prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
npm install
npm run check
npm run tauri dev
```

## Privacy boundary

- No account, cloud, telemetry, page content, browsing history, titles, or full URLs.
- Config contains user-created application IDs, hostnames, and visibility levels only.
- Active context remains memory-only and clears on disconnect.
- Hardware preview lasts three seconds and restores captured brightness. Manual mode offers explicit restore. Automatic app rules capture once, apply configured 10–100% hardware level, then restore on focus loss, pause, error, or normal exit.
- Product reduces casual shoulder-surfing. It cannot stop cameras, screenshots, close viewing, or replace a physical privacy filter.
