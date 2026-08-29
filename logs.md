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
- Created stacked branch `docs-product-readme` from clean macOS overlay PR #3 head; `main` remained untouched.
- Queried existing Graphify map for implemented rule, storage, matching, and message-validation boundaries; verified user-facing claims against current source and validation log.
- Confirmed GitHub has no published release and inspected real dark-mode settings screenshot before README use.
- Rebuilt README as product page: outcome-led hero, screenshot, app-only versus panel-wide explanation, verified capability/status tables, privacy promise, honest source-build CTA, compact architecture, and prioritized feature roadmap.
- GitHub Markdown API accepted README in repository context; referenced icon and screenshot files exist.
- Documentation branch validation passed: TypeScript type-check, 12 automated Rust tests, zero failures, 2 expected ignored GUI/hardware tests, and `git diff --check`.
- Committed product README as `024f68b`, pushed only `docs-product-readme`, and opened stacked PR #5 against `feat/macos-native-overlays`. GitHub reports PR open and mergeable with zero configured checks; no merge or direct `main` push performed.

## 2026-08-29

- Refreshed GitHub state: PRs #1 through #6 had been merged and `main` was at `e9c25f1`; no open PR remained. Created `fix/review-correctness-blockers` from clean, current `main` before edits.
- Confirmed all eight review findings in live source. No P0 condition found; four P1 and four P2 correctness defects required fixes before release.
- Added red regressions for duplicate application matchers and hostname specificity. Both failed against previous behavior, then passed after Rust validation and specificity fixes.
- Changed emergency removal to pause runtime config before cleanup, attempt overlay and brightness recovery independently, clear status, and persist paused state in native command.
- Added overlay partial-apply rollback. Every touched overlay is tracked immediately; failure hides all tracked windows and leaves bookkeeping available for later recovery retries.
- Restricted local-storage persistence to browser preview. Native failures now show errors and restore last confirmed UI config; backend rolls storage/runtime config back together if activation fails.
- Disabled and labeled launch-at-login/global-shortcut controls as pending; onboarding and protection screen now direct users to working in-app recovery.
- Added per-display supported/error reporting and partial-support UI. Added shared transactional brightness application with rollback across macOS, Windows, and Linux.
- Added UI and Rust duplicate application-rule guards. Website matching now selects longest matching hostname before application fallback.
- Automated validation passed: TypeScript build, 17 Rust tests, clippy with warnings denied, formatting, and diff checks. Two GUI/hardware tests remain ignored in normal suite by design.
- Initial ignored-test run inside sandbox lacked WindowServer/display access. Re-ran serially with GUI/hardware permission: foreground/window discovery passed; physical brightness change-and-restore passed.
- Windows cross-check remains environment-blocked before project compilation by missing `x86_64-w64-mingw32-windres`. Linux cross-check remains environment-blocked by absent cross-target GTK/GIO/Pango pkg-config sysroot; no platform runtime claim added.
- Built unsigned release `Privacy Aperture.app`, launched release runtime successfully, stopped it with normal interrupt, and verified no `privacy-aperture` process remained.
