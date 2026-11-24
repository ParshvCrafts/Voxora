# Voxora macOS port: research (2026-11-24)

Scope: Tauri 2.12.x, Apple Silicon primary, $0 (no Apple Developer account). Items marked UNVERIFIED are from memory or not confirmed against a live source.
Crate versions checked on crates.io today: tauri 2.12.7, cpal 0.18.2, core-graphics 0.25.0, objc2 0.6.5, objc2-app-kit 0.3.2, objc2-core-graphics 0.3.2, arboard 3.7.0, global-hotkey 0.10.0, rdev 0.5.3 (stale since 2023), device_query 4.0.1, tauri-plugin-autostart 2.7.0, tauri-plugin-single-instance 2.5.5, tauri-nspanel 2.1.1.

## 1. Inference sidecars

### whisper-server: no prebuilt macOS binary, build our own
- VERIFIED (GitHub API): latest whisper.cpp tag with assets is `b5907` (2026-11-20). Semver tags (v1.9.6, 2026-11-23) have 0 assets; the `bNNNN` tags carry them. The only macOS asset is `whisper-b5907-xcframework.zip` (a framework, no `whisper-server`). The release workflow builds the xcframework with `-DWHISPER_BUILD_SERVER=OFF`. Linux and Windows bins exist.
- Recommendation: build in our own GitHub Actions (macos-14 arm64) and publish `whisper-server-macos-arm64.tar.gz` plus SHA-256 as a release asset of the Voxora repo; pin URL+SHA in `engines/download.rs`.
- Target name: `whisper-server` (`examples/server/CMakeLists.txt`: `set(TARGET whisper-server)`). Option `WHISPER_BUILD_SERVER` defaults to `WHISPER_STANDALONE`; set it explicitly.
- Commands (static, Metal shader embedded, single self-contained binary):
  ```
  git clone --depth 1 --branch b5907 https://github.com/ggml-org/whisper.cpp
  cd whisper.cpp
  cmake -B build -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF \
    -DGGML_METAL=ON -DGGML_METAL_EMBED_LIBRARY=ON -DGGML_NATIVE=OFF \
    -DCMAKE_OSX_DEPLOYMENT_TARGET=13.3 -DCMAKE_OSX_ARCHITECTURES=arm64 \
    -DWHISPER_BUILD_SERVER=ON -DWHISPER_BUILD_TESTS=OFF
  cmake --build build --config Release -j --target whisper-server
  # output build/bin/whisper-server ; check `otool -L` shows only system libs/frameworks
  codesign --force -s - build/bin/whisper-server
  ```
  `GGML_METAL_EMBED_LIBRARY` defaults to `${GGML_METAL}` (ggml/CMakeLists.txt); passing it is for clarity. `GGML_BLAS` defaults ON on Apple (Accelerate). `WHISPER_METAL*` options are deprecated aliases: use `GGML_METAL*`.
- Intel (nice-to-have): separate job on `macos-15-intel` with `-DGGML_METAL=OFF -DCMAKE_OSX_ARCHITECTURES=x86_64` (llama.cpp does the same). Ship per-arch assets, choose by `std::env::consts::ARCH`.
- Fallback: Homebrew `whisper-cpp` formula. Binary names UNVERIFIED (I recall `whisper-cli` and `whisper-server` in recent versions; older was `main`). Requires brew; not a default.
- Sources: https://github.com/ggml-org/whisper.cpp/releases , https://raw.githubusercontent.com/ggml-org/whisper.cpp/master/examples/server/CMakeLists.txt , https://raw.githubusercontent.com/ggml-org/whisper.cpp/master/CMakeLists.txt , https://raw.githubusercontent.com/ggml-org/whisper.cpp/master/.github/workflows/release.yml

