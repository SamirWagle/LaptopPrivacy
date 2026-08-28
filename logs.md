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
- Committed as `a665653`, pushed `feat/privacy-aperture-foundation`, and opened PR #1 against `main`. GitHub reports zero configured check runs; no merge performed.

## 2026-08-28

- Created stacked branch `feat/macos-automatic-brightness` from clean foundation head `e6790fd`; `main` remained untouched.
- Added native macOS foreground and running-app discovery using bundle identifiers only. No Accessibility permission, activity history, titles, URLs, or content capture.
- Added 150 ms protection loop connecting foreground app rules to real hardware brightness level, with captured-original restore on focus loss, pause, error, and exit.
- Added shared automatic/manual/three-second-preview brightness ownership so target changes do not overwrite original restore level.
- Added current foreground/matched-rule UI state and running-application picker.
- Graphify code-only map: 5 Rust files, 77 nodes, 126 edges, 11 communities; generated output stays ignored.
- Live macOS foreground/running-app acceptance passed.
- Live hardware acceptance reduced physical panel brightness by 10 percentage points and restored captured level; ignored test passed.
- `npm run build` passed. `cargo clippy --all-targets -- -D warnings` passed. Tauri dev app launched in real desktop session without startup crash and stopped cleanly.
- Final gates: 10 automated Rust tests passed, 2 hardware/desktop tests remained explicitly ignored in normal CI and passed when run manually, TypeScript passed, formatting passed, and `git diff --check` passed.
- Built unsigned release `Privacy Aperture.app` successfully with automatic foreground-brightness runtime included.
- Committed as `ab21a37`, pushed only `feat/macos-automatic-brightness`, and opened stacked PR #2 against foundation branch. No merge or direct `main` push performed.
- Created stacked branch `feat/macos-native-overlays` from PR #2 head.
- First live overlay used display bounds; user correctly rejected it because unrelated apps also dimmed. Stopped runtime immediately and replaced display-wide geometry.
- Added CoreGraphics visible-window discovery keyed only by foreground process ID. No window title/content lookup and no Accessibility permission.
- Live Notes acceptance: native overlay position and size exactly matched Notes window; surrounding desktop stayed outside overlay region.
- Test created one temporary Notes rule. Removed exactly that rule afterward; local config returned to zero app rules.
- Running both ignored macOS desktop tests concurrently produced empty native enumerations. Re-ran each serially; foreground/window-bounds and physical brightness/restore tests both passed.
- Final overlay gates: 12 automated tests passed, TypeScript build passed, clippy with warnings denied passed, formatting/diff checks passed, and unsigned release `.app` built.
- Started current-app overlay preview, terminated desktop runtime while overlay active, and verified Privacy Aperture plus Vite processes exited with no stuck overlay.
- Committed as `45666f6`, pushed only `feat/macos-native-overlays`, and opened stacked PR #3 against PR #2 branch. No merge or direct `main` push performed.
