# Game Optimizer

[Español](./README-ES.md) | English

Game Optimizer is a Windows desktop app for organizing games and managing how their processes run in Windows. It changes selected Windows settings; **it does not change a game's graphics settings**, such as resolution, visual quality or effects. Results depend on the game, hardware and Windows; improved performance or FPS is not guaranteed.

> **Work in progress:** The app does not yet replace all features of the legacy scripts.

## Current features

- **Library:** Add Windows executables with the file picker or by dragging them in, and assign each a profile. Launch a game or open its folder from its card. Launching does not apply the profile automatically.
- **Organization:** Choose a grid or list, mark favorites, sort by name or arrange items manually. Related names can be grouped, although automatic grouping may not always match. Manual ordering also works with the keyboard; favorites stay first.
- **Profiles:** Under Settings > Profiles, adjust the built-in profiles or create your own. A profile can request a CPU priority and GPU preference for an executable. CPU priority applies to future processes with the same executable name, including in other folders, and requires administrator permission. The GPU preference asks Windows which GPU to use; Windows or the app may choose differently.
- **Apply and restore:** Use Optimize on a library card to apply its profile, and Restore to return to the saved previous settings. Confirm changes before applying them. Status describes changes managed by this app, not all Windows settings.
- **Defaults:** This action removes the selected CPU and GPU settings and discards the corresponding saved backup. Once completed, Restore cannot undo it. It does not remove Defender exclusions; restore an optimization that includes them first.
- **Defender:** Exclusions are optional and off by default. Enabling them excludes the executable's entire folder and files opened by that process from scanning, reducing protection. Administrator approval is required; the status remains unverified until Restore checks it.
- **Global optimizations:** Power plan, Game Mode and Game DVR are separate from profiles and do nothing by default. The power plan affects the entire computer and the chosen plan must exist. Game DVR controls Windows recording, not game graphics.
- **Settings and updates:** Choose English or Spanish and whether the app minimizes or closes after a successful launch. Under Settings > About, check for the latest stable release on GitHub or open GitHub to report a bug or suggest an improvement. Optional startup checks are off by default and only notify you. Downloading opens the portable Windows x64 executable in your browser; replace the old executable manually. Installation and signature verification are not automatic.

If Windows settings were changed elsewhere, Restore will not overwrite the conflicting change. If an operation is interrupted or needs recovery, keep the app data and the library entry and use Restore. Only one instance can access the app data at a time.

The [legacy scripts](./legacy-scripts/) are separate from the desktop app and have different behavior and risks.

## Implementation and recovery details

The app is built with Tauri 2, React and TypeScript. Library, profiles and transaction journals are stored under the application's `%LOCALAPPDATA%` directory; logs use Tauri's Windows log directory. Replacing the portable executable on the same Windows account does not remove those files.

Defaults permanently removes enabled CPU/GPU settings, including those in an active optimization, and discards the corresponding backup. Interrupted operations retain recovery information. Older defaults transactions remain restorable or can be explicitly cleared with Defaults.

Defender restoration preserves preexisting exclusions and verifies changes with elevation. Active profiles with overlapping folders or physically identical executables are rejected, including across users. Older journals without resource identities require manual inspection. Do not move or redirect the executable's folders or links while applying or restoring an exclusion. Machine-wide reservations are stored in `%PROGRAMDATA%\GameOptimizer\resources` and new Defender reservations in `%PROGRAMDATA%\GameOptimizer-Defender-2739818f`; do not delete them while changes are active. Restore older Defender transactions before creating new exclusions. These reservations coordinate this app, but Windows or other programs may change Defender independently.

Global actions update `AutoGameModeEnabled` and `GameDVR_Enabled` for the current Windows user; their effectiveness depends on Windows and policy settings. The previous registry values and types are journaled before each write. After interruption, use Restore in global settings; external changes are not overwritten.

## License

Game Optimizer is licensed under the GNU General Public License, version 3 or (at your option) any later version (GPL-3.0-or-later). See [LICENSE](./LICENSE). Third-party dependencies retain their own licenses.

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

Once the workflow is on the default branch, update the version in `package.json` and `package-lock.json`, then publish a GitHub release for a matching tag such as `v1.2.3` or `v1.2.3-rc.1` (build metadata such as `+build` is not supported). The workflow checks out that tag, verifies it matches `package.json`, builds a Windows x64 portable executable and attaches `Game-Optimizer-<tag>-windows-x64.exe` and `LICENSE` to the release. Tauri reads the displayed app version and Windows executable version metadata from `package.json`; each numeric version component must be at most 65535. Draft releases build when published. If either asset already exists, remove it before rerunning the workflow.

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

## Real Defender test

This opt-in test requires several UAC approvals. It temporarily adds exclusions for a new folder and a disposable executable inside it, then attempts to restore them and verify the result. The folder exclusion affects all its files while active, so do not put other files there. The journal is stored separately.

```powershell
$id = [guid]::NewGuid().ToString('N')
$dir = Join-Path $env:TEMP "GameOptimizerDefenderTest-$id"
$dummyDir = Join-Path $dir 'dummy'
New-Item -ItemType Directory -Path $dummyDir | Out-Null
$env:GAME_OPTIMIZER_DUMMY_EXE = Join-Path $dummyDir "dummy-$id.exe"
$env:GAME_OPTIMIZER_SERVICE_ROOT = Join-Path $dir 'journal'
rustc --edition 2021 tests\fixtures\dummy.rs -o $env:GAME_OPTIMIZER_DUMMY_EXE
if ($LASTEXITCODE -ne 0) { throw 'Failed to build the dummy executable' }
cargo build --manifest-path src-tauri\Cargo.toml
if ($LASTEXITCODE -ne 0) { throw 'Failed to build the administrator helper' }
$env:GAME_OPTIMIZER_HELPER_EXE = (Resolve-Path src-tauri\target\debug\game-optimizer-app.exe).Path
cargo test --manifest-path src-tauri\Cargo.toml real_dummy_coordinated_defender_profile_applies_and_restores -- --ignored --nocapture
```

If the test is interrupted or restoration fails, keep the directory and journal. With the same `GAME_OPTIMIZER_DUMMY_EXE`, `GAME_OPTIMIZER_SERVICE_ROOT` and `GAME_OPTIMIZER_HELPER_EXE`, try to recover the journal:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml recover_full_profile -- --ignored --nocapture
```

Do not delete the directory or journal until the exclusions have been verified as restored.

## Pending work

Defender resource snapshots record the Windows volume and file ID. CPU restoration restores the priority value but can leave empty IFEO registry keys created during application. Comprehensive UI and interruption testing is still needed.