### llama-server: prebuilt exists
- VERIFIED: assets `llama-<tag>-bin-macos-arm64.tar.gz` and `llama-<tag>-bin-macos-x64.tar.gz` (tar.gz, not zip). Latest at check time `b12675` (2026-11-24; several releases per day). arm64 asset 16.8 MB.
- Contents (downloaded b12675 arm64 and listed): `llama-server`, `libllama-server-impl.dylib`, `libllama.dylib`, `libllama-common*.dylib`, `libggml*.dylib` (ggml, ggml-base, ggml-cpu, ggml-metal, ggml-blas, ggml-rpc), `libmtmd.dylib`, other tools, LICENSE. Dylibs are versioned with symlinks (`libggml.1.dylib -> libggml.1.0.0.dylib`): extract the whole archive with symlink support (Rust `tar` crate does); do not copy files one by one.
- Built with `-DCMAKE_INSTALL_RPATH='@loader_path'`: keep the extracted dir intact and run `llama-server` from it. Archive root dir is `llama-<tag>/`.
- arm64 build flags: `-DGGML_METAL_EMBED_LIBRARY=ON -DCMAKE_OSX_DEPLOYMENT_TARGET=13.3` (built on `macos-26`; Metal on, shaders embedded). x64: `-DGGML_METAL=OFF`. Minimum macOS 13.3.
- GPU offload default on Apple Silicon: `-ngl` behaviour in current build UNVERIFIED; pass `-ngl 99` explicitly.
- Sources: https://github.com/ggml-org/llama.cpp/releases , https://raw.githubusercontent.com/ggml-org/llama.cpp/master/.github/workflows/release.yml (job `macos-cpu`)

