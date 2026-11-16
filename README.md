# Game Optimizer

A Windows desktop application built with Tauri 2, React and TypeScript.

> **Work in progress:** CPU priority, GPU preference and global settings can be applied and restored; defaults removes selected settings permanently. New Defender exclusions remain disabled while cross-path and cross-user conflict handling is validated. This is not yet a complete replacement for the legacy scripts.

## Current features

- English and Spanish UI.
- Executable library with grid/list views, Windows file icons and names from executable metadata when available. The library notice can be dismissed permanently.
- Add executables using the file picker or drag and drop; assign a profile when adding.
- Open an executable's folder or launch it with its button or icon. In Settings, choose whether the app minimizes (default) or closes after a successful launch; starting a game does not automatically apply a profile.
- Manage language and launch behavior under Settings > General, and built-in and custom profiles under Settings > Profiles.
- Apply and restore CPU priority and GPU preferences with confirmation and a persistent transaction journal. CPU priority requires administrator approval and affects future processes with the same executable name, including in other folders.
- Defaults permanently removes the enabled CPU and GPU settings and any CPU/GPU actions in an active optimization. It discards that optimization's backup; once completed, Restore cannot undo it. Interrupted operations retain recovery information. Older defaults transactions remain restorable or can be explicitly cleared with Defaults.
- Global power, Game Mode and Game DVR options are separate from executable profiles and default to no change. Power changes affect the whole computer and require the selected plan to exist.
- Defender actions are disabled for new changes. When an existing profile includes Defender, the confirmation explains it will be skipped and only CPU/GPU actions will run; the profile is not changed. Defender journals created without a resource identity cannot be restored automatically and require manual inspection; newer journals require elevated verification and refuse to modify a replaced or inaccessible resource.
- Library and profile data are stored under the application's `%LOCALAPPDATA%` directory. Application logs use Tauri's Windows log directory.

The profile status reflects changes managed by this application, not an assessment of all existing Windows settings. If a setting changes externally, restoration refuses to overwrite it; resolve the conflict before retrying. Do not delete the application data or an active executable entry while its change requires recovery. Only one instance can access the application data at a time. Machine-wide resource reservations are stored under `%PROGRAMDATA%\GameOptimizer\resources`; do not delete them while changes are active.

Global actions update `AutoGameModeEnabled` and `GameDVR_Enabled` for the current Windows user; their effectiveness depends on Windows and policy settings. The previous registry values and types are journaled before each write. If interrupted, use Restore in the global settings page; external changes are not overwritten.

The legacy scripts, their documentation and tests are in [legacy-scripts](./legacy-scripts/).

## Development

Requirements: Windows, Node.js, Rust with the MSVC toolchain, Visual C++ build tools and WebView2 Runtime.

If Cargo is installed through rustup but `cargo` is not found, restart VS Code or add it to the current PowerShell session:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo --version
```

If `cargo --version` still fails, install Rust with rustup and reopen the terminal.

```powershell
npm install
npm run tauri dev
```

Run frontend tests with `npm test` and native tests with `cargo test --manifest-path src-tauri\Cargo.toml`.

To build a portable executable without an installer:

```powershell
npm run tauri build -- --no-bundle
```

The executable is produced under `src-tauri\target\release`. WebView2 Runtime must be installed on the target machine. App data remains on the machine, not beside the executable.

## Publishing a release

Once the workflow is on the default branch, update the version in `package.json` and `package-lock.json`, then publish a GitHub release for a matching tag such as `v1.2.3` or `v1.2.3-rc.1` (build metadata such as `+build` is not supported). The workflow checks out that tag, verifies it matches `package.json`, builds a Windows x64 portable executable and attaches `Game-Optimizer-<tag>-windows-x64.exe` to the release. Tauri reads the displayed app version and Windows executable version metadata from `package.json`; each numeric version component must be at most 65535. Draft releases build when published. If an asset with that name already exists, remove it before rerunning the workflow.

Pushes to `main` run the same build and populate the Rust dependency cache for later releases; they do not upload release assets. The first build, or a build after a cache miss, may still take longer.

## Real GPU recovery test

This opt-in test temporarily changes the current user's GPU preference **only for the generated dummy executable path**. It injects a failure after the write, then restores and verifies the previous registry value. The journal must not already exist. Keep the test directory and journal until restoration is confirmed.

```powershell
$id = [guid]::NewGuid().ToString('N')
$dir = Join-Path $env:TEMP "GameOptimizerTest-$id"
New-Item -ItemType Directory -Path $dir | Out-Null
$env:GAME_OPTIMIZER_DUMMY_EXE = Join-Path $dir "dummy-$id.exe"
$env:GAME_OPTIMIZER_TEST_JOURNAL = Join-Path $dir 'journal.json'
rustc --edition 2021 tests\fixtures\dummy.rs -o $env:GAME_OPTIMIZER_DUMMY_EXE
cargo test --manifest-path src-tauri\Cargo.toml real_dummy_gpu_preference_recovers_after_a_partial_failure -- --ignored --nocapture
```

If interrupted or restoration fails, keep the same `GAME_OPTIMIZER_TEST_JOURNAL` and run:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml recover_dummy_gpu_journal -- --ignored --nocapture
```

## Pending work

Defender resource snapshots record the Windows volume and file ID, and identity-based reservations recognize hard links to the same file. Defender profile actions remain disabled: overlapping folders and secure cross-user coordination are not yet handled. CPU restoration restores the priority value but can leave empty IFEO registry keys created during application. Comprehensive UI and interruption testing is still needed.
