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
- Committed all eight review fixes as `6cda97a`, pushed only `fix/review-correctness-blockers`, and opened PR #7 against `main`. GitHub reports PR open and mergeable with zero configured checks; no merge or direct `main` push performed.
- Confirmed PR #7 merged at `6fdeed9`, fast-forwarded local `main`, and created direct-main branch `feat/macos-command-center` before edits.
- Queried existing Graphify map for rule/runtime/storage integration points, then verified every decision against current source and installed Tauri APIs.
- Added menu-bar command center with live Watching/Protected/Peek/Paused/Error state, Protect Current App, Peek, Pause/Resume, Open Settings, and cleanup-first Quit.
- Added official Tauri autostart and global-shortcut plugins behind Rust commands. Launch-at-login writes OS and config transactionally; shortcut conflicts retain previous registration.
- Added memory-only hold-to-Peek, persistent emergency pause, close-to-hide, hidden autostart launch, Quick Protect at 35% visibility, and existing-rule dedupe/edit routing.
- Added unit coverage for default shortcut parsing, Peek collision, shortcut rollback, autostart reconciliation, and Quick Protect dedupe.
- Automated checks passed before live acceptance: TypeScript/Vite production build, 22 Rust tests with 2 expected ignored hardware/GUI tests, clippy with warnings denied, and unsigned macOS `.app` bundle.
- Live macOS acceptance passed: command-center item list visible; Command-W hid settings while process/menu stayed alive; Open Settings restored focus; tray Peek changed state `Watching -> Peek -> Watching`; physical Command-Shift-0 changed `Watching -> Paused` and persisted pause; tray Resume restored enabled config; tray Quit exited tested process.
- Forced `open -n` during lifecycle testing created second development instance and caused later UI clicks to hit stale instance. Stopped only two test PIDs, restored exact pre-test config summary (enabled, launch-at-login off, hardware opt-in off, 35%, zero rules), verified both processes stopped, and deleted two temporary screenshots containing unrelated desktop content.
- Rebuilt latest unsigned bundle after final context-boundary fix. Clean single-instance smoke exposed complete Watching-state tray menu, tray Quit ended process, and post-smoke config still matched recorded baseline.
- Final review found possible worker/main-thread quit deadlock around synchronous tray item mutation. Moved tray updates onto Tauri main-thread queue; repeated 22 tests, clippy, frontend build, formatting, and diff checks passed.
- Committed as `f616bfe`, pushed only `feat/macos-command-center`, and opened direct-main PR #8. No merge or direct `main` push performed.
- Added official Tauri single-instance plugin first in builder order and reused `show_settings` for warm-launch restoration. Initial warm-path validation did not prove simultaneous cold-start ownership.
- Single-instance validation passed: 22 Rust tests passed with 2 expected ignored GUI/hardware tests, clippy with warnings denied passed, and unsigned release `.app` rebuilt successfully.
- Live two-launch acceptance passed. Command-W changed settings window count from one to zero; second binary invocation exited successfully without changing first process IDs; existing instance returned frontmost with one window. Clean quit left no Privacy Aperture process, and config baseline remained v2, enabled, launch-at-login/hardware brightness off, 35%, and zero rules.
- Review found official plugin 2.4.3 probes its macOS socket, then spawns an asynchronous bind; two cold processes could both return from plugin setup before either listener existed. Rejected the warm-only result as insufficient.
- Added a macOS standard-library startup ownership lock with no new crate. Primary removes stale official socket only after acquiring ownership and holds the file lock through `app.run`; blocked processes retry until the official socket accepts connections, take ownership if primary dies first, or exit after a bounded timeout. Handoff unexpectedly reaching user setup exits before runtime/tray/shortcut/config/autostart side effects.
- Added a pending settings-restore latch consumed at Tauri Ready after autostart hiding. Path test asserts exact official macOS socket naming from the Tauri identifier.
- Review-fix gates passed: 23 Rust tests with 2 expected ignored GUI/hardware tests, clippy with warnings denied, formatting, TypeScript/Vite build, and unsigned release `.app` bundle.
- Concurrent cold launch passed: one process became primary, second exited successfully, and only one app process remained. Warm relaunch restored hidden settings from zero to one window without changing primary process IDs. Clean quit removed official socket and left no app process; config baseline remained v2, enabled, launch-at-login/hardware brightness off, 35%, and zero rules.
- Final review rejected socket-path existence as readiness because stale crash residue could force a false handoff. Replaced it with successful `UnixStream::connect` probes before lock acquisition, after ownership acquisition, and while blocked. Failed pre-build gates now exit nonzero.
- Added isolated macOS socket tests using unique identifiers: a live listener hands off without removal; a dropped-listener stale socket becomes primary and is removed. All exact test socket/lock paths were cleaned.
- Final gates passed: 25 Rust tests with 2 expected ignored GUI/hardware tests, clippy with warnings denied, formatting, TypeScript/Vite build, and unsigned release `.app` bundle. Sandboxed `/tmp` socket binding was denied, so socket tests were rerun with macOS permission and passed.
- Final live cold/warm acceptance passed: concurrent cold launches produced one primary and one successful handoff; warm launch restored hidden settings from zero to one window with unchanged primary process IDs. Clean quit left no app process or official socket, and config baseline remained unchanged.
- Independent final review found no P0/P1/P2 issue. Remaining boundary: a same-user local actor can deny relaunch through predictable `/tmp` lock/socket paths, matching the official plugin trust boundary; async listener-bind failure can prevent restore notification, but the held startup lock still prevents a duplicate runtime.
- User screenshot showed application picker exposing macOS helpers, extensions, agents, and background services because `running()` returned every `NSWorkspace.runningApplications` entry.
- Added pure regression proving only eligible window-owner PIDs survive, filtering happens before bundle-ID deduplication, hidden helpers are removed, and Privacy Aperture excludes itself. Red run failed with missing `filter_windowed_applications`; focused green run passed after implementation.
- Added one shared CoreGraphics eligible-window scanner using existing on-screen, desktop-exclusion, layer-zero, positive-alpha, and minimum 40×40 semantics. Picker builds an owner-PID set once; overlay bounds consume the same scanner.
- Relabeled picker to `Visible application` / `Choose visible app…`. Preserved manual bundle-ID fields and unchanged foreground `current()` behavior.
- Validation passed: 26 Rust tests with 2 expected GUI/hardware ignores, clippy with warnings denied, formatting, diff checks, and the production frontend build through pnpm. Pnpm migration files stayed outside this focused feature diff.
- Active macOS desktop integration test passed for foreground identity, filtered running applications, and standard-window bounds. Visual picker inspection remains manual; no UI screenshot claim added.