### GitHub-hosted runners and Metal
- llama.cpp release.yml says Metal is off on the x64 runner "due to intermittent failures with Github runners not having a GPU". For arm64, a runner-images thread says arm64 mac runners lacked virtualization and PyTorch MPS failed (https://github.com/actions/runner-images/issues/9254); llama.cpp keeps Metal tests on self-hosted runners (`ci-self-hosted-metal.yml`).
- Conclusion: compiling with Metal on is fine on hosted runners; treat GPU execution in CI as unreliable and smoke-test on CPU (`whisper-server -ng`, `llama-server -ngl 0`). Whether current macos-14/15 arm64 expose Metal is UNVERIFIED; check once with `system_profiler SPDisplaysDataType`.
- If the build fails on `xcrun metal` (newer Xcode splits out the Metal toolchain: `xcodebuild -downloadComponent MetalToolchain`), add that step. UNVERIFIED.
- Arm64 binaries must carry at least an ad-hoc signature or the kernel kills them. The Apple linker adds one for CMake-built outputs; run `codesign --force -s -` after any strip/modify. Check `codesign -dv` on the llama b12675 binaries (UNVERIFIED).

## 2. Global hotkey (hold/tap on Right Option, Esc while recording)

- Recommendation: own thin CGEventTap module on a dedicated thread using `core-graphics 0.25.0` + `core-foundation 0.10.1`. `global-hotkey 0.10` uses Carbon hotkeys needing key+modifier combos (modifier-only: believed unsupported, UNVERIFIED). `rdev 0.5.3` is unmaintained since 2023. `device_query` polls.
- Reference app doing the same (hold to talk or tap to toggle, tauri-nspanel): Handy, https://github.com/cjpais/Handy (README: needs microphone + accessibility; fn/Globe works only on Apple keyboards).
- API (docs.rs core-graphics 0.25.0): `CGEventTap::new(tap: CGEventTapLocation, place: CGEventTapPlacement, options: CGEventTapOptions, events_of_interest: Vec<CGEventType>, callback: impl Fn(CGEventTapProxy, CGEventType, &CGEvent) -> CallbackResult + Send + 'static) -> Result<CGEventTap, ()>`; also `with_enabled(..., with_fn)` (tap lives only during `with_fn`, run the loop inside), `.enable()`, `.mach_port()`. Callback is a closure. Enum variant names (`ListenOnly`, `FlagsChanged`) follow `kCGEventTapOptionListenOnly` and `kCGEventFlagsChanged`; docs.rs page did not list them, check in source (UNVERIFIED).
- Handle `TapDisabledByTimeout` / `TapDisabledByUserInput` event types by re-enabling the tap; keep callback fast.
- Right Option: `FlagsChanged` event, keycode via `event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE)` == 61 (left option 58, right cmd 54, right shift 60, right ctrl 62, fn 63). Press vs release: track our own state per keycode and check the Alternate flag (mask 0x00080000); with Left Option also held the flag stays set, so state must be per keycode.
- Hold vs tap: timestamp the press; release under ~250 ms = tap (toggle), longer = hold-release (stop). Cancel if a KeyDown arrives while Option is held (chord such as Option+E).
- Esc: KeyDown keycode 53, act only while recording. Listen-only is enough (Esc also reaches the foreground app; acceptable).
- Run loop: `let src = tap.mach_port().create_runloop_source(0)`; `CFRunLoop::get_current().add_source(&src, kCFRunLoopCommonModes)`; `tap.enable()`; `CFRunLoop::run_current()` on a dedicated `std::thread` kept alive for the process lifetime.
- TCC (from Apple forum posts, not official docs): listen-only tap needs Input Monitoring; default (active) tap needs Accessibility. A granted Accessibility also satisfies listening. We need Accessibility anyway for posting Cmd+V (section 3), so one permission (Accessibility) covers everything; still use listen-only, since a forum report says an active tap plus revoked Accessibility froze mouse clicks until reboot.
- Check/prompt APIs:
  - Accessibility: `AXIsProcessTrusted()`; prompt with `AXIsProcessTrustedWithOptions` and `{kAXTrustedCheckOptionPrompt: true}` (ApplicationServices). Crate `macos-accessibility-client 0.0.2` wraps it, or declare `extern "C"` yourself.
  - Input Monitoring: `IOHIDCheckAccess(kIOHIDRequestTypeListenEvent)` and `IOHIDRequestAccess(kIOHIDRequestTypeListenEvent)` (IOKit). Apple DTS says IOHIDRequestAccess is the call that reliably adds the app to the Input Monitoring list; `CGPreflightListenEventAccess()` / `CGRequestListenEventAccess()` exist (10.15+) but may show a prompt without listing the app.
  - A failed tap creation does not say which permission is missing: preflight first.
  - `tauri-plugin-macos-permissions 2.3.0` (last update 2025-05) wraps these; own ~30 lines of FFI is less dependency risk.
- Permission granted while running may need the tap to be (re)created or an app restart (commonly reported, UNVERIFIED): poll the trust state and create the tap when it flips.
- Sources: https://docs.rs/core-graphics/0.25.0/core_graphics/event/struct.CGEventTap.html , https://developer.apple.com/forums/thread/828052 , https://developer.apple.com/forums/thread/735204 , https://developer.apple.com/forums/thread/780626

## 3. Paste injection

- Recommendation: `objc2-app-kit 0.3.2` (`NSPasteboard`, `NSPasteboardItem`) + `objc2-foundation`. `arboard 3.7.0` is not enough (no transient marker, no multi-item snapshot).
- APIs: `NSPasteboard::generalPasteboard()`, `.changeCount()`, `.pasteboardItems()`, per item `.types()` and `.dataForType(&NSString)`; restore with `clearContents()` then `writeObjects` of fresh `NSPasteboardItem`s using `setData_forType`. Plain text: `setString_forType(.., NSPasteboardTypeString)`. Exact method names follow the Obj-C selectors (docs.rs: https://docs.rs/objc2-app-kit/0.3.2/objc2_app_kit/struct.NSPasteboard.html); each needs its cargo feature enabled.
- Snapshot all items and all types' data before writing. Lazy/promised data may be slow: time-box and skip huge items (UNVERIFIED).
- Marker types (nspasteboard.org convention honored by clipboard managers): add `org.nspasteboard.TransientType` (and optionally `org.nspasteboard.ConcealedType`, `org.nspasteboard.AutoGeneratedType`) to our item with empty data. Source: https://nspasteboard.org
- Sequence number: store `changeCount` after our write; restore the old contents only if `changeCount` still equals it (user did not copy meanwhile). Same logic as the Windows sequence number.
- Cmd+V: `CGEventSource::new(CGEventSourceStateID::HIDSystemState)`; `CGEvent::new_keyboard_event(source, 9, true)` (kVK_ANSI_V = 9); `.set_flags(CGEventFlags::CGEventFlagCommand)`; `.post(CGEventTapLocation::HID)`; then the key-up with the same flags. Set flags on both down and up. Non-QWERTY layouts: Cmd+V uses the key equivalent so keycode 9 generally still pastes, UNVERIFIED across layouts.
- Needs Accessibility (`AXIsProcessTrusted`); without it posted events are silently dropped.
- Modifier gotcha: Right Option may still be down when we post, giving Option+Cmd+V (Finder: "Move Item Here"; some apps: Paste and Match Style variants). Wait for Option release and set flags to Command only. Delay ~20 ms between pasteboard write and post, and ~150-300 ms before restoring the clipboard (target reads asynchronously).
- Secure input: `IsSecureEventInputEnabled()` (Carbon HIToolbox, `extern "C" fn IsSecureEventInputEnabled() -> bool`). When true (password fields, some terminals) synthetic key events may be blocked: tell the user and leave the text on the clipboard instead of restoring.
- Alternative without clipboard: AX `AXUIElementSetAttributeValue(kAXSelectedTextAttribute)`; works in many native fields, fails in Electron/web. UNVERIFIED reliability.

## 4. Foreground app detection
- Recommendation: bundle id only. `NSWorkspace::sharedWorkspace().frontmostApplication()` then `NSRunningApplication` `.bundleIdentifier()`, `.localizedName()`, `.processIdentifier()`. No permission needed.
- Other apps' window titles via `CGWindowListCopyWindowInfo` `kCGWindowName` require Screen Recording on 10.15+ (UNVERIFIED on 15). If a title rule is ever needed, use AX (`AXUIElementCreateApplication(pid)` -> `kAXFocusedWindowAttribute` -> `kAXTitleAttribute`), which needs only Accessibility, already required. Do not request Screen Recording.
- Query the frontmost app before showing the overlay; a non-activating panel keeps it unchanged.

## 5. Overlay pill without stealing focus
- Recommendation: `tauri-nspanel = "2.1"` (2.1.1, 2026-10-08; requires tauri >= 2.8.5 with `macos-private-api`; objc2 0.6; maintained branch `v2.1`; used by Handy, Cap, screenpipe, Vibe). Verified from its README:
  ```rust
  tauri_panel! { panel!(PillPanel { config: { can_become_key_window: false, is_floating_panel: true } }) }
  let panel = PanelBuilder::<_, PillPanel>::new(app.handle(), "pill")
      .url(WebviewUrl::App("pill.html".into())).level(PanelLevel::Floating).build()?;
  panel.show();            // show_and_make_key() would take focus
  ```
  Plugin init `.plugin(tauri_nspanel::init())`; fetch later with `app.get_webview_panel("pill")` (`ManagerExt`). Whether `can_become_key_window: false` alone yields a non-activating panel, and the config keys for style mask, collection behavior (`canJoinAllSpaces | fullScreenAuxiliary`, so it shows over fullscreen apps) and click-through, are UNVERIFIED: read https://docs.aremu.dev/tauri-nspanel/ . Mouse pass-through: `ignoresMouseEvents = true`.
- Transparency: `"app": {"macOSPrivateApi": true}` in tauri.conf.json plus Cargo feature `tauri/macos-private-api`; window `transparent: true`, `decorations: false`, `shadow: false`. This only blocks Mac App Store, irrelevant here.
- Plain Tauri window: `show()` with `focus: false` is widely reported to still activate the app on macOS (UNVERIFIED) and steal focus from the target app, which breaks pasting. Fallback without the plugin: objc2 `NSWindow::orderFrontRegardless()` + `setLevel` + `setCollectionBehavior` + `setIgnoresMouseEvents(true)`, never calling `window.show()`. A truly non-activating window needs an NSPanel with `NSWindowStyleMaskNonactivatingPanel`, which is why the plugin is the stable route.
- Tray-only: `app.set_activation_policy(tauri::ActivationPolicy::Accessory)` in `setup` under `#[cfg(target_os = "macos")]` (no Dock icon). Alternative Info.plist `LSUIElement = true`. For the settings window, switch to `Regular` while open and back to `Accessory` on close (common pattern, UNVERIFIED).
- Sources: https://github.com/ahkohd/tauri-nspanel (branch v2.1 README), https://crates.io/crates/tauri-nspanel

## 6. Microphone
- cpal 0.18.2 uses CoreAudio for input. First capture triggers the TCC prompt. The prompt attributes to the .app only when run bundled; under `tauri dev` it attributes to the launching terminal/IDE. Bluetooth headsets switch to a low sample rate: query `default_input_config()` at every start and resample.
- Info.plist: Tauri v2 merges `src-tauri/Info.plist` into the bundle; add `NSMicrophoneUsageDescription` (UNVERIFIED, from memory; confirm at https://v2.tauri.app/distribute/macos-application-bundle/).
- Entitlements file `src-tauri/Entitlements.plist` referenced by `bundle.macOS.entitlements`: `com.apple.security.device.audio-input` = true. Only enforced with hardened runtime; harmless under ad-hoc. If hardened runtime is ever on, sidecar dylibs signed with a different identity need `com.apple.security.cs.disable-library-validation`. Do not enable the App Sandbox (breaks sidecar spawn and event posting).
- Mic status: `AVCaptureDevice authorizationStatusForMediaType:` (AVMediaTypeAudio) / `requestAccessForMediaType:`, or `tauri-plugin-macos-permissions`.

## 7. Distribution without a paid account
- `bundle.targets: ["app", "dmg"]` in a `tauri.macos.conf.json` merge file. VERIFIED (Tauri docs): `"bundle": {"macOS": {"signingIdentity": "-"}}` = ad-hoc signing; docs say it is needed on Apple Silicon ("code-signing is required for all apps from the Internet") and that it does not remove the user's need to allow the app in Privacy & Security. Always ad-hoc sign; unsigned arm64 apps will not run. Source: https://v2.tauri.app/distribute/sign/macos/
- Other keys from memory (UNVERIFIED): `bundle.macOS.entitlements`, `minimumSystemVersion` (use "13.0"), `hardenedRuntime`.
- Sequoia (macOS 15) user steps: the Control-click > Open bypass was removed. Open the app, dismiss the dialog, System Settings > Privacy & Security > scroll to Security > "Voxora was blocked" > Open Anyway > password. Terminal alternative: `xattr -dr com.apple.quarantine /Applications/Voxora.app`. Reports conflict on whether Open Anyway appears for fully unsigned apps on 15.1+ (OSNews says no; MacRumors says only signed-but-not-notarized); ad-hoc signed should be treated as the safe case. `xattr` clearing was not confirmed by these sources but is the widely used workaround (UNVERIFIED on latest 15.x/26). Document both in the README. Sources: https://www.idownloadblog.com/apple-macos-sequoia-gatekeeper-change-install-unsigned-apps-mac/ , https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unknown-developer-mh40616/15.0/mac/15.0 , https://eclecticlight.co/2024/10/01/living-without-notarization/ , https://forums.macrumors.com/threads/2441314
- Homebrew cask with `--no-quarantine` avoids quarantine (Handy is distributed so, unofficially). Own tap is a $0 option; current homebrew/cask policy for non-notarized casks UNVERIFIED.
- TCC and ad-hoc signing (from knowledge, UNVERIFIED by a live source): grants are keyed to the code requirement; for ad-hoc that is the cdhash, which changes every build. So each update looks like a new app: Settings shows the toggle ON but `AXIsProcessTrusted()` is false. Implications: (a) onboarding must detect "listed but not trusted" and tell the user to remove the entry (minus button) and re-add, or run `tccutil reset Accessibility <bundle-id>` (also `ListenEvent`, `Microphone`); (b) keep the bundle identifier stable; (c) a free self-signed code-signing certificate (Keychain Access > Certificate Assistant) gives a stable designated requirement, so grants survive rebuilds on that machine; for distribution you could import the same cert in CI via secrets. Users still get the Gatekeeper warning, but TCC might persist across updates. Test this; it is the best $0 improvement.
- Sidecars downloaded by our app: the quarantine xattr is applied by apps that opt in (browsers, or apps with `LSFileQuarantineEnabled`); plain file writes by reqwest/std::fs do not get it (UNVERIFIED but standard behavior; Tauri does not set the key). Defensive steps after extract: `xattr -dr com.apple.quarantine <dir>` ignoring failure, and set mode 0o755 via `std::os::unix::fs::PermissionsExt` (tar keeps modes, zip may not). Prefer shipping binaries already ad-hoc signed by CI over relying on `codesign` on the user machine (needs Command Line Tools).
- Data dir: `~/Library/Application Support/<bundle-id>` via Tauri `app.path().app_data_dir()` (or `dirs::data_local_dir()`); logs `~/Library/Logs/<bundle-id>`.

## 8. Autostart and single instance
- `tauri-plugin-autostart 2.7.0`: macOS supports `MacosLauncher::LaunchAgent` (writes `~/Library/LaunchAgents/...plist`) or `MacosLauncher::AppleScript` (may trigger an Automation prompt). Use LaunchAgent: `tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--minimized"]))`; toggle with `app.autolaunch().enable()/disable()/is_enabled()`. The plist stores the executable path, so re-enable on launch if the app was moved (UNVERIFIED behavior). https://v2.tauri.app/plugin/autostart/
- `tauri-plugin-single-instance 2.5.5`: supported on macOS; register it first in the builder; callback `|app, args, cwd|` runs in the first instance. Finder double-click already dedupes by bundle id; the plugin matters for CLI relaunch. https://v2.tauri.app/plugin/single-instance/

## 9. CI/CD
- Recommended: plain commands (keeps our gate identical). `runs-on: macos-14`; `rustup target add aarch64-apple-darwin`; `pnpm install`; `pnpm tauri build --target aarch64-apple-darwin --bundles app,dmg`. Output under `src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/`. Also run `cargo test -p voxora-core` on macOS.
- Alternative: `tauri-apps/tauri-action` with a matrix (arm64 `macos-14`, Intel `macos-15-intel` with `--target x86_64-apple-darwin`); handles release upload. Current version tag UNVERIFIED. Tauri docs' CI example builds separate aarch64 and x86_64 targets (https://v2.tauri.app/distribute/sign/macos/).
- Universal: `pnpm tauri build --target universal-apple-darwin` (both rust targets). Sidecars are per-arch and downloaded at runtime, so universal gains little; recommend two dmgs (arm64 primary, Intel optional).
- macOS minutes are free for public repos (UNVERIFIED, check GitHub billing docs).
- Build the whisper-server sidecar in a separate workflow (section 1) that uploads the asset and prints the SHA-256 for `download.rs`.

## Port touch points
- `voxora-core` stays platform-neutral. Add `cfg(target_os = "macos")` modules in the app crate: hotkey (CGEventTap), paste (NSPasteboard + CGEvent), foreground (NSWorkspace), overlay (nspanel), permissions (AX / IOHID / AVCapture checks + onboarding UI).
- `engines/download.rs`: per-OS/arch asset tables, tar.gz extraction with symlinks for llama, own whisper-server asset, keep SHA-256 pins.
- `scripts/e2e.ps1` has no mac equivalent. Idea (UNVERIFIED): BlackHole virtual audio device plus an osascript focus guard, preserving the "own window must have focus" rule.
