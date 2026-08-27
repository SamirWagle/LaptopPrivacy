# Work log

## 2026-08-27

- Confirmed repository contained only `.gitattributes`; worktree clean on `main` at `50cdb9a`.
- Created branch `feat/privacy-aperture-foundation` before implementation.
- Read repository instructions plus Caveman, Ponytail, frontend-design, and lean-build guidance.
- Verified current Tauri 2 project/command structure and Chromium MV3 native-messaging model against official documentation.
- Started Milestone 1 foundation. Validation evidence appended after checks run.
- Added Tauri/Rust/vanilla TypeScript foundation and local bundled fonts.
- Passed frontend production build and 7 Rust tests.
- Built unsigned macOS `.app` bundle successfully.
- In-app browser backend unavailable; performed window-only macOS screenshot review instead.
- Screenshot found dark-theme root-color defect; fixed token ownership and reduced bundled font assets from all language subsets to Latin only.
- User corrected scope: actual hardware brightness required. Stopped test app/dev server before pivot.
- Verified Galaxy S26 Ultra Privacy display uses hardware-integrated Flex Magic Pixel viewing-angle control. Ordinary laptop panels cannot reproduce this optically in software.
- Verified platform brightness interfaces against Apple IOKit, Microsoft Monitor Configuration, and Linux kernel backlight documentation.
- Physical macOS probe found public IOKit symbols but zero controllable displays on current macOS 26.6 machine. Added built-in-panel `DisplayServices` fallback; direct distribution constraint recorded.
- Added v1-to-v2 config migration with preserved app/site rules and new hardware brightness settings.
- Added three-second hardware preview, manual apply/update/restore, normal-exit restore, per-display capability status, and S26-style Automatic conditions/Maximum privacy settings.
- Built Tauri app and verified built-in display detection in real WindowServer session: initial UI reading 87%.
- Applied hardware brightness at 35% through app; UI reported 35% and exposed Restore original.
- Restored brightness; next live read reported 82%. Difference from earlier 87% is consistent with system auto-brightness drift; framebuffer screenshots cannot prove perceived luminance.
- Fixed restore UI refresh lag and toast lifetime found during runtime test.
- `npm run build`, raw `cargo test`, and `git diff --check` passed; 9 Rust tests pass.
- Windows cross-check progressed until missing host `x86_64-w64-mingw32-windres`; Linux cross-check blocked by missing cross-compiled GTK/WebKit pkg-config sysroot. Neither platform has physical hardware validation.
- Generated desktop PNG/ICNS/ICO assets; removed generated Android/iOS icons because product scope is desktop-only.
- Final unsigned macOS `.app` bundle rebuilt successfully after formatting, UI refresh fix, and platform ownership review.
- Parallel validation found test temp-path collision from coarse clock resolution; replaced clock suffix with atomic per-process IDs. Raw Cargo rerun: 9/9 pass. Clippy with warnings denied: pass.