## 2026-08-30

- Confirmed PR #11 merged and fetched `origin/main` at `2c02ba8` before creating `docs/readme-current-capabilities`.
- Updated README picker behavior to match eligible on-screen standard-window filtering and documented that most helpers, extensions, and agents are excluded when they own no eligible window.
- Documented merged menu-bar command center, Peek, launch-at-login, emergency shortcut, cleanup-first quit, and single-instance settings restore as current capabilities; removed their stale roadmap entries.
- Corrected Focus wording and restored roadmap order: Focus, settings UI V2, Chromium, then signed distribution.
- Replaced stale feature-branch clone and every npm command with direct-main clone and pnpm commands. Updated Tauri lifecycle hooks, pinned pnpm 11.22.0, adopted pnpm lock/workspace policy, and removed npm lockfile.
- Independent review found and resolved four P2 truth gaps: overbroad helper exclusion, npm-backed Tauri hooks, incorrect Focus behavior/order, and understated Vite Node.js minimum.
- `pnpm install --frozen-lockfile`, `pnpm build`, and `pnpm tauri build --no-bundle` passed. Tauri output confirmed `beforeBuildCommand` runs `pnpm build` and produced release binary.
- Sandboxed `pnpm check` passed TypeScript but blocked two existing Unix-socket tests with `Operation not permitted`; permitted rerun passed 26 Rust tests with 2 expected GUI/hardware ignores.
- `git diff --check` passed; README stale npm/feature-branch/picker wording scan found no current-instruction regressions.
