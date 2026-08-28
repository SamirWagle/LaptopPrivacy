# Privacy Aperture

Local-first laptop privacy dimmer. Sensitive apps and websites can combine real hardware brightness reduction with click-through black overlays.

Current stacked branch adds macOS foreground-app protection to repository foundation. Matching app rules cover only visible windows owned by that app. Optional hardware brightness changes real panel level and therefore remains explicitly global. Browser integration and Windows/Linux foreground automation remain separate milestones.

## Hardware behavior

- macOS overlay: CoreGraphics supplies visible bounds for foreground process without reading titles or content. Raw native black windows match those bounds and ignore pointer input.
- macOS hardware: public IOKit first, then direct-distribution `DisplayServices` fallback for built-in Apple panels. This changes same global level as display brightness keys; it cannot dim one app independently.
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

macOS hardware/desktop acceptance tests are ignored by normal CI and must run individually in active GUI session:

```sh
cargo test --manifest-path src-tauri/Cargo.toml reads_foreground_and_running_applications -- --ignored --test-threads=1
cargo test --manifest-path src-tauri/Cargo.toml automatic_control_changes_and_restores_panel_level -- --ignored --test-threads=1
```

## Privacy boundary

- No account, cloud, telemetry, page content, browsing history, titles, or full URLs.
- Config contains user-created application IDs, hostnames, and visibility levels only.
- Active context remains memory-only and clears on disconnect.
- App rules use window-bounded overlays. Hardware brightness is optional, disabled by default, and always panel-wide.
- Hardware preview lasts three seconds and restores captured brightness. Manual mode offers explicit restore. Automatic hardware mode captures once, applies configured 10–100% global level, then restores on focus loss, pause, error, or normal exit.
- Product reduces casual shoulder-surfing. It cannot stop cameras, screenshots, close viewing, or replace a physical privacy filter.
